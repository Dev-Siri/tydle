use core::fmt;
use std::str::FromStr;

use anyhow::{Result, anyhow};
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum YtStreamSource {
    URL(String),
    Signature(String),
}

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

pub type YtStreams = Vec<YtStream>;

pub struct YtStreamList(YtStreams);

pub trait Filterable {
    /// Filter to return video-only streams.
    ///
    /// ```
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///   let ty = Ty::new();
    ///   // Get the stream with the lowest bitrate.
    ///   let video_only = ty
    ///      .fetch_streams(&VideoId::new("dQw4w9WgXcQ")?)
    ///      .video_only();
    /// }
    /// ```
    fn video_only(&self) -> YtStreamList;
    /// Filter to return audio-only streams.
    ///
    /// ```
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///   let ty = Ty::new();
    ///   // Get the stream with the lowest bitrate.
    ///   let audio_only = ty
    ///      .fetch_streams(&VideoId::new("dQw4w9WgXcQ")?)
    ///      .audio_only();
    /// }
    /// ```
    fn audio_only(&self) -> YtStreamList;
    /// Filter to return only those streams which do not require signature deciphering.
    ///
    /// ```
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///   let ty = Ty::new();
    ///   // Get the stream with the lowest bitrate.
    ///   let lowest_br_stream = ty
    ///      .fetch_streams(&VideoId::new("dQw4w9WgXcQ")?)
    ///      .with_lowest_bitrate()
    ///      .first();
    /// }
    /// ```
    fn with_lowest_bitrate(&self) -> YtStreamList;
    /// Sort streams to highest bitrate first.
    ///
    /// ```
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///   let ty = Ty::new();
    ///   // Get the stream with the highest bitrate.
    ///   let highest_br_stream = ty
    ///      .fetch_streams(&VideoId::new("dQw4w9WgXcQ")?)
    ///      .with_highest_bitrate()
    ///      .first();
    /// }
    /// ```
    fn with_highest_bitrate(&self) -> YtStreamList;
    /// Filter to return only those streams which require signature deciphering.
    /// For the purpose of signature deciphering, use `Ty::decipher_signature`
    ///
    /// ```
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///   let ty = Ty::new();
    ///   // Get direct URL streams.
    ///   let signature_streams = ty
    ///      .fetch_streams(&VideoId::new("dQw4w9WgXcQ")?)
    ///      .only_signatures()
    /// }
    /// ```
    fn only_signatures(&self) -> YtStreamList;
    /// Filter streams to return only those which do not require signature deciphering.
    ///
    /// ```
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///   let ty = Ty::new();
    ///   // Get direct URL streams.
    ///   let url_streams = ty
    ///      .fetch_streams(&VideoId::new("dQw4w9WgXcQ")?)
    ///      .only_urls()
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
