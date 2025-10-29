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
}

impl YtClient {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Web => "web",
        }
    }
}

pub const DEFAULT_YT_CLIENT: YtClient = YtClient::Web;

pub const SUPPORTED_LANGUAGE_CODES: [&str; 83] = [
    "af", "az", "id", "ms", "bs", "ca", "cs", "da", "de", "et", "en-IN", "en-GB", "en", "es",
    "es-419", "es-US", "eu", "fil", "fr", "fr-CA", "gl", "hr", "zu", "is", "it", "sw", "lv", "lt",
    "hu", "nl", "no", "uz", "pl", "pt-PT", "pt", "ro", "sq", "sk", "sl", "sr-Latn", "fi", "sv",
    "vi", "tr", "be", "bg", "ky", "kk", "mk", "mn", "ru", "sr", "uk", "el", "hy", "iw", "ur", "ar",
    "fa", "ne", "mr", "hi", "as", "bn", "pa", "gu", "or", "ta", "te", "kn", "ml", "si", "th", "lo",
    "my", "ka", "am", "km", "zh-CN", "zh-TW", "zh-HK", "ja", "ko",
];

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
