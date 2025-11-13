use std::{collections::HashMap, sync::atomic::Ordering};

use anyhow::{Result, anyhow};
use serde_json::Value;

use crate::{
    cookies::CookieStore,
    extractor::{cookies::ExtractorCookieHandle, extract::YtExtractor, json::ExtractorJsonHandle},
    utils::{convert_to_query_string, parse_query_string},
    yt_interface::{PREFERRED_LOCALE, YT_URL},
};

pub trait ExtractorAuthHandle {
    fn initialize_cookie_auth(&self) -> Result<()>;
    fn initialize_consent(&self) -> Result<()>;
    fn initialize_pref(&self) -> Result<()>;
    fn is_authenticated(&self) -> Result<bool>;
    fn has_auth_cookies(&self) -> Result<bool>;
    /// Extract current delegated session ID required to download private playlists of secondary channels.
    fn get_delegated_session_id(&self, ytcfg: &[&HashMap<String, Value>]) -> Option<String>;
    /// Extract current account dataSyncId in the format DELEGATED_SESSION_ID||USER_SESSION_ID or USER_SESSION_ID||
    fn get_data_sync_id(&self, ytcfgs: &[&HashMap<String, Value>]) -> Option<String>;
    /// Extract current user session ID.
    fn get_user_session_id(&self, ytcfgs: &[&HashMap<String, Value>]) -> Option<String>;
    /// Parse `data_sync_id` into `delegated_session_id` and `user_session_id`.
    /// `data_sync_id` is of the form `"delegated_session_id||user_session_id"` for secondary channel
    /// and just `"user_session_id||"` for primary channel.
    fn parse_data_sync_id(&self, data_sync_id: String) -> (Option<String>, Option<String>);
    /// Index of current account in account list.
    fn get_session_index(&self, data: &[&HashMap<String, Value>]) -> Option<i32>;
    fn generate_cookie_auth_headers(
        &self,
        ytcfg: HashMap<String, Value>,
        delegated_session_id: Option<String>,
        user_session_id: Option<String>,
        session_index: Option<i32>,
        origin: String,
    ) -> Result<HashMap<&str, String>>;
}

/// Handles Auth with cookies and user-set preferences.
impl ExtractorAuthHandle for YtExtractor {
    fn initialize_cookie_auth(&self) -> Result<()> {
        self.passed_auth_cookies.store(false, Ordering::Relaxed);
        if self.has_auth_cookies()? {
            self.passed_auth_cookies.store(true, Ordering::Relaxed);
            log::info!("Found YouTube account cookies.");
        }

        Ok(())
    }

    fn initialize_consent(&self) -> Result<()> {
        if self.has_auth_cookies()? {
            return Ok(());
        }

        let yt_cookies = self.get_youtube_cookies()?;

        if let Some(socs) = yt_cookies.get("SOCS") {
            if !socs.starts_with("CAA") {
                return Ok(());
            }
        }

        self.cookie_jar.set(YT_URL, "SOCS", "CAI")?;
        Ok(())
    }

    fn initialize_pref(&self) -> Result<()> {
        let youtube_cookies = self.get_youtube_cookies()?;
        let mut pref: HashMap<String, String> = HashMap::new();

        if let Some(raw_pref) = youtube_cookies.get("PREF") {
            match parse_query_string(&raw_pref) {
                Some(parsed_qs) => pref = parsed_qs,
                None => return Err(anyhow!("Failed to parse user PREF cookie.")),
            }
        }

        pref.insert("hl".into(), PREFERRED_LOCALE.into());
        pref.insert("tz".into(), "UTC".into());

        let pref_qs = convert_to_query_string(&pref);

        self.cookie_jar.set(YT_URL, "PREF", pref_qs.as_str())?;
        Ok(())
    }

    fn is_authenticated(&self) -> Result<bool> {
        self.has_auth_cookies()
    }

    fn has_auth_cookies(&self) -> Result<bool> {
        let sid_cookies = self.get_sid_cookies()?;
        let youtube_cookies = self.get_youtube_cookies()?;

        Ok(youtube_cookies.contains_key("LOGIN_INFO")
            && (sid_cookies.yt_sapisid.is_some()
                || sid_cookies.yt_1psapisid.is_some()
                || sid_cookies.yt_3psapisid.is_some()))
    }

