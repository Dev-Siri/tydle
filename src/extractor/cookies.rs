use std::collections::HashMap;

use anyhow::Result;
use sha1::{Digest, Sha1};

use crate::{
    cache::CacheAccess,
    cookies::{CookieStore, DomainCookies},
    extractor::extract::YtExtractor,
    utils::unix_timestamp_secs,
    yt_interface::{YT_DOMAIN, YT_URL},
};

#[derive(Debug)]
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
    fn get_cookies(&self, url: &str) -> Result<DomainCookies>;
    fn get_youtube_cookies(&self) -> Result<DomainCookies>;
    /// Get SAPISID, 1PSAPISID, 3PSAPISID cookie values.
    fn get_sid_cookies(&self) -> Result<SidCookies>;
    fn make_sid_authorization(
        &self,
        scheme: &str,
        sid: String,
        origin: String,
        additional_parts: HashMap<&str, String>,
    ) -> Result<String>;
    /// Generate API Session ID Authorization for Innertube requests. Assumes all requests are secure. (HTTPS)
    fn get_sid_authorization_header(
        &self,
        origin: Option<&String>,
        user_session_id: Option<String>,
    ) -> Result<Option<String>>;
}

impl<P, C> ExtractorCookieHandle for YtExtractor<P, C>
where
    P: CacheAccess<(String, String)> + Send + Sync,
    C: CacheAccess,
{
    fn get_cookies(&self, url: &str) -> Result<DomainCookies> {
        let cookies = self.cookie_jar.get_all(url)?;
        Ok(cookies)
    }

    fn get_youtube_cookies(&self) -> Result<DomainCookies> {
        self.get_cookies(YT_DOMAIN)
    }

    fn get_sid_cookies(&self) -> Result<SidCookies> {
        let yt_cookies = self.get_youtube_cookies()?;
        let yt_sapisid = yt_cookies
            .get("SAPISID")
            .and_then(|c| Some(c.value.clone()));
        let yt_3papisid = yt_cookies
            .get("__Secure-3PAPISID")
            .and_then(|c| Some(c.value.clone()));
        let yt_1papisid = yt_cookies
            .get("__Secure-1PAPISID")
            .and_then(|c| Some(c.value.clone()));
        let sid_cookies = SidCookies::new(
            yt_sapisid.or_else(|| yt_3papisid.clone()),
            yt_1papisid,
            yt_3papisid,
        );

        Ok(sid_cookies)
    }

    fn make_sid_authorization(
        &self,
        scheme: &str,
        sid: String,
        origin: String,
        additional_parts: HashMap<&str, String>,
    ) -> Result<String> {
        let time_stamp = unix_timestamp_secs().round().to_string();

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
        origin: Option<&String>,
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
                    origin.cloned().unwrap_or(YT_URL.to_string()),
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
