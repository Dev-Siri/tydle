use std::collections::HashMap;

use anyhow::Result;
use reqwest::Url;
use serde_json::Value;

use crate::extractor::{
    auth::ExtractorAuthHandle,
    client::INNERTUBE_CLIENTS,
    extract::YtExtractor,
    yt_interface::{DEFAULT_YT_CLIENT, YtClient, YtEndpoint},
    ytcfg::ExtractorYtCfgHandle,
};

pub trait ExtractorApiHandle {
    fn generate_api_headers(
        &self,
        ytcfg: HashMap<String, Value>,
        delegated_session_id: Option<String>,
        user_session_id: Option<String>,
        session_index: Option<i32>,
        default_client: Option<&YtClient>,
    ) -> Result<HashMap<&str, String>>;
    async fn call_api(
        &self,
        endpoint: YtEndpoint,
        query: HashMap<&str, &str>,
        headers: Option<HashMap<&str, String>>,
        context: Option<HashMap<String, Value>>,
        api_key: Option<String>,
        default_client: Option<&YtClient>,
    ) -> Result<HashMap<String, Value>>;
}

impl ExtractorApiHandle for YtExtractor {
    fn generate_api_headers(
        &self,
        ytcfg: HashMap<String, Value>,
        delegated_session_id: Option<String>,
        user_session_id: Option<String>,
        session_index: Option<i32>,
        default_client: Option<&YtClient>,
    ) -> Result<HashMap<&str, String>> {
        let client = default_client.unwrap_or(&DEFAULT_YT_CLIENT);
        let innertube_client = INNERTUBE_CLIENTS.get(client).unwrap();
        let host_name = self.select_api_hostname(Some(client));

        let origin = format!("https://{}", host_name);
        let mut headers = HashMap::new();

        headers.insert(
            "X-YouTube-Client-Name",
            innertube_client.innertube_context_client_name.to_string(),
        );

        headers.insert(
            "X-YouTube-Client-Version",
            self.select_client_version(Some(client)).to_string(),
        );

        headers.insert("Origin", origin.clone());

        if let Some(visitor_id) = self.select_visitor_data(&ytcfg) {
            headers.insert("X-Goog-Visitor-Id", visitor_id);
        }

        let innertube_client_context = innertube_client.innertube_context.get("client").unwrap();

        if let Some(user_agent) = innertube_client_context.get("userAgent") {
            headers.insert("User-Agent", user_agent.to_string());
        }

        let cookie_headers = self.generate_cookie_auth_headers(
            ytcfg,
            delegated_session_id,
            user_session_id,
            session_index,
            origin,
        )?;

        for (k, v) in cookie_headers {
            headers.insert(k, v);
        }

        Ok(headers)
    }

    async fn call_api(
        &self,
        endpoint: YtEndpoint,
        query: HashMap<&str, &str>,
        headers: Option<HashMap<&str, String>>,
        context: Option<HashMap<String, Value>>,
        api_key: Option<String>,
        default_client: Option<&YtClient>,
    ) -> Result<HashMap<String, Value>> {
        let client = default_client.unwrap_or(&DEFAULT_YT_CLIENT);

        let host_name = self.select_api_hostname(Some(client));
        let ep = endpoint.as_str();
        let api_url = format!("https://{}/youtubei/v1/{}", host_name, ep);
        let yt_url = Url::parse(api_url.as_str())?;

        let http_client = reqwest::Client::new();
        let real_headers =
            self.generate_api_headers(Default::default(), None, None, None, Some(client))?;
        let mut data = match context {
            Some(ctx) => ctx,
            None => self.select_context(None, Some(client))?,
        };

        for (k, v) in query {
            data.insert(k.into(), v.into());
        }

        let mut request_builder = http_client
            .post(yt_url)
            .json(&data)
            .query(&[("prettyPrint", "false")]);

        if let Some(available_api_key) = api_key {
            request_builder = request_builder.query(&[("key", available_api_key)]);
        }

        for (k, v) in real_headers {
            request_builder = request_builder.header(k, v);
        }

        if let Some(provided_headers) = headers {
            for (key, value) in provided_headers {
                request_builder = request_builder.header(key, value);
            }
        }

        request_builder = request_builder.header("Content-Type", "application/json");

        let response = request_builder.send().await?;
        Ok(response.json().await?)
    }
}
