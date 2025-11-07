use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
};

use anyhow::{Result, anyhow};
use fancy_regex::Regex;
// #[cfg(not(target_arch = "wasm32"))]
use reqwest::cookie;
use serde_json::Value;

use crate::{
    extractor::{
        auth::ExtractorAuthHandle, client::INNERTUBE_CLIENTS, download::ExtractorDownloadHandle,
        json::ExtractorJsonHandle, player::ExtractorPlayerHandle, ytcfg::ExtractorYtCfgHandle,
    },
    yt_interface::{VideoId, YtClient, YtStream, YtStreamSource},
};

pub struct YtExtractor {
    pub passed_auth_cookies: Cell<bool>,
    pub http_client: reqwest::Client,
    // #[cfg(not(target_arch = "wasm32"))]
    pub cookie_jar: cookie::Jar,
    pub code_cache: HashMap<String, String>,
    pub player_cache: HashMap<(String, String), String>,
}

pub trait InfoExtractor {
    fn extract_formats(
        &self,
        player_responses: Vec<HashMap<String, Value>>,
    ) -> Result<Vec<YtStream>>;
    async fn extract_streams(&mut self, video_id: &VideoId) -> Result<Vec<YtStream>>;
    fn generate_checkok_params(&self) -> HashMap<String, Value>;
    fn is_premium_subscriber(&self, initial_data: &HashMap<String, Value>) -> Result<bool>;
    fn extract_ytcfg(&self, webpage_content: String) -> Result<HashMap<String, Value>>;
    fn extract_yt_initial_data(&self, webpage_content: &String) -> Result<HashMap<String, Value>>;
    fn get_clients(&self, is_premium_subscriber: bool) -> Result<Vec<YtClient>>;
    async fn initial_extract(
        &mut self,
        webpage_url: &str,
        webpage_client: &YtClient,
        video_id: &VideoId,
    ) -> Result<Vec<HashMap<String, Value>>>;
}

impl YtExtractor {
    pub fn new() -> Result<Self> {
        let extractor = Self {
            passed_auth_cookies: Cell::new(false),
            http_client: reqwest::Client::new(),
            // #[cfg(not(target_arch = "wasm32"))]
            cookie_jar: cookie::Jar::default(),
            code_cache: HashMap::new(),
            player_cache: HashMap::new(),
            // x_forwarded_for_ip: None,
        };

        extractor.initialize_pref()?;
        extractor.initialize_consent()?;
        extractor.initialize_cookie_auth()?;

        Ok(extractor)
    }
}

impl InfoExtractor for YtExtractor {
    fn generate_checkok_params(&self) -> HashMap<String, Value> {
        let mut checkout_params_map = HashMap::new();

        checkout_params_map.insert("contentCheckOk".into(), true.into());
        checkout_params_map.insert("racyCheckOk".into(), true.into());

        checkout_params_map
    }

    fn is_premium_subscriber(&self, initial_data: &HashMap<String, Value>) -> Result<bool> {
        if !self.is_authenticated()? || initial_data.is_empty() {
            return Ok(false);
        }

        let tlr = initial_data
            .get("topbar")
            .and_then(|v| v.get("desktopTopbarRenderer"))
            .and_then(|v| v.get("logo"))
            .and_then(|v| v.get("topbarLogoRenderer"));
        let logo_match = tlr
            .and_then(|v| v.get("iconImage"))
            .and_then(|v| v.get("iconType"))
            .unwrap_or(&Value::Null);
        let logo_match_str = logo_match.as_str().unwrap_or_default();

        Ok(logo_match_str == "YOUTUBE_PREMIUM_LOGO"
            || self
                .get_text(
                    tlr.unwrap_or_default(),
                    Some(vec![vec!["tooltipText"]]),
                    None,
                )
                .unwrap_or_default()
                .to_lowercase()
                .contains("premium"))
    }

    fn extract_ytcfg(&self, webpage_content: String) -> Result<HashMap<String, Value>> {
        if webpage_content.is_empty() {
            return Ok(HashMap::new());
        }

        let search_re = Regex::new(r"ytcfg\.set\s*\(\s*({.+?})\s*\)\s*;")?;
        let json_str = search_re
            .captures(&webpage_content)?
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str())
            .unwrap_or("{}");

        let ytcfg: HashMap<String, Value> = serde_json::from_str(json_str)?;

