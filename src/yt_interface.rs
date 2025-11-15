use core::fmt;
use std::{collections::HashMap, ops::Deref, str::FromStr};

use anyhow::{Result, anyhow, bail};
use serde_json::Value;

#[derive(Debug)]
pub enum YtEndpoint {
    // Browse,
    Player,
    Next,
}

impl YtEndpoint {
    pub fn as_str(&self) -> &'static str {
        match self {
            // Self::Browse => "browse",
            Self::Player => "player",
            Self::Next => "next",
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum YtClient {
    Web,
    /// Safari UA returns pre-merged video+audio 144p/240p/360p/720p/1080p HLS formats.
    WebSafari,
    WebEmbedded,
    WebMusic,
    /// This client now requires sign-in for every video.
    WebCreator,
    Android,
    /// Doesn't require a PoToken.
    AndroidSdkless,
    /// YouTube Kids videos aren't returned on this client.
    AndroidVr,
    /// iOS clients have HLS live streams. Setting device model to get 60fps formats.
    IOS,
    // mweb has 'ultralow' formats.
    MWeb,
    Tv,
    TvSimply,
    /// This client now requires sign-in for every video.
    /// It was previously an age-gate workaround for videos that were `playable_in_embed`
    /// It may still be useful if signed into an EU account that is not age-verified.
    TvEmbedded,
}

impl YtClient {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::WebSafari => "web_safari",
            Self::WebEmbedded => "web_embedded",
            Self::WebMusic => "web_music",
            Self::WebCreator => "web_creator",
            Self::Android => "android",
            Self::AndroidSdkless => "android_sdkless",
            Self::AndroidVr => "android_vr",
            Self::IOS => "ios",
            Self::MWeb => "mweb",
            Self::Tv => "tv",
            Self::TvSimply => "tv_simply",
            Self::TvEmbedded => "tv_embedded",
        }
    }

    pub fn get_variant(&self) -> &'static str {
        self.as_str()
            .split_once('_')
            .map(|(_, v)| v)
            .unwrap_or(self.as_str())
    }

    pub fn get_base(&self) -> &'static str {
        self.as_str()
            .split_once('_')
            .map(|(b, _)| b)
            .unwrap_or(self.as_str())
    }
}

pub(crate) const DEFAULT_YT_CLIENT: YtClient = YtClient::Web;
pub(crate) const PREFERRED_LOCALE: &str = "en";
pub(crate) const YT_DOMAIN: &str = ".youtube.com";
pub(crate) const YT_URL: &str = "https://www.youtube.com";

pub const AUDIO_ONLY_FORMATS: [&str; 4] = [
    "audio_quality_ultralow",
    "audio_quality_low",
    "audio_quality_medium",
    "audio_quality_high",
];

pub const VIDEO_ONLY_FORMATS: [&str; 10] = [
    "tiny", "small", "medium", "large", "hd720", "hd1080", "hd1440", "hd2160", "hd2880", "highres",
];

// pub const STREAMING_DATA_CLIENT_NAME: &str = "__yt_dlp_client";
// pub const STREAMING_DATA_FETCH_SUBS_PO_TOKEN: &str = "__yt_dlp_fetch_subs_po_token";
// pub const STREAMING_DATA_FETCH_GVS_PO_TOKEN: &str = "__yt_dlp_fetch_gvs_po_token";
// pub const STREAMING_DATA_PLAYER_TOKEN_PROVIDED: &str = "__yt_dlp_player_token_provided";
// pub const STREAMING_DATA_INNERTUBE_CONTEXT: &str = "__yt_dlp_innertube_context";
// pub const STREAMING_DATA_IS_PREMIUM_SUBSCRIBER: &str = "__yt_dlp_is_premium_subscriber";
// pub const STREAMING_DATA_FETCHED_TIMESTAMP: &str = "__yt_dlp_fetched_timestamp";
// pub const DEFAULT_PLAYER_JS_VERSION: &str = "actual";
// pub const DEFAULT_PLAYER_JS_VARIANT: &str = "main";

pub const PLAYER_JS_MAIN_VARIANT: &str = "player_ias.vflset/en_US/base.js";
// pub const PLAYER_JS_INVERSE_MAIN_VARIANT: &str = "main";

