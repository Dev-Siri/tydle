use anyhow::Result;
use std::pin::Pin;
#[cfg(feature = "cipher")]
use std::sync::Mutex as StdMutex;
#[cfg(target_arch = "wasm32")]
use std::sync::Mutex;
use std::{future::Future, sync::Arc};
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::Mutex;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

use crate::YtClient;
use crate::cache::CacheStore;
#[cfg(feature = "cipher")]
use crate::cipher::decipher::{SignatureDecipher, SignatureDecipherHandle};
use crate::cookies::DomainCookies;
use crate::yt_interface::{YtManifest, YtStreamResponse, YtVideoInfo};
use crate::{
    extractor::extract::{InfoExtractor, YtExtractor},
    yt_interface::VideoId,
};

#[cfg_attr(
    target_arch = "wasm32",
    derive(serde::Serialize, serde::Deserialize, tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "camelCase"),
    serde(default)
)]
#[derive(Default)]
pub struct TydleOptions {
    /// Map of cookies extracted from an authenticated YouTube account.
    pub auth_cookies: DomainCookies,
    /// Attempts to fetch over http instead of https.
    pub prefer_insecure: bool,
    /// Provide an address to set it as the `X-Forwarded-For` header when requesting YouTube.
    pub source_address: String,
    /// Provide a proxy domain address which tydle will request to instead of `www.youtube.com` in cases where you are CORS restricted.
    pub proxy_address: String,
    /// Provide a default client that tydle will use to request YouTube when it fetches without a specific client internally.
    pub default_client: YtClient,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct Tydle {
    yt_extractor: Arc<Mutex<YtExtractor>>,
    #[cfg(feature = "cipher")]
    signature_decipher: Arc<StdMutex<SignatureDecipher>>,
}

impl Tydle {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(options: TydleOptions) -> Result<Self> {
        let player_cache = Arc::new(CacheStore::new());
        let code_cache = Arc::new(CacheStore::new());

        let yt_extractor = YtExtractor::new(player_cache.clone(), code_cache.clone(), options)?;
        #[cfg(feature = "cipher")]
        let signature_decipher = SignatureDecipher::new(player_cache, code_cache);

