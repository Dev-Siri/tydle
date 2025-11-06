use std::collections::HashMap;

use anyhow::{Result, anyhow};
use fancy_regex::Regex;
use reqwest::Url;
use serde_json::{Value, json};

use crate::extractor::{
    api::ExtractorApiHandle,
    auth::ExtractorAuthHandle,
    cache::ExtractorCacheHandle,
    download::ExtractorDownloadHandle,
    extract::{InfoExtractor, YtExtractor},
    json::ExtractorJsonHandle,
    token_policy::PlayerPoTokenPolicy,
    yt_interface::{
        PLAYER_JS_MAIN_VARIANT, PlayerIdentifier, VideoId, YT_URL, YtClient, YtEndpoint,
    },
    ytcfg::ExtractorYtCfgHandle,
};

pub trait ExtractorPlayerHandle {
    fn generate_player_context(&self, sts: Option<i64>) -> HashMap<String, Value>;
    fn get_player_id_and_path(&self, player_url: &String) -> Result<(String, String)>;
    async fn load_player(&mut self, video_id: &VideoId, player_url: String) -> Result<String>;
    /// Extract `signatureTimestamp` (sts)
    /// Required to tell API what sig/player version is in use.
    async fn extract_signature_timestamp(
        &mut self,
        video_id: &VideoId,
        player_url: String,
        ytcfg: &HashMap<String, Value>,
    ) -> Result<Option<i64>>;
    fn construct_player_url(&self, player_identifier: PlayerIdentifier) -> Result<String>;
    fn extract_player_info(&self, player_url: &String) -> Result<String>;
    fn get_player_url(&self, ytcfgs: &[&HashMap<String, Value>]) -> Result<String>;
    fn invalid_player_response(
        &self,
        pr: &HashMap<String, Value>,
        video_id: &VideoId,
    ) -> Option<String>;
    async fn extract_player_response(
        &mut self,
        client: &YtClient,
        video_id: &VideoId,
        webpage_ytcfg: &HashMap<String, Value>,
        player_ytcfg: &HashMap<String, Value>,
        player_url: &Option<String>,
        initial_pr: &HashMap<String, Value>,
        visitor_data: &Option<String>,
        data_sync_id: &Option<String>,
        po_token: Option<String>,
    ) -> Result<HashMap<String, Value>>;
    async fn extract_player_responses(
        &mut self,
        clients: &Vec<YtClient>,
        video_id: &VideoId,
        webpage: &String,
        webpage_client: &YtClient,
        webpage_ytcfg: &HashMap<String, Value>,
        is_premium_subscriber: bool,
    ) -> Result<()>;
}

impl ExtractorPlayerHandle for YtExtractor {
    fn extract_player_info(&self, player_url: &String) -> Result<String> {
        const PLAYER_INFO_RE: [&str; 3] = [
            r"/s/player/(?P<id>[a-zA-Z0-9_-]{8,})/(?:tv-)?player",
            r"/(?P<id>[a-zA-Z0-9_-]{8,})/player(?:_ias\.vflset(?:/[a-zA-Z]{2,3}_[a-zA-Z]{2,3})?|-plasma-ias-(?:phone|tablet)-[a-z]{2}_[A-Z]{2}\.vflset)/base\.js$",
            r"\b(?P<id>vfl[a-zA-Z0-9_-]+)\b.*?\.js$",
        ];

        for player_info_re in PLAYER_INFO_RE {
            let re = Regex::new(player_info_re)?;
            if let Ok(Some(caps)) = re.captures(player_url) {
                if let Some(matched) = caps.name("id") {
                    return Ok(matched.as_str().to_string());
                }
            }
        }

        Err(anyhow!("Cannot identify player: {}", player_url))
    }

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

    fn get_player_id_and_path(&self, player_url: &String) -> Result<(String, String)> {
        let player_id = self.extract_player_info(player_url)?;
        let player_path = Url::parse(player_url)?.path().to_string();

        Ok((player_id, player_path))
    }

    async fn load_player(&mut self, video_id: &VideoId, player_url: String) -> Result<String> {
        let player_js_key = self.player_js_cache_key(&player_url)?;

        if self.code_cache.contains_key(&player_js_key) {
            return Ok(self.code_cache.get(&player_js_key).unwrap().clone());
        }

        let code = self
            .download_direct_webpage(&player_url, &YtClient::Web, video_id)
            .await?;

        if !code.is_empty() {
            self.code_cache.insert(player_js_key, code.clone());
        }

        Ok(code)
    }

