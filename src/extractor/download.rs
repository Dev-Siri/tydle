use std::collections::HashMap;

use anyhow::{Error, Result};
use fancy_regex::Regex;
use reqwest::Url;
use serde_json::Value;

use crate::extractor::{
    api::ExtractorApiHandle,
    client::INNERTUBE_CLIENTS,
    extract::{InfoExtractor, YtExtractor},
    player::ExtractorPlayerHandle,
    yt_interface::{PlayerIdentifier, VideoId, YtClient, YtEndpoint},
    ytcfg::ExtractorYtCfgHandle,
};

pub trait ExtractorDownloadHandle {
    async fn download_direct_webpage(
        &self,
        webpage_url: &str,
        webpage_client: &YtClient,
        video_id: &VideoId,
    ) -> Result<String>;
    async fn download_initial_data(
        &self,
        video_id: &VideoId,
        webpage_content: &String,
        webpage_client: &YtClient,
        webpage_ytcfg: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>>;
    async fn download_player_url(&self, video_id: &VideoId) -> Result<Option<String>>;
    async fn download_webpage(
        &self,
        webpage_url: &str,
        webpage_client: &YtClient,
        video_id: &VideoId,
    ) -> Result<String>;
    async fn download_initial_webpage(
        &self,
        webpage_url: Url,
        webpage_client: &YtClient,
        video_id: &VideoId,
    ) -> Result<String>;
}

impl ExtractorDownloadHandle for YtExtractor {
    async fn download_initial_data(
        &self,
        video_id: &VideoId,
        webpage_content: &String,
        webpage_client: &YtClient,
        webpage_ytcfg: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>> {
        let mut initial_data: Option<HashMap<String, Value>> = if !webpage_content.is_empty() {
            Some(self.extract_yt_initial_data(webpage_content)?)
        } else {
            None
        };

        if initial_data.is_none() {
            let mut query = self.generate_checkok_params();
            query.insert("videoId".into(), video_id.as_str().into());

            initial_data = Some(
                self.call_api(
                    YtEndpoint::Next,
                    query,
                    None,
                    Some(self.select_context(Some(webpage_ytcfg), Some(webpage_client))?),
                    None,
                    Some(webpage_client),
                )
                .await?,
            );
        }

        Ok(initial_data.unwrap())
    }

    async fn download_player_url(&self, video_id: &VideoId) -> Result<Option<String>> {
        let formatted_url = Url::parse("https://www.youtube.com/iframe_api")?;
        let iframe_webpage = self
            .download_initial_webpage(formatted_url, &YtClient::Web, video_id)
            .await?;

        let player_version_re = Regex::new(r"player\\?/([0-9a-fA-F]{8})\\?/")?;
        let player_version = player_version_re.captures(&iframe_webpage)?;

        if let Some(caps) = player_version {
            if let Some(m) = caps.get(1) {
                return Ok(Some(self.construct_player_url(
                    PlayerIdentifier::PlayerId(m.as_str().to_string()),
                )?));
            }
        }

        Ok(None)
    }

    async fn download_direct_webpage(
        &self,
        webpage_url: &str,
        webpage_client: &YtClient,
        video_id: &VideoId,
    ) -> Result<String> {
        let formatted_url = Url::parse(webpage_url)?;
        self.download_initial_webpage(formatted_url, webpage_client, video_id)
            .await
    }

    async fn download_webpage(
        &self,
        webpage_url: &str,
        webpage_client: &YtClient,
        video_id: &VideoId,
    ) -> Result<String> {
        let formatted_url = Url::parse(webpage_url)?.join("watch")?;
        self.download_initial_webpage(formatted_url, webpage_client, video_id)
            .await
    }

    // ! DOES NOT YET IMPLEMENT PLAYER PARAMS
    async fn download_initial_webpage(
        &self,
        webpage_url: Url,
        webpage_client: &YtClient,
        video_id: &VideoId,
    ) -> Result<String> {
        let mut webpage_request = self.http_client.get(webpage_url).query(&[
            ("bpctr", "9999999999"),
            ("has_verified", "1"),
            ("v", video_id.as_str()),
        ]);
        let innertube_client = INNERTUBE_CLIENTS.get(webpage_client).unwrap();

        let client = innertube_client.innertube_context.get("client").unwrap();
        if let Some(user_agent) = client.get("userAgent") {
            webpage_request =
                webpage_request.header("User-Agent", user_agent.as_str().unwrap_or_default());
        }

        let response = webpage_request.send().await?;

        let webpage = response.text().await.map_err(|e| Error::new(e))?;

        Ok(webpage)
    }
}
