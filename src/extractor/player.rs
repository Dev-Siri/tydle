use std::collections::HashMap;

use anyhow::{Result, anyhow};
use fancy_regex::Regex;
use maplit::hashmap;
use serde_json::{Map, Value, json};

use crate::{
    STREAMING_DATA_CLIENT_NAME, STREAMING_DATA_INNERTUBE_CONTEXT,
    cache::{CacheAccess, PlayerCacheHandle},
    extractor::{
        api::ExtractorApiHandle,
        auth::ExtractorAuthHandle,
        download::ExtractorDownloadHandle,
        extract::{InfoExtractor, YtExtractor},
        json::ExtractorJsonHandle,
        ytcfg::ExtractorYtCfgHandle,
    },
    yt_interface::{
        PLAYER_JS_MAIN_VARIANT, PlayerIdentifier, VideoId, YT_URL, YtClient, YtEndpoint,
    },
};

pub trait ExtractorPlayerHandle {
    fn is_unplayable(&self, player_response: &HashMap<String, Value>) -> bool;
    fn is_age_gated(&self, player_response: &HashMap<String, Value>) -> bool;
    fn generate_player_context(&self, sts: Option<i64>) -> HashMap<String, Value>;
    async fn load_player(&self, video_id: &VideoId, player_url: String) -> Result<String>;
    /// Extract `signatureTimestamp` (sts)
    /// Required to tell API what sig/player version is in use.
    async fn extract_signature_timestamp(
        &self,
        video_id: &VideoId,
        player_url: String,
        ytcfg: &HashMap<String, Value>,
    ) -> Result<Option<i64>>;
    fn construct_player_url(&self, player_identifier: PlayerIdentifier) -> Result<String>;
    fn get_player_url(&self, ytcfgs: &[&HashMap<String, Value>]) -> Result<String>;
    fn invalid_player_response(
        &self,
        pr: &HashMap<String, Value>,
        video_id: &VideoId,
    ) -> Option<String>;
    async fn extract_player_response(
        &self,
        client: &YtClient,
        video_id: &VideoId,
        webpage_ytcfg: &HashMap<String, Value>,
        player_ytcfg: &HashMap<String, Value>,
        player_url: &Option<String>,
        initial_pr: &HashMap<String, Value>,
        visitor_data: &Option<String>,
        data_sync_id: &Option<String>,
    ) -> Result<HashMap<String, Value>>;
    async fn extract_player_responses(
        &self,
        clients: &Vec<YtClient>,
        video_id: &VideoId,
        webpage: &String,
        webpage_client: &YtClient,
        webpage_ytcfg: &HashMap<String, Value>,
    ) -> Result<(Vec<HashMap<String, Value>>, String)>;
}

impl ExtractorPlayerHandle for YtExtractor {
    fn construct_player_url(&self, player_identifier: PlayerIdentifier) -> Result<String> {
        match player_identifier {
            PlayerIdentifier::PlayerUrl(player_url) => {
                return Ok(format!("{}{}", YT_URL, player_url));
            }
            PlayerIdentifier::PlayerId(player_id) => Ok(format!(
                "{}/s/player/{}/{}",
                YT_URL, player_id, PLAYER_JS_MAIN_VARIANT
            )),
        }
    }

    fn get_player_url(&self, ytcfgs: &[&HashMap<String, Value>]) -> Result<String> {
        for ytcfg in ytcfgs {
            if let Some(Value::String(url)) = ytcfg.get("PLAYER_JS_URL") {
                return Ok(self.construct_player_url(PlayerIdentifier::PlayerUrl(url.clone()))?);
            }

            if let Some(web_player_context) = ytcfg.get("WEB_PLAYER_CONTEXT_CONFIGS") {
                if let Some(obj) = web_player_context.as_object() {
                    for (_, v) in obj {
                        if let Some(js_url) = v
                            .get("jsUrl")
                            .and_then(|x| x.as_str().map(|s| s.to_string()))
                        {
                            return self.construct_player_url(PlayerIdentifier::PlayerUrl(js_url));
                        }
                    }
                }
            }
        }

        Ok(String::new())
    }