        Ok(Self {
            yt_extractor: Arc::new(Mutex::new(yt_extractor)),
            #[cfg(feature = "cipher")]
            signature_decipher: Arc::new(StdMutex::new(signature_decipher)),
        })
    }
}

pub trait Extract {
    /// Extract the raw JSON manifest from YouTube's API.
    ///
    /// This method is useful if you need to fetch both the metadata and the streams of a particular video.
    /// Call this method once to extract the video's raw JSON manifest,
    /// and then pass it to either `Tydle::get_video_info_from_manifest` or `Tydle::get_streams_from_manifest`.
    /// It's better to use `Tydle::get_video_info` or `Tydle::get_streams` directly if you only
    /// need to fetch either and not both since they call `Tydle::get_manifest` themselves internally.
    ///
    /// ```
    /// use tydle::{Tydle, TydleOptions, Extract, VideoId, YtManifest};
    /// use anyhow::Result;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///   let ty = Tydle::new(TydleOptions{ ..Default::default() })?;
    ///
    ///   let video_id = VideoId::new("dQw4w9WgXcQ")?;
    ///
    ///   // Since you have this manifest separately, you can pass it to a fetcher.
    ///   let manifest: YtManifest = ty.get_manifest(&video_id).await?;
    ///   let streams = ty.get_streams_from_manifest(&manifest).await?;
    ///   let video_info = ty.get_video_info_from_manifest(&manifest).await?;
    ///
    ///   println!("Manifest: {:?}", manifest);
    ///   println!("Streams: {:?}", streams);
    ///   println!("Video Metadata: {:?}", video_info);
    ///   Ok(())
    /// }
    /// ```
    fn get_manifest<'a>(&'a self, video_id: &'a VideoId) -> Self::ExtractManifestFut<'a>;
    /// Extract the metadata of a video from YouTube.
    ///
    /// If you already have a raw manifest fetched, use `Tydle::get_video_info_from_manifest` instead to avoid refetching.
    ///
    /// ```
    /// use tydle::{Tydle, TydleOptions, Extract, VideoId, YtVideoInfo};
    /// use anyhow::Result;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///   let ty = Tydle::new(TydleOptions{ ..Default::default() })?;
    ///
    ///   let video_id = VideoId::new("dQw4w9WgXcQ")?;
    ///   let video_info: YtVideoInfo = ty.get_video_info(&video_id).await?;
    ///
    ///   println!("Video Metadata: {:?}", video_info);
    ///   Ok(())
    /// }
    /// ```
    fn get_video_info<'a>(&'a self, video_id: &'a VideoId) -> Self::ExtractInfoFut<'a>;
    /// Fetch and parse general video information (metadata) from an already fetched manifest.
    ///
    /// If you do not require using the manifest directly, use `Tydle::get_video_info` instead to fetch directly.
    ///
    /// ```
    /// use tydle::{Tydle, TydleOptions, Extract, VideoId, YtVideoInfo};
    /// use anyhow::Result;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///   let ty = Tydle::new(TydleOptions{ ..Default::default() })?;
    ///
    ///   let video_id = VideoId::new("dQw4w9WgXcQ")?;
    ///
    ///   let manifest = ty.get_manifest(&video_id).await?;
    ///   let video_info: YtVideoInfo = ty.get_video_info_from_manifest(&manifest).await?;
    ///
    ///   println!("Video Metadata: {:?}", video_info);
    ///   Ok(())
    /// }
    /// ```
    ///
    fn get_video_info_from_manifest<'a>(
        &'a self,
        manifest: &'a YtManifest,
    ) -> Self::ExtractInfoFut<'a>;
    /// Fetch and parse the streams from an already fetched manifest.
    ///
    /// If you do not require using the manifest directly, use `Tydle::get_streams` instead to fetch directly.
    ///
    /// ```
    /// use tydle::{Tydle, TydleOptions, Extract, VideoId, YtStreamResponse};
    /// use anyhow::Result;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///   let ty = Tydle::new(TydleOptions{ ..Default::default() })?;
    ///
    ///   let video_id = VideoId::new("dQw4w9WgXcQ")?;
    ///
    ///   let manifest = ty.get_manifest(&video_id).await?;
    ///   let stream_response: YtStreamResponse = ty.get_streams_from_manifest(&manifest).await?;
    ///
    ///   for stream in stream_response.streams {
    ///     println!("Stream: {:?}", stream);
    ///   }
    ///
    ///   Ok(())
    /// }
    /// ```
    ///
    fn get_streams_from_manifest<'a>(
        &'a self,
        manifest: &'a YtManifest,
    ) -> Self::ExtractStreamFut<'a>;
    /// Extract playable streams from YouTube and get their source either as a `Signature` or an `URL`
    ///
    /// If you already have a raw manifest fetched, use `Tydle::get_streams_from_manifest` instead to avoid refetching.
    ///
    /// ```
    /// use tydle::{Tydle, TydleOptions, Extract, VideoId, YtStreamResponse};
    /// use anyhow::Result;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///   let ty = Tydle::new(TydleOptions{ ..Default::default() })?;
    ///
    ///   let video_id = VideoId::new("dQw4w9WgXcQ")?;
    ///   let stream_response: YtStreamResponse = ty.get_streams(&video_id).await?;
    ///
    ///   for stream in stream_response.streams {
    ///     println!("Stream: {:?}", stream);
    ///   }
    ///
    ///   Ok(())
    /// }
    /// ```
    fn get_streams<'a>(&'a self, video_id: &'a VideoId) -> Self::ExtractStreamFut<'a>;

    type ExtractStreamFut<'a>: Future<Output = Result<YtStreamResponse>> + 'a
    where
        Self: 'a;
    type ExtractInfoFut<'a>: Future<Output = Result<YtVideoInfo>> + 'a
    where
        Self: 'a;
    type ExtractManifestFut<'a>: Future<Output = Result<YtManifest>> + 'a
    where
        Self: 'a;
}