    fn get_delegated_session_id(&self, ytcfgs: &[&HashMap<String, Value>]) -> Option<String> {
        for ytcfg in ytcfgs {
            for (_, v) in *ytcfg {
                if let Some(val) = self.find_key(v, "DELEGATED_SESSION_ID") {
                    return Some(val);
                }
            }
        }

        match self.get_data_sync_id(ytcfgs) {
            Some(data_sync_id) => self.parse_data_sync_id(data_sync_id).0,
            None => None,
        }
    }

    fn parse_data_sync_id(&self, data_sync_id: String) -> (Option<String>, Option<String>) {
        let mut parts = data_sync_id.splitn(2, "||");
        let first = parts.next().map(|s| s.to_string());
        let second = parts.next().map(|s| s.to_string());

        match (&first, &second) {
            (Some(_), Some(second_val)) if !second_val.is_empty() => (first, second),
            (Some(first_val), _) => (None, Some(first_val.clone())),
            _ => (None, None),
        }
    }

    fn get_data_sync_id(&self, ytcfgs: &[&HashMap<String, Value>]) -> Option<String> {
        for ytcfg in ytcfgs {
            for (_, v) in *ytcfg {
                if let Some(val) = self.find_key(v, "DATASYNC_ID") {
                    return Some(val);
                }
            }

            for (_, v) in *ytcfg {
                if let Some(val) = v
                    .get("responseContext")
                    .and_then(|rc| rc.get("mainAppWebResponseContext"))
                    .and_then(|m| m.get("datasyncId"))
                    .and_then(|id| id.as_str())
                {
                    return Some(val.to_string());
                }
            }
        }

        None
    }

    fn get_user_session_id(&self, ytcfgs: &[&HashMap<String, Value>]) -> Option<String> {
        for ytcfg in ytcfgs {
            for (_, v) in *ytcfg {
                if let Some(val) = self.find_key(v, "USER_SESSION_ID") {
                    return Some(val);
                }
            }
        }

        match self.get_data_sync_id(ytcfgs) {
            Some(data_sync_id) => self.parse_data_sync_id(data_sync_id).1,
            None => None,
        }
    }

    fn get_session_index(&self, data: &[&HashMap<String, Value>]) -> Option<i32> {
        for yt_cfg in data {
            if let Some(session_index) = yt_cfg.get("SESSION_INDEX") {
                return Some(
                    session_index
                        .as_str()
                        .unwrap_or_default()
                        .parse()
                        .unwrap_or_default(),
                );
            }
        }

        None
    }

    fn generate_cookie_auth_headers(
        &self,
        ytcfg: HashMap<String, Value>,
        delegated_session_id: Option<String>,
        user_session_id: Option<String>,
        session_index: Option<i32>,
        origin: String,
    ) -> Result<HashMap<&str, String>> {
        let mut headers = HashMap::new();

        let delegated_sess_id = if delegated_session_id.is_none() {
            self.get_delegated_session_id(&[&ytcfg])
        } else {
            None
        };

        if let Some(delegated_s_id) = delegated_sess_id.clone() {
            headers.insert("X-Goog-PageId", delegated_s_id);
        }

        let sess_index = match session_index {
            Some(s_id) => Some(s_id),
            None => self.get_session_index(&[&ytcfg]),
        };

        if delegated_sess_id.is_some() || sess_index.is_some() {
            headers.insert(
                "X-Goog-AuthUser",
                sess_index.unwrap_or_default().to_string(),
            );
        }

        let user_sess_id = match user_session_id {
            Some(user_s_id) => Some(user_s_id),
            None => self.get_user_session_id(&[&ytcfg]),
        };

        if let Some(auth) = self.get_sid_authorization_header(Some(origin.clone()), user_sess_id)? {
            headers.insert("Authorization", auth);
            headers.insert("X-Origin", origin);
        }

        if let Some(logged_in) = ytcfg.get("LOGGED_IN") {
            if logged_in.as_bool().unwrap_or_default() {
                headers.insert("X-Youtube-Bootstrap-Logged-In", "true".into());
            }
        }

        Ok(headers)
    }
}
