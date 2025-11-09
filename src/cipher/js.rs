use std::collections::HashMap;

use anyhow::{Result, bail};
use deno_core::JsRuntime;
use serde_json::json;

use crate::{
    cache::CacheAccess,
    cipher::decipher::{SignatureDecipher, SignatureType},
};

pub trait SignatureJsHandle {
    async fn get_js_modules(&self) -> Result<(String, String)>;
    async fn parse_signature_js(
        &self,
        code: String,
        example_sig: String,
        signature_type: SignatureType,
    ) -> Result<String>;
}

impl SignatureJsHandle for SignatureDecipher {
    async fn get_js_modules(&self) -> Result<(String, String)> {
        const YT_DLP_YT_SOLVER_PKG_LIB_URL: &str =
            "https://github.com/yt-dlp/ejs/releases/download/0.3.1/yt.solver.lib.min.js";
        const YT_DLP_YT_SOLVER_PKG_CORE_URL: &str =
            "https://github.com/yt-dlp/ejs/releases/download/0.3.1/yt.solver.core.min.js";

        let lib_code = match self.code_cache.get(&YT_DLP_YT_SOLVER_PKG_LIB_URL.into())? {
            Some(cached_lib_code) => cached_lib_code,
            None => {
                let fetched_lib = reqwest::get(YT_DLP_YT_SOLVER_PKG_LIB_URL)
                    .await?
                    .text()
                    .await?;

                self.code_cache
                    .add(YT_DLP_YT_SOLVER_PKG_LIB_URL.into(), fetched_lib.clone())?;

                fetched_lib
            }
        };

        let core_code = match self.code_cache.get(&YT_DLP_YT_SOLVER_PKG_CORE_URL.into())? {
            Some(cached_lib_code) => cached_lib_code,
            None => {
                let fetched_lib = reqwest::get(YT_DLP_YT_SOLVER_PKG_CORE_URL)
                    .await?
                    .text()
                    .await?;

                self.code_cache
                    .add(YT_DLP_YT_SOLVER_PKG_CORE_URL.into(), fetched_lib.clone())?;

                fetched_lib
            }
        };

        Ok((lib_code, core_code))
    }

    // Taken from `youtube_explode_dart`'s implementation with `yt-dlp`'s ejs cipher library.
    // See: https://github.com/Hexer10/youtube_explode_dart/blob/a993b3d463713b0aabd945f07a7e6a1635bcf1e7/lib/src/reverse_engineering/challenges/ejs/ejs.dart
    async fn parse_signature_js(
        &self,
        code: String,
        example_sig: String,
        signature_type: SignatureType,
    ) -> Result<String> {
        let (lib_code, core_code) = self.get_js_modules().await?;

        let js_env = format!(
            "{}\nObject.assign(globalThis, lib);\n{}",
            lib_code, core_code
        );

        let mut deno = JsRuntime::new(Default::default());

        deno.execute_script("<setup_environment>", js_env)?;

        let input = json!({
            "type": "player",
            "player": code,
            "requests": [{"type": signature_type.as_str(), "challenges": [example_sig]}],
            "output_preprocessed": true
        });

        let set_input_js = format!("globalThis.__input = {};", input.to_string());
        deno.execute_script("<set_input>", set_input_js)?;

        let js_call = r#"(function() {
            var res = jsc(globalThis.__input);
            return JSON.stringify(res);
        })();"#;
        let global_value = deno.execute_script("<parse_sig>", js_call)?;

        deno.run_event_loop(Default::default()).await?;

        let local_value = global_value.open(deno.v8_isolate());

        let mut scope = deno.handle_scope();
        let result_str = local_value.to_rust_string_lossy(&mut scope);

        let result: HashMap<String, serde_json::Value> = serde_json::from_str(&result_str)?;
        let Some(deciphered_sig) = result
            .get("responses")
            .and_then(|r| r.get(0))
            .and_then(|r| r.get("data"))
            .and_then(|r| r.get(example_sig))
            .and_then(|v| v.as_str())
        else {
            bail!("Signature deciphering failed because ytcore returned an invalid response.")
        };

        Ok(deciphered_sig.into())
    }
}