#[cfg(feature = "cipher")]
pub trait Cipher {
    /// Deciphers a stream's signature and returns it's URL.
    fn decipher_signature<'a>(
        &'a self,
        signature: String,
        player_url: String,
    ) -> Self::DecipherFut<'a>;
    type DecipherFut<'a>: Future<Output = Result<String>> + 'a
    where
        Self: 'a;
}

impl Extract for Tydle {
    #[cfg(not(target_arch = "wasm32"))]
    type ExtractStreamFut<'a> = Pin<Box<dyn Future<Output = Result<YtStreamResponse>> + Send + 'a>>;
    #[cfg(not(target_arch = "wasm32"))]
    type ExtractInfoFut<'a> = Pin<Box<dyn Future<Output = Result<YtVideoInfo>> + Send + 'a>>;
    #[cfg(not(target_arch = "wasm32"))]
    type ExtractManifestFut<'a> = Pin<Box<dyn Future<Output = Result<YtManifest>> + Send + 'a>>;

    #[cfg(target_arch = "wasm32")]
    type ExtractStreamFut<'a> = Pin<Box<dyn Future<Output = Result<YtStreamResponse>> + 'a>>;
    #[cfg(target_arch = "wasm32")]
    type ExtractInfoFut<'a> = Pin<Box<dyn Future<Output = Result<YtVideoInfo>> + 'a>>;
    #[cfg(target_arch = "wasm32")]
    type ExtractManifestFut<'a> = Pin<Box<dyn Future<Output = Result<YtManifest>> + 'a>>;

    fn get_streams<'a>(&'a self, video_id: &'a VideoId) -> Self::ExtractStreamFut<'a> {
        Box::pin(async move {
            #[cfg(not(target_arch = "wasm32"))]
            let extractor = self.yt_extractor.lock().await;
            #[cfg(target_arch = "wasm32")]
            let extractor = self
                .yt_extractor
                .lock()
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            extractor.extract_streams(video_id).await
        })
    }

    fn get_manifest<'a>(&'a self, video_id: &'a VideoId) -> Self::ExtractManifestFut<'a> {
        Box::pin(async move {
            #[cfg(not(target_arch = "wasm32"))]
            let extractor = self.yt_extractor.lock().await;
            #[cfg(target_arch = "wasm32")]
            let extractor = self
                .yt_extractor
                .lock()
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            extractor.extract_manifest(video_id).await
        })
    }

    fn get_video_info<'a>(&'a self, video_id: &'a VideoId) -> Self::ExtractInfoFut<'a> {
        Box::pin(async move {
            #[cfg(not(target_arch = "wasm32"))]
            let extractor = self.yt_extractor.lock().await;
            #[cfg(target_arch = "wasm32")]
            let extractor = self
                .yt_extractor
                .lock()
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            extractor.extract_video_info(video_id).await
        })
    }

    fn get_streams_from_manifest<'a>(
        &'a self,
        manifest: &'a YtManifest,
    ) -> Self::ExtractStreamFut<'a> {
        Box::pin(async move {
            #[cfg(not(target_arch = "wasm32"))]
            let extractor = self.yt_extractor.lock().await;
            #[cfg(target_arch = "wasm32")]
            let extractor = self
                .yt_extractor
                .lock()
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            extractor.extract_streams_from_manifest(manifest).await
        })
    }

    fn get_video_info_from_manifest<'a>(
        &'a self,
        manifest: &'a YtManifest,
    ) -> Self::ExtractInfoFut<'a> {
        Box::pin(async move {
            #[cfg(not(target_arch = "wasm32"))]
            let extractor = self.yt_extractor.lock().await;
            #[cfg(target_arch = "wasm32")]
            let extractor = self
                .yt_extractor
                .lock()
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            extractor.extract_video_info_from_manifest(manifest).await
        })
    }
}

