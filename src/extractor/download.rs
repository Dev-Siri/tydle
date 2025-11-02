use std::collections::HashMap;

use anyhow::Result;
use serde_json::Value;
use url::Url;

use crate::extractor::{
    api::ExtractorApiHandle,
    client::INNERTUBE_CLIENTS,
    extract::{InfoExtractor, YtExtractor},
    yt_interface::{VideoId, YtClient, YtEndpoint},
    ytcfg::ExtractorYtCfgHandle,
};

pub trait ExtractorDownloadHandle {
    async fn download_initial_data(
        &self,
        video_id: &VideoId,
        webpage_content: String,
        webpage_client: &YtClient,
        webpage_ytcfg: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>>;
    async fn download_initial_webpage(
        &self,
        webpage_url: &str,
        webpage_client: &YtClient,
        video_id: &VideoId,
    ) -> Result<String>;
}

impl ExtractorDownloadHandle for YtExtractor {
    async fn download_initial_data(
        &self,
        video_id: &VideoId,
        webpage_content: String,
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
            query.insert("videoId", video_id.as_str());
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

    // ! DOES NOT YET IMPLEMENT PLAYER PARAMS
    async fn download_initial_webpage(
        &self,
        webpage_url: &str,
        webpage_client: &YtClient,
        video_id: &VideoId,
    ) -> Result<String> {
        let formatted_url = Url::parse(webpage_url)?;
        let watch_page_url = formatted_url.join("watch")?;
        let mut webpage_request = self.http_client.get(watch_page_url).query(&[
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

        let webpage = response.text().await.map_err(|e| anyhow::Error::new(e))?;

        Ok(webpage)
    }
}