    fn invalid_player_response(
        &self,
        pr: &HashMap<String, Value>,
        video_id: &VideoId,
    ) -> Option<String> {
        // YouTube may return a different video player response than expected.
        let pr_id = pr
            .get("videoDetails")
            .and_then(|vd| vd.get("videoId"))
            .unwrap_or_default()
            .as_str()
            .unwrap_or_default();

        if pr_id != video_id.as_str() {
            return Some(pr_id.to_string());
        }

        None
    }

    fn is_age_gated(&self, player_response: &HashMap<String, Value>) -> bool {
        if player_response
            .get("playabilityStatus")
            .and_then(|ps| ps.get("desktopLegacyAgeGateReason"))
            .is_some()
        {
            return true;
        }

        let reasons_array: Vec<Value> = player_response
            .get("playabilityStatus")
            .and_then(|ps| ps.as_object())
            .and_then(|o| o.get("status"))
            .and_then(|s| s.as_object())
            .and_then(|s| s.get("reason"))
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_else(|| vec![]);

        let reasons: Vec<&str> = reasons_array
            .iter()
            .map(|x| x.as_str().unwrap_or_default())
            .collect();

        const AGE_GATE_REASONS: [&str; 5] = [
            "confirm your age",
            "age-restricted",
            "inappropriate",
            "age_verification_required",
            "age_check_required",
        ];

        for expected in AGE_GATE_REASONS {
            for reason in &reasons {
                if reason.contains(expected) {
                    return true;
                }
            }
        }

        false
    }

    fn is_unplayable(&self, player_response: &HashMap<String, Value>) -> bool {
        if let Some(status) = player_response
            .get("playabilityStatus")
            .and_then(|ps| ps.get("status"))
            .and_then(|s| s.as_str())
        {
            return status == "UNPLAYABLE";
        }

        false
    }

    async fn load_player(&self, video_id: &VideoId, player_url: String) -> Result<String> {
        let player_js_key = self.player_cache.player_js_cache_key(&player_url)?;

        if self.code_cache.contains(&player_js_key)? {
            return Ok(self.code_cache.get(&player_js_key)?.unwrap().clone());
        }

        let code = self
            .download_webpage(&player_url, &YtClient::Web, video_id)
            .await?;

        if !code.is_empty() {
            self.code_cache.add(player_js_key, code.clone())?;
        }

        Ok(code)
    }

    async fn extract_signature_timestamp(
        &self,
        video_id: &VideoId,
        player_url: String,
        ytcfg: &HashMap<String, Value>,
    ) -> Result<Option<i64>> {
        if let Some(sts) = ytcfg.get("STS") {
            return Ok(sts.as_i64());
        }

        if let Some(sts) = self
            .player_cache
            .load_player_data_from_cache("sts", player_url.clone())?
        {
            return Ok(Some(sts.parse::<i64>()?));
        }

        let code = self.load_player(video_id, player_url).await?;

        let re = Regex::new(r"(?:signatureTimestamp|sts)\s*:\s*(?P<sts>[0-9]{5})")?;
        let code_caps = re.captures(&code)?;

        let sts = if let Some(caps) = code_caps {
            if let Some(matched) = caps.name("sts") {
                matched.as_str().parse::<i64>()?
            } else {
                return Ok(None);
            }
        } else {
            return Ok(None);
        };

        Ok(Some(sts))
    }

    fn generate_player_context(&self, sts: Option<i64>) -> HashMap<String, Value> {
        let checkout_params = self.generate_checkok_params();

        let mut context: HashMap<String, Value> = hashmap! {
            "html5Preference".into() => "HTML5_PREF_WANTS".into(),
        };

        if let Some(valid_sts) = sts {
            context.insert("signatureTimestamp".into(), valid_sts.into());
        }

        let mut player_context: HashMap<String, Value> = hashmap! {
            "playbackContext".into() => json!({
                "contentPlaybackContext": context
            })
        };

        player_context.extend(checkout_params);
        player_context
    }