// pub static PLAYER_JS_VARIANT_MAP: Lazy<HashMap<&str, &str>> = Lazy::new(|| {
//     let mut m = HashMap::new();

//     m.insert("main", "player_ias.vflset/en_US/base.js");
//     m.insert("tcc", "player_ias_tcc.vflset/en_US/base.js");
//     m.insert("tce", "player_ias_tce.vflset/en_US/base.js");
//     m.insert("es5", "player_es5.vflset/en_US/base.js");
//     m.insert("es6", "player_es6.vflset/en_US/base.js");
//     m.insert("tv", "tv-player-ias.vflset/tv-player-ias.js");
//     m.insert("tv_es6", "tv-player-es6.vflset/tv-player-es6.js");
//     m.insert("phone", "player-plasma-ias-phone-en_US.vflset/base.js");
//     m.insert("tablet", "player-plasma-ias-tablet-en_US.vflset/base.js");

//     m
// });

// pub static INVERSE_PLAYER_JS_VARIANT_MAP: Lazy<HashMap<&str, &str>> = Lazy::new(|| {
//     let mut m = HashMap::new();

//     for (k, v) in PLAYER_JS_VARIANT_MAP.iter() {
//         m.insert(*v, *k);
//     }

//     m
// });

pub enum PlayerIdentifier {
    PlayerId(String),
    PlayerUrl(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct VideoId(String);

impl VideoId {
    pub fn new<S: Into<String>>(s: S) -> Result<Self> {
        let s = s.into();
        if s.len() != 11 {
            return Err(anyhow!(
                "invalid length: expected 11 characters, got {}",
                s.len()
            ));
        }

        if !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(anyhow!("invalid characters in video ID: {}", s));
        }

        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<VideoId> for Value {
    fn from(value: VideoId) -> Self {
        Value::String(value.0)
    }
}

impl FromStr for VideoId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl fmt::Display for VideoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg_attr(
    target_arch = "wasm32",
    derive(serde::Serialize, serde::Deserialize, tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "camelCase")
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct YtChannel {
    id: String,
    name: Option<String>,
}

impl YtChannel {
    pub fn new<S: Into<String>>(id: S, name: Option<String>) -> Result<Self> {
        let id = id.into();
        if id.starts_with("UC") && id.len() == 24 {
            Ok(Self { id, name })
        } else {
            bail!("Invalid channel ID for Channel: {}", id)
        }
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn get_url(&self) -> String {
        format!("{}/channel/{}", YT_URL, self.id)
    }
}

#[cfg_attr(
    target_arch = "wasm32",
    derive(serde::Serialize, serde::Deserialize, tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "lowercase")
)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum YtStreamSource {
    URL(String),
    Signature(String),
}

#[cfg_attr(
    target_arch = "wasm32",
    derive(serde::Serialize, serde::Deserialize, tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "camelCase")
)]
#[derive(Debug, Clone)]
pub struct YtStream {
    pub asr: Option<u64>,
    pub file_size: Option<u64>,
    pub itag: Option<String>,
    pub quality: Option<String>,
    pub source: YtStreamSource,
    pub tbr: f64,
}

impl YtStream {
    pub fn new(
        asr: Option<u64>,
        file_size: Option<u64>,
        itag: Option<String>,
        quality: Option<String>,
        source: YtStreamSource,
        tbr: f64,
    ) -> Self {
        Self {
            asr,
            file_size,
            itag,
            quality,
            source,
            tbr,
        }
    }
}

#[cfg_attr(target_arch = "wasm32", tsify::declare)]
pub type YtStreams = Vec<YtStream>;

#[cfg_attr(
    target_arch = "wasm32",
    derive(serde::Serialize, serde::Deserialize, tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "camelCase")
)]
#[derive(Debug)]
pub struct YtStreamList(YtStreams);

