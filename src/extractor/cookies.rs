use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
// #[cfg(not(target_arch = "wasm32"))]
use reqwest::{Url, cookie::CookieStore};
// #[cfg(target_arch = "wasm32")]
// use reqwest::{Url, header::HeaderValue};
use sha1::{Digest, Sha1};

use crate::{extractor::extract::YtExtractor, yt_interface::YT_URL};

pub struct SidCookies {
    pub yt_sapisid: Option<String>,
    pub yt_1psapisid: Option<String>,
    pub yt_3psapisid: Option<String>,
}

impl SidCookies {
    pub fn new(
        yt_sapisid: Option<String>,
        yt_1psapisid: Option<String>,
        yt_3psapisid: Option<String>,
    ) -> Self {
        Self {
            yt_sapisid,
            yt_1psapisid,
            yt_3psapisid,
        }
    }
}
pub trait ExtractorCookieHandle {
    fn get_cookies(&self, url: &str) -> Result<HashMap<String, String>>;
    fn get_youtube_cookies(&self) -> Result<HashMap<String, String>>;
    /// Get SAPISID, 1PSAPISID, 3PSAPISID cookie values.
    fn get_sid_cookies(&self) -> Result<SidCookies>;
    fn make_sid_authorization(
        &self,
        scheme: &'static str,
        sid: String,
        origin: String,
        additional_parts: HashMap<&str, String>,
    ) -> Result<String>;
    /// Generate API Session ID Authorization for Innertube requests. Assumes all requests are secure. (HTTPS)
    fn get_sid_authorization_header(
        &self,
        origin: Option<String>,
        user_session_id: Option<String>,
    ) -> Result<Option<String>>;
}

impl ExtractorCookieHandle for YtExtractor {
    fn get_cookies(&self, url: &str) -> Result<HashMap<String, String>> {
        // #[cfg(not(target_arch = "wasm32"))]
        let url = Url::parse(url)?;
        // #[cfg(not(target_arch = "wasm32"))]
        let cookies = self.cookie_jar.cookies(&url);
        // #[cfg(target_arch = "wasm32")]
        // let cookies: Option<HeaderValue> = None;
        let cookies_str = match cookies {
            Some(cookie_val) => cookie_val.to_str()?.to_owned(),
            None => String::new(),
        };

        let cookies_map: HashMap<String, String> = cookies_str
            .split("; ")
            .filter_map(|pair| {
                let mut parts = pair.splitn(2, '=');
                Some((parts.next()?.to_string(), parts.next()?.to_string()))
            })
            .collect();

        Ok(cookies_map)
    }

    fn get_youtube_cookies(&self) -> Result<HashMap<String, String>> {
        self.get_cookies(YT_URL)
    }

    fn get_sid_cookies(&self) -> Result<SidCookies> {
        let yt_cookies = self.get_youtube_cookies()?;
        let yt_sapisid = yt_cookies.get("SAPISID").cloned();
        let yt_3papisid = yt_cookies.get("__Secure-3PAPISID").cloned();
        let yt_1papisid = yt_cookies.get("__Secure-1PAPISID").cloned();
        let sid_cookies = SidCookies::new(
            yt_sapisid.or_else(|| yt_3papisid.clone()),
            yt_1papisid,
            yt_3papisid,
        );

        Ok(sid_cookies)
    }

    fn make_sid_authorization(
        &self,
        scheme: &'static str,
        sid: String,
        origin: String,
        additional_parts: HashMap<&str, String>,
    ) -> Result<String> {
        let now = SystemTime::now();
        let epoch_duration = now.duration_since(UNIX_EPOCH)?;
        let time_stamp = epoch_duration.as_secs_f64().round().to_string();

        let mut hash_parts: Vec<String> = Vec::new();

        if !additional_parts.is_empty() {
            let joined = additional_parts
                .values()
                .cloned()
                .collect::<Vec<_>>()
                .join(":");
            hash_parts.push(joined);
        }

        hash_parts.extend_from_slice(&[time_stamp.clone(), sid.to_string(), origin.to_string()]);
        let joined = hash_parts.join(" ");

        let mut hasher = Sha1::new();
        hasher.update(joined.as_bytes());
        let sid_hash = format!("{:x}", hasher.finalize());

        let mut parts: Vec<String> = vec![time_stamp, sid_hash];

        if !additional_parts.is_empty() {
            let joined = additional_parts
                .values()
                .cloned()
                .collect::<Vec<_>>()
                .join("");

            parts.push(joined);
        }

        let sid_auth = format!("{} {}", scheme, parts.join("_"));
        Ok(sid_auth)
    }

    fn get_sid_authorization_header(
        &self,
        origin: Option<String>,
        user_session_id: Option<String>,
    ) -> Result<Option<String>> {
        let mut authorizations: Vec<String> = Vec::new();
        let mut additional_parts: HashMap<&str, String> = HashMap::new();

        let sid_cookies = self.get_sid_cookies()?;

        if let Some(user_sess_id) = user_session_id {
            additional_parts.insert("u", user_sess_id);
        }

        for (scheme, sid_opt) in [
            ("SAPISIDHASH", sid_cookies.yt_sapisid),
            ("SAPISID1PHASH", sid_cookies.yt_1psapisid),
            ("SAPISID3PHASH", sid_cookies.yt_3psapisid),
        ] {
            if let Some(sid) = sid_opt {
                let auth = self.make_sid_authorization(
                    scheme,
                    sid,
                    origin.as_deref().unwrap_or(YT_URL).to_string(),
                    additional_parts.clone(),
                )?;
                authorizations.push(auth);
            }
        }

        if authorizations.is_empty() {
            return Ok(None);
        }

        Ok(Some(authorizations.join(" ")))
    }
}
