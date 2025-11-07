use anyhow::Result;

use crate::extractor::{
    extract::{InfoExtractor, YtExtractor},
    yt_interface::{VideoId, YtStream},
};

pub struct Ty;

impl Ty {
    pub async fn extract(video_id: &VideoId) -> Result<Vec<YtStream>> {
        let mut yt_extractor = YtExtractor::new()?;

        let streams = yt_extractor.extract_streams(video_id).await?;

        Ok(streams)
    }
}