impl<'a> IntoIterator for &'a YtStreamList {
    type Item = &'a YtStream;
    type IntoIter = std::slice::Iter<'a, YtStream>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl IntoIterator for YtStreamList {
    type Item = YtStream;
    type IntoIter = std::vec::IntoIter<YtStream>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Deref for YtStreamList {
    type Target = Vec<YtStream>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub trait Filterable {
    /// Filter to return video-only streams.
    ///
    /// ```
    /// use tydle::{Tydle, TydleOptions, VideoId, Extract, Filterable};
    /// use anyhow::Result;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///   let ty = Tydle::new(TydleOptions { ..Default::default() })?;
    ///   // Get the stream with the lowest bitrate.
    ///   let video_only = ty
    ///      .get_streams(&VideoId::new("dQw4w9WgXcQ")?)
    ///      .await?
    ///      .streams
    ///      .video_only();
    ///
    ///    println!("Video-Only streams: {:?}", video_only);
    ///    Ok(())
    /// }
    /// ```
    fn video_only(&self) -> YtStreamList;
    /// Filter to return audio-only streams.
    ///
    /// ```
    /// use tydle::{Tydle, TydleOptions, Extract, VideoId, Filterable};
    /// use anyhow::Result;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///   let ty = Tydle::new(TydleOptions { ..Default::default() })?;
    ///   // Get the stream with the lowest bitrate.
    ///   let audio_only = ty
    ///      .get_streams(&VideoId::new("dQw4w9WgXcQ")?)
    ///      .await?
    ///      .streams
    ///      .audio_only();
    ///
    ///    println!("Audio-Only streams: {:?}", audio_only);
    ///    Ok(())
    /// }
    /// ```
    fn audio_only(&self) -> YtStreamList;
    /// Filter to return only those streams which do not require signature deciphering.
    ///
    /// ```
    /// use tydle::{Tydle, TydleOptions, Extract, VideoId, Filterable};
    /// use anyhow::Result;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///   let ty = Tydle::new(TydleOptions { ..Default::default() })?;
    ///   // Get the stream with the lowest bitrate.
    ///   let lowest_br_streams = ty
    ///      .get_streams(&VideoId::new("dQw4w9WgXcQ")?)
    ///      .await?
    ///      .streams
    ///      .with_lowest_bitrate();
    ///
    ///   println!("Lowest bitrate stream: {:?}", lowest_br_streams.first());
    ///   Ok(())
    /// }
    /// ```
    fn with_lowest_bitrate(&self) -> YtStreamList;
    /// Sort streams to highest bitrate first.
    ///
    /// ```
    /// use tydle::{Tydle, TydleOptions, Extract, VideoId, Filterable};
    /// use anyhow::Result;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///   let ty = Tydle::new(TydleOptions { ..Default::default() })?;
    ///   // Get the streams with the highest bitrate.
    ///   let highest_br_streams = ty
    ///      .get_streams(&VideoId::new("dQw4w9WgXcQ")?)
    ///      .await?
    ///      .streams
    ///      .with_highest_bitrate();
    ///
    ///   println!("Highest bitrate stream: {:?}", highest_br_streams.first());
    ///   Ok(())
    /// }
    /// ```
    fn with_highest_bitrate(&self) -> YtStreamList;
    /// Filter to return only those streams which require signature deciphering.
    /// For the purpose of signature deciphering, use `Tydle::decipher_signature`
    ///
    /// ```
    /// use tydle::{Tydle, TydleOptions, Extract, VideoId, Filterable};
    /// use anyhow::Result;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///   let ty = Tydle::new(TydleOptions { ..Default::default() })?;
    ///   // Get direct signature streams.
    ///   let signature_streams = ty
    ///      .get_streams(&VideoId::new("dQw4w9WgXcQ")?)
    ///      .await?
    ///      .streams
    ///      .only_signatures();
    ///
    ///   for stream in signature_streams {
    ///     println!("Signature: {:?}", stream.source);
    ///   }
    ///
    ///   Ok(())
    /// }
    /// ```
    fn only_signatures(&self) -> YtStreamList;
    /// Filter streams to return only those which do not require signature deciphering.
    ///
    /// ```
    /// use tydle::{Tydle, TydleOptions, Extract, VideoId, Filterable};
    /// use anyhow::Result;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///   let ty = Tydle::new(TydleOptions { ..Default::default() })?;
    ///   // Get direct URL streams.
    ///   let url_streams = ty
    ///      .get_streams(&VideoId::new("dQw4w9WgXcQ")?)
    ///      .await?
    ///      .streams
    ///      .only_urls();
    ///
    ///   for stream in url_streams {
    ///     println!("URL: {:?}", stream.source);
    ///   }
    ///
    ///   Ok(())
    /// }
    /// ```
    fn only_urls(&self) -> YtStreamList;
}

impl Filterable for YtStreamList {
    fn with_highest_bitrate(&self) -> YtStreamList {
        let mut streams = self.0.clone();
        streams.sort_by(|a, b| {
            b.tbr
                .partial_cmp(&a.tbr)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        YtStreamList(streams)
    }

    fn audio_only(&self) -> YtStreamList {
        let streams = self.0.clone();
        YtStreamList(
            streams
                .iter()
                .filter(|s| {
                    AUDIO_ONLY_FORMATS.contains(&s.quality.clone().unwrap_or_default().as_str())
                })
                .cloned()
                .collect(),
        )
    }

    fn video_only(&self) -> YtStreamList {
        let streams = self.0.clone();
        YtStreamList(
            streams
                .iter()
                .filter(|s| {
                    VIDEO_ONLY_FORMATS.contains(&s.quality.clone().unwrap_or_default().as_str())
                })
                .cloned()
                .collect(),
        )
    }

    fn with_lowest_bitrate(&self) -> YtStreamList {
        let mut streams = self.0.clone();
        streams.sort_by(|a, b| {
            a.tbr
                .partial_cmp(&b.tbr)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        YtStreamList(streams)
    }

    fn only_urls(&self) -> YtStreamList {
        YtStreamList(
            self.0
                .iter()
                .filter(|s| matches!(s.source, YtStreamSource::URL(_)))
                .cloned()
                .collect(),
        )
    }

    fn only_signatures(&self) -> YtStreamList {
        YtStreamList(
            self.0
                .iter()
                .filter(|s| matches!(s.source, YtStreamSource::Signature(_)))
                .cloned()
                .collect(),
        )
    }
}

#[cfg_attr(
    target_arch = "wasm32",
    derive(serde::Serialize, serde::Deserialize, tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "camelCase")
)]
#[derive(Debug)]
pub struct YtStreamResponse {
    pub player_url: String,
    pub streams: YtStreamList,
}

impl YtStreamResponse {
    pub fn new(player_url: String, streams: YtStreams) -> Self {
        Self {
            player_url,
            streams: YtStreamList(streams),
        }
    }
}

#[cfg_attr(
    target_arch = "wasm32",
    derive(serde::Serialize, serde::Deserialize, tsify::Tsify,),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "camelCase")
)]
#[derive(Debug)]
pub struct YtManifest {
    pub extracted_manifest: Vec<HashMap<String, Value>>,
    pub player_url: String,
}

impl YtManifest {
    pub fn new(extracted_manifest: Vec<HashMap<String, Value>>, player_url: String) -> Self {
        Self {
            extracted_manifest,
            player_url,
        }
    }
}

#[cfg_attr(
    target_arch = "wasm32",
    derive(serde::Serialize, serde::Deserialize, tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "camelCase")
)]
#[derive(Debug, Default)]
pub enum YtMediaType {
    LiveStream,
    Short,
    #[default]
    Video,
}

#[cfg_attr(
    target_arch = "wasm32",
    derive(serde::Serialize, serde::Deserialize, tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "lowercase")
)]
#[derive(Debug, Default)]
pub enum YtAgeLimit {
    Adult,
    #[default]
    None,
}

#[cfg_attr(
    target_arch = "wasm32",
    derive(serde::Serialize, serde::Deserialize, tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "camelCase")
)]
#[derive(Debug)]
pub struct YtThumbnail {
    pub url: String,
    pub height: Option<u64>,
    pub width: Option<u64>,
}

#[cfg_attr(
    target_arch = "wasm32",
    derive(serde::Serialize, serde::Deserialize, tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "camelCase")
)]
#[derive(Debug)]
pub struct YtVideoInfo {
    pub title: String,
    pub description: String,
    /// Rounded-off duration of the video in seconds.
    pub duration: u64,
    pub view_count: u64,
    pub channel: YtChannel,
    pub keywords: Vec<String>,
    pub thumbnails: Vec<YtThumbnail>,
    pub media_type: YtMediaType,
    pub age_limit: YtAgeLimit,
}
