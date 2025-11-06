use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
};

use anyhow::{Result, anyhow};
use fancy_regex::Regex;
use reqwest::cookie;
use serde_json::Value;

use crate::extractor::{
    auth::ExtractorAuthHandle,
    client::INNERTUBE_CLIENTS,
    download::ExtractorDownloadHandle,
    json::ExtractorJsonHandle,
    player::ExtractorPlayerHandle,
    yt_interface::{VideoId, YtClient},
    ytcfg::ExtractorYtCfgHandle,
};

pub struct YtExtractor {
    pub passed_auth_cookies: Cell<bool>,
    pub http_client: reqwest::Client,
    pub cookie_jar: cookie::Jar,
    pub code_cache: HashMap<String, String>,
    pub player_cache: HashMap<(String, String), String>,
    // pub x_forwarded_for_ip: Option<&'static str>,
}

pub trait InfoExtractor {
    fn generate_checkok_params(&self) -> HashMap<String, Value>;
    fn is_music_url(&self, url: &str) -> Result<bool>;
    fn is_premium_subscriber(&self, initial_data: &HashMap<String, Value>) -> Result<bool>;
    fn extract_ytcfg(&self, webpage_content: String) -> Result<HashMap<String, Value>>;
    fn extract_yt_initial_data(&self, webpage_content: &String) -> Result<HashMap<String, Value>>;
    fn get_clients(
        &self,
        url: &str,
        smuggled_data: &HashMap<String, String>,
        is_premium_subscriber: bool,
    ) -> Result<Vec<YtClient>>;
    async fn initial_extract(
        &mut self,
        url: &str,
        smuggled_data: HashMap<String, String>,
        webpage_url: &str,
        webpage_client: &YtClient,
        video_id: &VideoId,
    ) -> Result<()>;
}

impl YtExtractor {
    pub fn new() -> Result<Self> {
        let extractor = Self {
            passed_auth_cookies: Cell::new(false),
            http_client: reqwest::Client::new(),
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

    fn is_music_url(&self, url: &str) -> Result<bool> {
        let re = Regex::new(r"(https?://)?music\.youtube\.com/")?;
        Ok(re.is_match(url)?)
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

    fn get_clients(
        &self,
        url: &str,
        smuggled_data: &HashMap<String, String>,
        is_premium_subscriber: bool,
    ) -> Result<Vec<YtClient>> {
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
            if smuggled_data.get("is_music_url").unwrap_or(&"".to_string()) == "true"
                || self.is_music_url(url)?
            {
                clients.push(YtClient::WebMusic);
            }

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

    async fn initial_extract(
        &mut self,
        url: &str,
        smuggled_data: HashMap<String, String>,
        webpage_url: &str,
        webpage_client: &YtClient,
        video_id: &VideoId,
    ) -> Result<()> {
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
        let clients = self.get_clients(url, &smuggled_data, is_premium_subscriber)?;
        let player_response = self
            .extract_player_responses(
                &clients,
                video_id,
                &webpage,
                webpage_client,
                &webpage_ytcfg,
                is_premium_subscriber,
            )
            .await?;

        Ok(player_response)
    }
}
