#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use anyhow::{Result, anyhow};
#[cfg(not(target_arch = "wasm32"))]
use anyhow::{Result, bail};
#[cfg(not(target_arch = "wasm32"))]
use deno_core::JsRuntime;
#[cfg(target_arch = "wasm32")]
use js_sys::{Function, eval};
#[cfg(not(target_arch = "wasm32"))]
use serde_json::json;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::{
    cache::{CacheAccess, PlayerCacheHandle},
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

impl<P, C> SignatureJsHandle for SignatureDecipher<P, C>
where
    P: CacheAccess<(String, String)> + PlayerCacheHandle,
    C: CacheAccess,
{
    async fn get_js_modules(&self) -> Result<(String, String)> {
        const YT_DLP_YT_SOLVER_PKG_LIB_URL: &str =
            "https://github.com/yt-dlp/ejs/releases/download/0.8.0/yt.solver.lib.min.js";
        const YT_DLP_YT_SOLVER_PKG_CORE_URL: &str =
            "https://github.com/yt-dlp/ejs/releases/download/0.8.0/yt.solver.core.min.js";

        let lib_code = match self.code_cache.get(&YT_DLP_YT_SOLVER_PKG_LIB_URL.into())? {
            Some(cached_lib_code) => cached_lib_code,
            None => {
                #[cfg(feature = "logging")]
                log::info!("Cache empty, downloading yt-dlp's EJS lib module.");
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
                #[cfg(feature = "logging")]
                log::info!("Cache empty, downloading yt-dlp's EJS core module.");
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
    #[cfg(not(target_arch = "wasm32"))]
    async fn parse_signature_js(
        &self,
        code: String,
        example_sig: String,
        signature_type: SignatureType,
    ) -> Result<String> {
        #[cfg(feature = "logging")]
        log::info!("Executing player.js JavaScript with Deno to decipher signature.");
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

    #[cfg(target_arch = "wasm32")]
    async fn parse_signature_js(
        &self,
        code: String,
        example_sig: String,
        signature_type: SignatureType,
    ) -> Result<String> {
        use js_sys::{Array, Object};
        use wasm_bindgen::JsValue;

        let (lib_code, core_code) = self.get_js_modules().await?;

        let js_env = format!(
            "{}\nObject.assign(globalThis, lib);\n{}\nglobalThis.jsc = jsc;",
            lib_code, core_code,
        );

        eval(&js_env).map_err(|err| anyhow!("JS eval failed: {:?}", err))?;

        let func = eval("jsc")
            .map_err(|_| anyhow!("jsc not defined"))?
            .dyn_into::<Function>()
            .map_err(|_| anyhow!("Failed to defined `jsc` in the JS context."))?;

        let obj = Object::new();
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("type"),
            &JsValue::from_str("player"),
        )
        .map_err(|e| anyhow!("{:?}", e))?;
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("player"),
            &JsValue::from_str(&code),
        )
        .map_err(|e| anyhow!("{:?}", e))?;

        let request = Object::new();
        js_sys::Reflect::set(
            &request,
            &JsValue::from_str("type"),
            &JsValue::from_str(signature_type.as_str()),
        )
        .map_err(|e| anyhow!("{:?}", e))?;
        js_sys::Reflect::set(
            &request,
            &JsValue::from_str("challenges"),
            &Array::of1(&JsValue::from_str(&example_sig)),
        )
        .map_err(|e| anyhow!("{:?}", e))?;

        js_sys::Reflect::set(&obj, &JsValue::from_str("requests"), &Array::of1(&request))
            .map_err(|e| anyhow!("{:?}", e))?;
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("output_preprocessed"),
            &JsValue::from_bool(true),
        )
        .map_err(|e| anyhow!("{:?}", e))?;

        let result_val = func
            .call1(&JsValue::NULL, &obj)
            .map_err(|e| anyhow!("jsc() call failed: {:?}", e))?;

        let result: serde_json::Value =
            serde_wasm_bindgen::from_value(result_val).map_err(|_| {
                anyhow!("Signature deciphering failed because the JS bridge returned an error.")
            })?;
        let deciphered = result["responses"][0]["data"][&example_sig]
            .as_str()
            .unwrap_or_default()
            .to_string();

        Ok(deciphered)
    }
}