    async fn extract_player_response(
        &self,
        client: &YtClient,
        video_id: &VideoId,
        webpage_ytcfg: &HashMap<String, Value>,
        player_ytcfg: &HashMap<String, Value>,
        player_url: &Option<String>,
        initial_pr: &HashMap<String, Value>,
        visitor_data: &Option<String>,
        data_sync_id: &Option<String>,
    ) -> Result<HashMap<String, Value>> {
        let (parsed_data_sync_id, parsed_user_session_id) =
            self.parse_data_sync_id(data_sync_id.clone().unwrap_or_default());
        let delegated_session_id = if parsed_data_sync_id.is_some() {
            parsed_data_sync_id
        } else {
            self.get_delegated_session_id(&[webpage_ytcfg, initial_pr, player_ytcfg])
        };
        let user_session_id = if parsed_user_session_id.is_some() {
            parsed_user_session_id
        } else {
            self.get_user_session_id(&[webpage_ytcfg, initial_pr, player_ytcfg])
        };

        let parsed_session_index = self.get_session_index(&[webpage_ytcfg, player_ytcfg]);
        let mut yt_query: HashMap<String, Value> = hashmap! {
            "videoId".into() => video_id.as_str().into(),
        };

        let sts = self
            .extract_signature_timestamp(
                video_id,
                player_url.clone().unwrap_or_default(),
                player_ytcfg,
            )
            .await?;

        let headers = self.generate_api_headers(
            player_ytcfg.clone(),
            delegated_session_id,
            user_session_id,
            parsed_session_index,
            visitor_data.clone(),
            Some(client),
        )?;

        let player_context = self.generate_player_context(sts);

        yt_query.extend(player_context);

        let player_response = self
            .call_api(
                YtEndpoint::Player,
                yt_query,
                Some(headers),
                Some(self.select_context(Some(&player_ytcfg), Some(client))?),
                None,
                Some(client),
            )
            .await?;

        Ok(player_response)
    }