        Ok(ytcfg)
    }

    fn extract_yt_initial_data(&self, webpage_content: &String) -> Result<HashMap<String, Value>> {
        let re = Regex::new(
            r#"(?:window\s*\[\s*["']ytInitialData["']\s*\]|ytInitialData)\s*=\s*(\{.*?\})\s*(?:;|</script>)"#,
        )?;
        let json_str = re
            .captures(&webpage_content)?
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str())
            .ok_or_else(|| anyhow!("ytInitialData not found"))?;

        let json_val: HashMap<String, Value> = serde_json::from_str(json_str)?;
        Ok(json_val)
    }

    fn get_clients(&self, is_premium_subscriber: bool) -> Result<Vec<YtClient>> {
        let mut clients = if is_premium_subscriber {
            // Premium does not require POT. (except for subtitles)
            vec![
                YtClient::Tv,
                YtClient::WebCreator,
                YtClient::WebSafari,
                YtClient::Web,
            ]
        } else if self.is_authenticated()? {
            vec![YtClient::Tv, YtClient::WebSafari, YtClient::Web]
        } else {
            vec![
                YtClient::AndroidSdkless,
                YtClient::Tv,
                YtClient::WebSafari,
                YtClient::Web,
            ]
        };

        if self.is_authenticated()? {
            let mut unsupported_clients = Vec::new();

            for client in &clients {
                if !INNERTUBE_CLIENTS.get(&client).unwrap().supports_cookies {
                    unsupported_clients.push(*client);
                }
            }

            for client in &unsupported_clients {
                println!(
                    "[WARN] Skipping client \"{}\" since it does not support cookies.",
                    client.as_str()
                );

                clients.retain(|c| !unsupported_clients.iter().any(|u| u.as_str() == c.as_str()));
            }
        }

        let mut seen = HashSet::new();
        let unique_clients: Vec<_> = clients.into_iter().filter(|c| seen.insert(*c)).collect();

        Ok(unique_clients)
    }

    fn extract_formats(
        &self,
        player_responses: Vec<HashMap<String, Value>>,
    ) -> Result<Vec<YtStream>> {
        let mut streams: Vec<YtStream> = vec![];

        for player_response in &player_responses {
            let streaming_formats = player_response.get("streamingData").unwrap_or_default();

            if streaming_formats.is_null() {
                continue;
            }

            let mut all_formats = Vec::new();

            if let Some(streaming_data) = player_response.get("streamingData") {
                if let Some(formats) = streaming_data.get("formats").and_then(|v| v.as_array()) {
                    all_formats.extend(formats.clone());
                }
                if let Some(adaptive_formats) = streaming_data
                    .get("adaptiveFormats")
                    .and_then(|v| v.as_array())
                {
                    all_formats.extend(adaptive_formats.clone());
                }
            }

            for fmt in all_formats {
                let target_duration_sec = fmt.get("targetDurationSec");

                // Skip livestream.
                if target_duration_sec.is_some() {
                    continue;
                }

                let itag = fmt
                    .get("itag")
                    .unwrap_or_default()
                    .as_str()
                    .and_then(|s| Some(s.to_string()));

                let mut quality = fmt
                    .get("quality")
                    .and_then(|s| Some(s.as_str().unwrap_or_default().to_string()));

                if quality.clone().unwrap_or_default() == "tiny" || quality.is_none() {
                    let audio_quality = fmt
                        .get("audioQuality")
                        .unwrap_or_default()
                        .as_str()
                        .unwrap_or_default()
                        .to_string();
                    quality = Some(audio_quality);
                }

                // The 3gp format (17) in android client has a quality of "small", but is actually worse than other formats.
                if itag.clone().unwrap_or_default() == "17" {
                    quality = Some("tiny".to_string());
                }

                let mut stream_source = None;

                if let Some(fmt_url) = fmt.get("url").clone() {
                    stream_source = Some(YtStreamSource::URL(
                        fmt_url.as_str().unwrap_or_default().to_string(),
                    ));
                }

                if let Some(sc) = fmt.get("signatureCipher").unwrap_or_default().as_str() {
                    stream_source = Some(YtStreamSource::Signature(sc.to_string()));
                }

                let Some(src) = stream_source else {
                    continue;
                };

                let tbr = fmt
                    .get("averageBitrate")
                    .or_else(|| fmt.get("bitrate"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1000 as f64);

                let yt_stream = YtStream::new(
                    fmt.get("audioSampleRate").and_then(|v| v.as_u64()),
                    fmt.get("contentLength")
                        .and_then(|v| v.as_str().and_then(|s| s.parse().ok())),
                    itag,
                    quality.and_then(|s| Some(s.to_lowercase())),
                    src,
                    tbr,
                );

                streams.push(yt_stream);
            }
        }

        Ok(streams)
    }

    async fn initial_extract(
        &mut self,
        webpage_url: &str,
        webpage_client: &YtClient,
        video_id: &VideoId,
    ) -> Result<Vec<HashMap<String, Value>>> {
        let webpage = self
            .download_webpage(webpage_url, webpage_client, video_id)
            .await?;

        let mut webpage_ytcfg = self.extract_ytcfg(webpage.clone())?;

        if webpage_ytcfg.is_empty() {
            webpage_ytcfg = self
                .select_default_ytcfg(Some(webpage_client))?
                .to_json_val_hashmap()?;
        }
        let initial_data = self
            .download_initial_data(video_id, &webpage, webpage_client, &webpage_ytcfg)
            .await?;

        let is_premium_subscriber = self.is_premium_subscriber(&initial_data)?;
        let clients = self.get_clients(is_premium_subscriber)?;
        let player_responses = self
            .extract_player_responses(&clients, video_id, &webpage, webpage_client, &webpage_ytcfg)
            .await?;

        Ok(player_responses)
    }

    async fn extract_streams(&mut self, video_id: &VideoId) -> Result<Vec<YtStream>> {
        // yt-dlp snippet: self.http_scheme() + "://"
        let webpage_url = "https://www.youtube.com/watch";
        let initial_extracted_data = self
            .initial_extract(webpage_url, &YtClient::Web, video_id)
            .await?;

        let formats = self.extract_formats(initial_extracted_data)?;

        Ok(formats)
    }
}
