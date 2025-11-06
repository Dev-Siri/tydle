use std::collections::HashMap;

use anyhow::Result;
use serde_json::{Map, Value};

use crate::extractor::{
    auth::ExtractorAuthHandle,
    client::{INNERTUBE_CLIENTS, InnerTubeClient},
    extract::YtExtractor,
    yt_interface::{DEFAULT_YT_CLIENT, PREFERRED_LOCALE, YtClient},
};

pub trait ExtractorYtCfgHandle {
    fn select_api_hostname(&self, default_client: Option<&YtClient>) -> &str;
    fn select_client_version(&self, default_client: Option<&YtClient>) -> &str;
    fn select_context(
        &self,
        ytcfg: Option<&HashMap<String, Value>>,
        default_client: Option<&YtClient>,
    ) -> Result<HashMap<String, Value>>;
    fn select_visitor_data(&self, ytcfgs: &[&HashMap<String, Value>]) -> Option<String>;
    fn select_default_ytcfg(&self, default_client: Option<&YtClient>) -> Result<InnerTubeClient>;
}

impl ExtractorYtCfgHandle for YtExtractor {
    fn select_api_hostname(&self, default_client: Option<&YtClient>) -> &str {
        let client = default_client.unwrap_or(&DEFAULT_YT_CLIENT);
        let innertube_client = INNERTUBE_CLIENTS.get(client).unwrap();
        return innertube_client.innertube_host;
    }

    fn select_client_version(&self, default_client: Option<&YtClient>) -> &str {
        let client = default_client.unwrap_or(&DEFAULT_YT_CLIENT);
        let innertube_client = INNERTUBE_CLIENTS.get(client).unwrap();

        let innertube_client_context = innertube_client.innertube_context.get("client").unwrap();
        innertube_client_context
            .get("clientVersion")
            .unwrap()
            .as_str()
            .unwrap()
    }

    fn select_context(
        &self,
        ytcfg: Option<&HashMap<String, Value>>,
        default_client: Option<&YtClient>,
    ) -> Result<HashMap<String, Value>> {
        let client = default_client.unwrap_or(&DEFAULT_YT_CLIENT);

        let innertube_client = match ytcfg {
            Some(cfg) => {
                if !cfg.is_empty() {
                    cfg
                } else {
                    &INNERTUBE_CLIENTS
                        .get(client)
                        .unwrap()
                        .to_json_val_hashmap()?
                }
            }
            None => &INNERTUBE_CLIENTS
                .get(client)
                .unwrap()
                .to_json_val_hashmap()?,
        };

        let mut client_context = innertube_client
            .get("INNERTUBE_CONTEXT")
            .and_then(|v| v.get("client"))
            .cloned()
            .unwrap_or(Value::Object(Map::new()));

        if let Some(map) = client_context.as_object_mut() {
            map.insert(
                "hl".to_string(),
                Value::String(PREFERRED_LOCALE.to_string()),
            );
            map.insert("timeZone".to_string(), Value::String("UTC".to_string()));
            map.insert("utcOffsetMinutes".to_string(), Value::Number(0.into()));
        }

        if let Value::Object(map) = client_context {
            Ok(map.into_iter().collect())
        } else {
            Ok(HashMap::new())
        }
    }

    fn select_visitor_data(&self, ytcfgs: &[&HashMap<String, Value>]) -> Option<String> {
        for ytcfg in ytcfgs {
            if let Some(v) = ytcfg.get("VISITOR_DATA").and_then(|v| v.as_str()) {
                return Some(v.to_string());
            }

            if let Some(v) = ytcfg
                .get("INNERTUBE_CONTEXT")
                .and_then(|v| v.get("client"))
                .and_then(|v| v.get("visitorData"))
                .and_then(|v| v.as_str())
            {
                return Some(v.to_string());
            }

            if let Some(v) = ytcfg
                .get("responseContext")
                .and_then(|v| v.get("visitorData"))
                .and_then(|v| v.as_str())
            {
                return Some(v.to_string());
            }
        }

        None
    }

    fn select_default_ytcfg(&self, default_client: Option<&YtClient>) -> Result<InnerTubeClient> {
        let client = default_client.unwrap_or(&DEFAULT_YT_CLIENT);
        let mut ytcfg = INNERTUBE_CLIENTS.get(client).cloned().unwrap();

        if let (Some(auth_ua), true) = (&ytcfg.authenticated_user_agent, self.is_authenticated()?) {
            let innertube_client_context = ytcfg
                .innertube_context
                .entry("client")
                .or_insert_with(HashMap::new);

            innertube_client_context.insert("userAgent", (*auth_ua).into());
        }

        Ok(ytcfg)
    }
}
