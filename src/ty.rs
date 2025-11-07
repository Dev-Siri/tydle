use anyhow::Result;

use crate::{
    extractor::extract::{InfoExtractor, YtExtractor},
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

// #[cfg(target_arch = "wasm32")]
// use wasm_bindgen::{JsValue, prelude::*};
// #[cfg(target_arch = "wasm32")]
// use wasm_bindgen_futures::wasm_bindgen::prelude::*;

// #[cfg(target_arch = "wasm32")]
// #[wasm_bindgen(js_name = "fetchYtStreams")]
// pub async fn wasm_fetch_yt_streams(video_id: &str) -> JsValue {
//     let Ok(video_id_parsed) = VideoId::new(video_id) else {
//         panic!("Invalid Video ID.")
//     };

//     match Ty::extract(&video_id_parsed).await {
//         Ok(streams) => JsValue::from_str(""),
//         Err(err) => panic!("{}", err),
//     }
// }
