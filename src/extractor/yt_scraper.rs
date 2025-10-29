use anyhow::Result;
use reqwest::Url;

use crate::extractor::{
    client::INNERTUBE_CLIENTS,
    yt_interface::{VideoId, YtClient},
};

pub struct YtScraper {
    http_client: reqwest::Client,
}

pub trait Downloader {
    async fn download_initial_webpage(
        self,
        webpage_url: &str,
        webpage_client: &YtClient,
        video_id: &VideoId,
    ) -> Result<String>;
}

impl YtScraper {
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
        }
    }
}

impl Downloader for YtScraper {
    // ! DOES NOT YET IMPLEMENT PLAYER PARAMS
    async fn download_initial_webpage(
        self,
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
            webpage_request = webpage_request.header("User-Agent", *user_agent);
        }

        let response = webpage_request.send().await?;
        let webpage = response.text().await.map_err(|e| anyhow::Error::new(e))?;

        Ok(webpage)
    }
}
