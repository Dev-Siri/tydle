use std::collections::HashMap;

use anyhow::Result;

use extractor::yt_interface::VideoId;

use crate::extractor::{
    extract::{InfoExtractor, YtExtractor},
    yt_interface::{YT_URL, YtClient},
};

mod extractor;
mod ty;

#[tokio::main]
async fn main() -> Result<()> {
    let mut extractor = YtExtractor::new()?;
    let video_id = VideoId::new("UWn9RdueB7E")?;

    match extractor
        .initial_extract(YT_URL, HashMap::new(), YT_URL, &YtClient::Web, &video_id)
        .await
    {
        Ok(_) => (),
        Err(e) => println!("Error during extraction: {:#?}", e.backtrace()),
    };

    Ok(())
}