    async fn extract_signature_timestamp(
        &mut self,
        video_id: &VideoId,
        player_url: String,
        ytcfg: &HashMap<String, Value>,
    ) -> Result<Option<i64>> {
        if let Some(sts) = ytcfg.get("STS") {
            return Ok(sts.as_i64());
        }

        if let Some(sts) = self.load_player_data_from_cache("sts", player_url.clone())? {
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

        let mut context: HashMap<String, Value> = HashMap::new();
        context.insert("html5Preference".into(), "HTML5_PREF_WANTS".into());

        if let Some(valid_sts) = sts {
            context.insert("signatureTimestamp".into(), valid_sts.into());
        }

        let mut player_context: HashMap<String, Value> = HashMap::new();

        player_context.insert(
            "playbackContext".into(),
            json!({
                "contentPlaybackContext": context
            }),
        );

        player_context.extend(checkout_params);
        player_context
    }

    async fn extract_player_response(
        &mut self,
        client: &YtClient,
        video_id: &VideoId,
        webpage_ytcfg: &HashMap<String, Value>,
        player_ytcfg: &HashMap<String, Value>,
        player_url: &Option<String>,
        initial_pr: &HashMap<String, Value>,
        visitor_data: &Option<String>,
        data_sync_id: &Option<String>,
        po_token: Option<String>,
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
        let mut yt_query: HashMap<String, Value> = HashMap::new();
        yt_query.insert("videoId".into(), video_id.as_str().into());

        // ! SKIPPED PLAYER PARAMS

        if let Some(po_tok) = po_token {
            yt_query.insert(
                "serviceIntegrityDimensions".into(),
                json!({
                  "poToken": po_tok
                }),
            );
        }

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

        Ok(HashMap::new())
    }

    async fn extract_player_responses(
        &mut self,
        clients: &Vec<YtClient>,
        video_id: &VideoId,
        webpage: &String,
        webpage_client: &YtClient,
        webpage_ytcfg: &HashMap<String, Value>,
        is_premium_subscriber: bool,
    ) -> Result<()> {
        let initial_pr = self.search_json(r"ytInitialPlayerResponse\s*=", &webpage, None, None)?;
        let mut prs: Vec<&HashMap<String, Value>> = vec![];
        let mut deprioritized_prs: Vec<&HashMap<String, Value>> = vec![];

        if !initial_pr.is_empty()
            && self
                .invalid_player_response(&initial_pr, video_id)
                .unwrap_or_default()
                .is_empty()
        {
            // Android player_response does not have microFormats which are needed for extraction of some data.
            // So we return the initial_pr with formats stripped out even if not requested by the user.
            let mut init_pr_copy = initial_pr.clone();
            init_pr_copy.insert("streamingData".into(), Value::Null);
            prs.push(&init_pr_copy);
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
            let base_client = popped_client.get_base();
            let variant = popped_client.get_variant();

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

            let pr: Option<HashMap<String, Value>> = if client == webpage_client.as_str() {
                Some(initial_pr.clone())
            } else {
                None
            };

            if visitor_data.is_none() {
                visitor_data =
                    self.select_visitor_data(&[webpage_ytcfg, &initial_pr, player_ytcfg]);
            }

            if data_sync_id.is_none() {
                data_sync_id = self.get_data_sync_id(&[webpage_ytcfg, &initial_pr, player_ytcfg]);
            }

            // TODO: Implement PO Token fetching
            let mut fetch_po_token_args: HashMap<String, Value> = HashMap::new();

            fetch_po_token_args.insert("client".into(), client.into());
            fetch_po_token_args.insert("visitor_data".into(), visitor_data.clone().into());
            fetch_po_token_args.insert("video_id".into(), video_id.clone().into());
            fetch_po_token_args.insert("data_sync_id".into(), data_sync_id.clone().into());
            fetch_po_token_args.insert(
                "player_url".into(),
                if require_js_player {
                    player_url.clone()
                } else {
                    None
                }
                .into(),
            );
            fetch_po_token_args.insert("webpage".into(), webpage.clone().into());
            fetch_po_token_args.insert(
                "session_index".into(),
                self.get_session_index(&[webpage_ytcfg, player_ytcfg])
                    .into(),
            );
            fetch_po_token_args.insert(
                "ytcfg".into(),
                if player_ytcfg.is_empty() {
                    self.select_default_ytcfg(Some(&popped_client))?
                        .to_json_val_hashmap()?
                } else {
                    player_ytcfg.clone()
                }
                .into_iter()
                .collect(),
            );

            let player_pot_policy = self
                .select_default_ytcfg(Some(&popped_client))?
                .player_po_token_policy;

            let player_po_token: Option<String> = None;

            let extracted_player_response = self
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
                    player_po_token,
                )
                .await?;
        }

        // println!("{:?}", webpage_ytcfg);
        // println!("{}", webpage);
        // println!("{:#?}", initial_pr);

        Ok(())
    }
}
