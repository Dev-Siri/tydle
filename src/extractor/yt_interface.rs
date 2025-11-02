use core::fmt;
use std::str::FromStr;

use anyhow::{Result, anyhow};

#[derive(Debug)]
pub enum YtEndpoint {
    Browse,
    Player,
    Next,
}

impl YtEndpoint {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Browse => "browse",
            Self::Player => "player",
            Self::Next => "next",
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

pub const DEFAULT_YT_CLIENT: YtClient = YtClient::Web;
pub const PREFERRED_LOCALE: &str = "en";
pub const YT_URL: &str = "https://www.youtube.com";

pub const STREAMING_DATA_CLIENT_NAME: &str = "__yt_dlp_client";
pub const STREAMING_DATA_FETCH_SUBS_PO_TOKEN: &str = "__yt_dlp_fetch_subs_po_token";
pub const STREAMING_DATA_FETCH_GVS_PO_TOKEN: &str = "__yt_dlp_fetch_gvs_po_token";
pub const STREAMING_DATA_PLAYER_TOKEN_PROVIDED: &str = "__yt_dlp_player_token_provided";
pub const STREAMING_DATA_INNERTUBE_CONTEXT: &str = "__yt_dlp_innertube_context";
pub const STREAMING_DATA_IS_PREMIUM_SUBSCRIBER: &str = "__yt_dlp_is_premium_subscriber";
pub const STREAMING_DATA_FETCHED_TIMESTAMP: &str = "__yt_dlp_fetched_timestamp";

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

// Optional: allow parsing from string literals
impl FromStr for VideoId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

// Optional: pretty print
impl fmt::Display for VideoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
