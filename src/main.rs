use anyhow::Result;

use crate::extractor::{
    yt_interface::{VideoId, YtClient},
    yt_scraper::{Downloader, YtScraper},
};

mod extractor;

#[tokio::main]
async fn main() -> Result<()> {
    // extractor::api::call_api(None, YtEndpoint::Browse).await?;
    let scraper = YtScraper::new();
    let video_id = VideoId::new("")?;
    let webpage = scraper
        .download_initial_webpage("https://www.youtube.com", &YtClient::Web, &video_id)
        .await?;

    println!("{}", webpage);

    Ok(())
}