#[cfg(feature = "cipher")]
impl Cipher for Tydle {
    type DecipherFut<'a> = Pin<Box<dyn Future<Output = Result<String>> + 'a>>;

    fn decipher_signature<'a>(
        &'a self,
        signature: String,
        player_url: String,
    ) -> Self::DecipherFut<'a> {
        Box::pin(async move {
            let signature_decipher = self
                .signature_decipher
                .lock()
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            signature_decipher.decipher(signature, player_url).await
        })
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm_api {
    use super::*;
    use wasm_bindgen::JsValue;

    #[wasm_bindgen]
    impl Tydle {
        #[wasm_bindgen(constructor)]
        pub fn new(options: Option<TydleOptions>) -> Result<Tydle, JsValue> {
            let player_cache = Arc::new(CacheStore::new());
            let code_cache = Arc::new(CacheStore::new());

            let yt_extractor = YtExtractor::new(
                player_cache.clone(),
                code_cache.clone(),
                options.unwrap_or_default(),
            )
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

            let signature_decipher = SignatureDecipher::new(player_cache, code_cache);

            Ok(Tydle {
                yt_extractor: Arc::new(Mutex::new(yt_extractor)),
                signature_decipher: Arc::new(Mutex::new(signature_decipher)),
            })
        }

        #[wasm_bindgen(js_name = "fetchStreams")]
        pub async fn fetch_streams(
            &self,
            #[wasm_bindgen(js_name = "videoId")] video_id: String,
        ) -> Result<YtStreamResponse, JsValue> {
            let id = VideoId::new(&video_id).map_err(|e| JsValue::from_str(&e.to_string()))?;

            Ok(self
                .get_streams(&id)
                .await
                .map_err(|e| JsValue::from_str(&e.to_string()))?)
        }

        #[wasm_bindgen(js_name = "fetchVideoInfo")]
        pub async fn fetch_video_info(
            &self,
            #[wasm_bindgen(js_name = "videoId")] video_id: String,
        ) -> Result<YtVideoInfo, JsValue> {
            let id = VideoId::new(&video_id).map_err(|e| JsValue::from_str(&e.to_string()))?;

            Ok(self
                .get_video_info(&id)
                .await
                .map_err(|e| JsValue::from_str(&e.to_string()))?)
        }

        #[wasm_bindgen(js_name = "fetchVideoInfoFromManifest")]
        pub async fn fetch_video_info_from_manifest(
            &self,
            manifest: YtManifest,
        ) -> Result<YtVideoInfo, JsValue> {
            Ok(self
                .get_video_info_from_manifest(&manifest)
                .await
                .map_err(|e| JsValue::from_str(&e.to_string()))?)
        }

        #[wasm_bindgen(js_name = "fetchStreamsFromManifest")]
        pub async fn fetch_streams_from_manifest(
            &self,
            manifest: YtManifest,
        ) -> Result<YtStreamResponse, JsValue> {
            Ok(self
                .get_streams_from_manifest(&manifest)
                .await
                .map_err(|e| JsValue::from_str(&e.to_string()))?)
        }

        #[wasm_bindgen(js_name = "fetchManifest")]
        pub async fn fetch_manifest(
            &self,
            #[wasm_bindgen(js_name = "videoId")] video_id: String,
        ) -> Result<YtManifest, JsValue> {
            let id = VideoId::new(&video_id).map_err(|e| JsValue::from_str(&e.to_string()))?;

            Ok(self
                .get_manifest(&id)
                .await
                .map_err(|e| JsValue::from_str(&e.to_string()))?)
        }

        #[wasm_bindgen(js_name = "decipherSignature")]
        pub async fn decipher_signature_js(
            &self,
            signature: String,
            #[wasm_bindgen(js_name = "playerUrl")] player_url: String,
        ) -> Result<String, JsValue> {
            let res = self
                .decipher_signature(signature, player_url)
                .await
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            Ok(res)
        }
    }
}
