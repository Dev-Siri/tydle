use anyhow::Result;

use extractor::yt_interface::VideoId;

use crate::extractor::{
    extract::{InfoExtractor, YtExtractor},
    yt_interface::{YT_URL, YtClient},
};

mod extractor;

#[tokio::main]
async fn main() -> Result<()> {
    let extractor = YtExtractor::new()?;
    let video_id = VideoId::new("UWn9RdueB7E")?;

    extractor
        .initial_extract(YT_URL, "", YT_URL, &YtClient::Web, &video_id)
        .await?;

    Ok(())
}
