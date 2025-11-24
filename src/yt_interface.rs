use core::fmt;
use std::{collections::HashMap, ops::Deref, str::FromStr};

use anyhow::{Result, anyhow, bail};
use serde::{Deserialize, Serialize};
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

#[cfg_attr(
    target_arch = "wasm32",
    derive(tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "camelCase")
)]
#[derive(
    Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Serialize, Deserialize,
)]
pub enum YtClient {
    #[default]
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
    pub fn from_str(str_client: &str) -> YtClient {
        match str_client {
            "web" => Self::Web,
            "web_safari" => Self::WebSafari,
            "web_embedded" => Self::WebEmbedded,
            "web_music" => Self::WebMusic,
            "web_creator" => Self::WebCreator,
            "android" => Self::Android,
            "android_sdkless" => Self::AndroidSdkless,
            "android_vr" => Self::AndroidVr,
            "ios" => Self::IOS,
            "mweb" => Self::MWeb,
            "tv" => Self::Tv,
            "tv_simply" => Self::TvSimply,
            "tv_embedded" => Self::TvEmbedded,
            _ => Self::Web, // Return a default client.
        }
    }

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

pub(crate) const PREFERRED_LOCALE: &str = "en";
pub(crate) const YT_DOMAIN: &str = ".youtube.com";
pub(crate) const YT_SUB_DOMAIN: &str = "www.youtube.com";
pub(crate) const YT_URL: &str = "https://www.youtube.com";

pub const STREAMING_DATA_CLIENT_NAME: &str = "__tydle_ytdlp_client";
// pub const STREAMING_DATA_FETCH_SUBS_PO_TOKEN: &str = "__tydle_ytdlp_fetch_subs_po_token";
// pub const STREAMING_DATA_FETCH_GVS_PO_TOKEN: &str = "__tydle_ytdlp_fetch_gvs_po_token";
pub const STREAMING_DATA_PLAYER_TOKEN_PROVIDED: &str = "__tydle_ytdlp_player_token_provided";
pub const STREAMING_DATA_INNERTUBE_CONTEXT: &str = "__tydle_ytdlp_innertube_context";
// pub const STREAMING_DATA_IS_PREMIUM_SUBSCRIBER: &str = "__tydle_ytdlp_is_premium_subscriber";
// pub const STREAMING_DATA_FETCHED_TIMESTAMP: &str = "__tydle_ytdlp_fetched_timestamp";
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
    derive(tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "camelCase")
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    derive(tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "lowercase")
)]
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum YtStreamSource {
    URL(String),
    Signature(String),
}

#[cfg_attr(
    target_arch = "wasm32",
    derive(tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "camelCase")
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YtStream {
    pub asr: Option<u64>,
    pub file_size: Option<u64>,
    pub file_size_approx: f64,
    pub height: Option<u64>,
    pub width: Option<u64>,
    pub format_duration: f64,
    pub has_drm: bool,
    pub itag: u16,
    pub source: YtStreamSource,
    pub source_preference: i16,
    pub tbr: f64,
    pub fps: u16,
    pub audio_track: AudioTrackInfo,
    pub quality_label: String,
    pub is_drc: bool,
    pub projection: Option<String>,
    pub spatial_audio: Option<String>,
    pub client: YtClient,
    pub ext: Ext,
    pub codec: Codec,
    pub is_dash: bool,
}

#[cfg_attr(
    target_arch = "wasm32",
    derive(tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi)
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Codec {
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
}

#[cfg_attr(
    target_arch = "wasm32",
    derive(tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "camelCase")
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTrackInfo {
    pub display_name: Option<String>,
    pub is_default: bool,
}

#[cfg_attr(target_arch = "wasm32", tsify::declare)]
pub type YtStreams = Vec<YtStream>;

