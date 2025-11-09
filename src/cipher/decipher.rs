use std::sync::Arc;

use anyhow::{Result, bail};

use crate::{
    cache::{CacheAccess, CacheStore, PlayerCacheHandle},
    cipher::js::SignatureJsHandle,
    utils::{parse_query_string, replace_n_sig_query_param},
};

pub enum SignatureType {
    Nsignature,
    Signature,
}

impl SignatureType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Nsignature => "n",
            Self::Signature => "sig",
        }
    }
}

pub struct SignatureDecipher {
    pub player_cache: Arc<CacheStore<(String, String)>>,
    pub code_cache: Arc<CacheStore>,
}

impl SignatureDecipher {
    pub fn new(
        player_cache: Arc<CacheStore<(String, String)>>,
        code_cache: Arc<CacheStore>,
    ) -> Self {
        Self {
            player_cache,
            code_cache,
        }
    }
}

pub trait SignatureDecipherHandle {
    async fn extract_signature_function(
        &self,
        player_url: String,
        example_sig: String,
        signature_type: SignatureType,
    ) -> Result<String>;
    async fn decrypt_signature(
        &self,
        signature_type: SignatureType,
        encrypted_signature: String,
        player_url: String,
    ) -> Result<String>;
    async fn decipher(&self, signature: String, player_url: String) -> Result<String>;
}

impl SignatureDecipherHandle for SignatureDecipher {
    async fn extract_signature_function(
        &self,
        player_url: String,
        example_sig: String,
        signature_type: SignatureType,
    ) -> Result<String> {
        let player_js_code_key = self.player_cache.player_js_cache_key(&player_url)?;

        if let Some(code) = self.code_cache.get(&player_js_code_key)? {
            let res = self
                .parse_signature_js(code, example_sig, signature_type)
                .await?;
            return Ok(res);
        }

        bail!(
            "The player.js was not downloaded before, deciphering failed because the code was not found."
        )
    }

    async fn decrypt_signature(
        &self,
        signature_type: SignatureType,
        encrypted_signature: String,
        player_url: String,
    ) -> Result<String> {
        let cache_id = (
            format!("{}-{}", signature_type.as_str(), player_url),
            encrypted_signature.clone(),
        );

        if let Some(cached_deciphered_value) = self.player_cache.get(&cache_id)? {
            return Ok(cached_deciphered_value);
        }

        let extracted_signature = self
            .extract_signature_function(player_url, encrypted_signature, signature_type)
            .await?;
        Ok(extracted_signature)
    }

    async fn decipher(&self, signature: String, player_url: String) -> Result<String> {
        let sc = parse_query_string(&signature).unwrap_or_default();

        let (Some(fmt_url), Some(encrypted_sig)) = (sc.get("url").cloned(), sc.get("s").cloned())
        else {
            bail!("The provided signature cannot be deciphered because it is missing `url`.")
        };

        let decrypted_signature = self
            .decrypt_signature(SignatureType::Signature, encrypted_sig, player_url.clone())
            .await?;
        let url_with_sig = format!(
            "{}&{}={}",
            fmt_url,
            sc.get("sp").map(String::as_str).unwrap_or("signature"),
            decrypted_signature,
        );

        Ok(
            match parse_query_string(&url_with_sig)
                .unwrap_or_default()
                .get("n")
            {
                Some(nsig) => replace_n_sig_query_param(
                    &url_with_sig,
                    self.decrypt_signature(SignatureType::Nsignature, nsig.clone(), player_url)
                        .await?,
                )?,
                None => url_with_sig,
            },
        )
    }
}