    async fn extract_player_responses(
        &self,
        clients: &Vec<YtClient>,
        video_id: &VideoId,
        webpage: &String,
        webpage_client: &YtClient,
        webpage_ytcfg: &HashMap<String, Value>,
    ) -> Result<(Vec<HashMap<String, Value>>, String)> {
        let initial_pr = self.search_json(r"ytInitialPlayerResponse\s*=", &webpage, None, None)?;
        let mut prs: Vec<HashMap<String, Value>> = vec![];

        let mut init_pr_copy = initial_pr.clone();
        init_pr_copy.insert("streamingData".into(), Value::Null);

        if !initial_pr.is_empty()
            && self
                .invalid_player_response(&initial_pr, video_id)
                .unwrap_or_default()
                .is_empty()
        {
            // Android player_response does not have microFormats which are needed for extraction of some data.
            // So we return the initial_pr with formats stripped out even if not requested by the user.
            prs.push(init_pr_copy);
        }

        let mut actual_clients = clients.clone();
        actual_clients.reverse();

        let mut tried_iframe_fallback = false;
        let mut player_url: Option<String> = None;
        let mut visitor_data: Option<String> = None;
        let mut data_sync_id: Option<String> = None;

        while !actual_clients.is_empty() {
            let popped_client = actual_clients.pop().unwrap();
            let client = popped_client.as_str();
            let variant = popped_client.get_variant();

            #[cfg(feature = "logging")]
            log::info!(
                "Extracting player response from \"{}\" client's manifest.",
                client
            );

            let player_ytcfg: &HashMap<String, Value> = if client == webpage_client.as_str() {
                webpage_ytcfg
            } else {
                &HashMap::new()
            };

            player_url =
                Some(player_url.unwrap_or(self.get_player_url(&[webpage_ytcfg, player_ytcfg])?));

            let require_js_player = self
                .select_default_ytcfg(Some(&popped_client))?
                .require_js_player;

            if player_url.is_none() && !tried_iframe_fallback && require_js_player {
                player_url = self.download_player_url(video_id).await?;
                tried_iframe_fallback = true;
            }

            if visitor_data.is_none() {
                visitor_data =
                    self.select_visitor_data(&[webpage_ytcfg, &initial_pr, player_ytcfg]);
            }

            if data_sync_id.is_none() {
                data_sync_id = self.get_data_sync_id(&[webpage_ytcfg, &initial_pr, player_ytcfg]);
            }

            // TODO: Implement PO Token fetching
            // let mut fetch_po_token_args: HashMap<String, Value> = HashMap::new();

            // fetch_po_token_args.insert("client".into(), client.into());
            // fetch_po_token_args.insert("visitor_data".into(), visitor_data.clone().into());
            // fetch_po_token_args.insert("video_id".into(), video_id.clone().into());
            // fetch_po_token_args.insert("data_sync_id".into(), data_sync_id.clone().into());
            // fetch_po_token_args.insert(
            //     "player_url".into(),
            //     if require_js_player {
            //         player_url.clone()
            //     } else {
            //         None
            //     }
            //     .into(),
            // );
            // fetch_po_token_args.insert("webpage".into(), webpage.clone().into());
            // fetch_po_token_args.insert(
            //     "session_index".into(),
            //     self.get_session_index(&[webpage_ytcfg, player_ytcfg])
            //         .into(),
            // );
            // fetch_po_token_args.insert(
            //     "ytcfg".into(),
            //     if player_ytcfg.is_empty() {
            //         self.select_default_ytcfg(Some(&popped_client))?
            //             .to_json_val_hashmap()?
            //     } else {
            //         player_ytcfg.clone()
            //     }
            //     .into_iter()
            //     .collect(),
            // );

            let mut player_response = self
                .extract_player_response(
                    &popped_client,
                    video_id,
                    if player_ytcfg.is_empty() {
                        webpage_ytcfg
                    } else {
                        player_ytcfg
                    },
                    player_ytcfg,
                    &player_url,
                    &initial_pr,
                    &visitor_data,
                    &data_sync_id,
                )
                .await?;

            if let Some(_invalid_pr_id) = self.invalid_player_response(&player_response, video_id) {
                #[cfg(feature = "logging")]
                log::warn!(
                    "Skipped {}. Received invalid player response for video with ID \"{}\", got {} instead.",
                    client,
                    video_id.as_str(),
                    _invalid_pr_id
                );
                continue;
            }

            if !player_response.is_empty() {
                let innertube_context = if player_ytcfg.is_empty() {
                    player_ytcfg
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<Map<String, Value>>()
                } else {
                    self.select_default_ytcfg(Some(&popped_client))?
                        .to_json_val_hashmap()?
                        .get("INNERTUBE_CONTEXT")
                        .unwrap()
                        .as_object()
                        .unwrap()
                        .clone()
                };

                player_response.insert(STREAMING_DATA_CLIENT_NAME.into(), client.into());
                player_response.insert(
                    STREAMING_DATA_INNERTUBE_CONTEXT.into(),
                    Value::Object(innertube_context),
                );

                prs.push(player_response.clone());
            }

            // web_embedded can work around age-gate and age-verification for some embeddable videos.
            if self.is_age_gated(&player_response) && variant != "web_embedded" {
                actual_clients.push(YtClient::WebEmbedded);
            }

            // Unauthenticated users will only get web_embedded client formats if age-gated.
            if self.is_age_gated(&player_response) && !self.is_authenticated()? {
                #[cfg(feature = "logging")]
                log::warn!(
                    "Skipping client \"{}\" since the video is age-restricted and unavailable without authentication.",
                    client
                );
                continue;
            }

            let embedding_is_disabled =
                variant == "web_embedded" && self.is_unplayable(&player_response);

            if self.is_authenticated()?
                && (self.is_age_gated(&player_response) || embedding_is_disabled)
            {
                #[cfg(feature = "logging")]
                log::warn!(
                    "Skipping client \"{}\" since the video is age-restricted and YouTube is requiring account verification.",
                    client
                );
                actual_clients.push(YtClient::TvEmbedded);
                actual_clients.push(YtClient::WebCreator);
                continue;
            }
        }

        if prs.is_empty() {
            return Err(anyhow!("Failed to extract any player response."));
        }

        Ok((prs, player_url.unwrap_or_default()))
    }
}