#[cfg_attr(
    target_arch = "wasm32",
    derive(tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "camelCase")
)]
#[derive(Debug, Serialize, Deserialize)]
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
                    matches!(&s.codec.vcodec, Some(v) if v == "none")
                        && !matches!(&s.codec.acodec, Some(a) if a == "none")
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
                .filter(|s| !matches!(&s.codec.vcodec, Some(v) if v == "none"))
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
    derive(tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "camelCase")
)]
#[derive(Debug, Serialize, Deserialize)]
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
    derive(tsify::Tsify,),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "camelCase")
)]
#[derive(Debug, Serialize, Deserialize)]
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
    derive(tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "camelCase")
)]
#[derive(Debug, Default, Serialize, Deserialize)]
pub enum YtMediaType {
    LiveStream,
    Short,
    #[default]
    Video,
}

#[cfg_attr(
    target_arch = "wasm32",
    derive(tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "lowercase")
)]
#[derive(Debug, Default, Serialize, Deserialize)]
pub enum YtAgeLimit {
    Adult,
    #[default]
    None,
}

#[cfg_attr(
    target_arch = "wasm32",
    derive(tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "camelCase")
)]
#[derive(Debug, Serialize, Deserialize)]
pub struct YtThumbnail {
    pub url: String,
    pub height: Option<u64>,
    pub width: Option<u64>,
}

#[cfg_attr(
    target_arch = "wasm32",
    derive(tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "camelCase")
)]
#[derive(Debug, Serialize, Deserialize)]
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

#[cfg_attr(
    target_arch = "wasm32",
    derive(tsify::Tsify),
    tsify(into_wasm_abi, from_wasm_abi),
    serde(rename_all = "lowercase")
)]
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub enum Ext {
    #[default]
    Unknown,
    /// 3gp, renamed because identifiers can't start with a number.
    ThreeGp,
    Ts,
    Mp4,
    Mpeg,
    M3u8,
    Mov,
    Webm,
    Vp9,
    Ogv,
    Flv,
    M4v,
    Mkv,
    Mng,
    Asf,
    Wmv,
    Avi,
    Mpd,
    F4m,
    Ism,
    M4a,
    Mp3,
    Mka,
    M3u,
    Aac,
    Flac,
    Mid,
    Ogg,
    Wav,
    Ra,
    Avif,
    Bmp,
    Gif,
    Jpg,
    Png,
    Svg,
    Tif,
    Wbmp,
    Webp,
    Ico,
    Jng,
    Fs,
    Tt,
    Dfxp,
    Ttml,
    Sami,
    Gz,
    Json,
    Xml,
    Zip,
}

impl Ext {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Unknown => "unknown",
            Self::ThreeGp => "3gp",
            Self::Ts => "ts",
            Self::Mp4 => "mp4",
            Self::Mpeg => "mpeg",
            Self::M3u8 => "m3u8",
            Self::Mov => "mov",
            Self::Webm => "webm",
            Self::Vp9 => "vp9",
            Self::Ogv => "ogv",
            Self::Flv => "flv",
            Self::M4v => "m4v",
            Self::Mkv => "mkv",
            Self::Mng => "mng",
            Self::Asf => "asf",
            Self::Wmv => "wmv",
            Self::Avi => "avi",
            Self::Mpd => "mpd",
            Self::F4m => "f4m",
            Self::Ism => "ism",
            Self::M4a => "m4a",
            Self::Mp3 => "mp3",
            Self::Mka => "mka",
            Self::M3u => "m3u",
            Self::Aac => "aac",
            Self::Flac => "flac",
            Self::Mid => "mid",
            Self::Ogg => "ogg",
            Self::Wav => "wav",
            Self::Ra => "ra",
            Self::Avif => "avif",
            Self::Bmp => "bmp",
            Self::Gif => "gif",
            Self::Jpg => "jpg",
            Self::Png => "png",
            Self::Svg => "svg",
            Self::Tif => "tif",
            Self::Wbmp => "wbmp",
            Self::Webp => "webp",
            Self::Ico => "ico",
            Self::Jng => "jng",
            Self::Fs => "fs",
            Self::Tt => "tt",
            Self::Dfxp => "dfxp",
            Self::Ttml => "ttml",
            Self::Sami => "sami",
            Self::Gz => "gz",
            Self::Json => "json",
            Self::Xml => "xml",
            Self::Zip => "zip",
        }
    }
}
