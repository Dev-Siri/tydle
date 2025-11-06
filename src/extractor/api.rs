use std::collections::HashMap;

use anyhow::Result;
use reqwest::Url;
use serde_json::{Value, json};

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
        visitor_id: Option<String>,
        default_client: Option<&YtClient>,
    ) -> Result<HashMap<&str, String>>;
    async fn call_api(
        &self,
        endpoint: YtEndpoint,
        query: HashMap<String, Value>,
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
        visitor_id: Option<String>,
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

        if let Some(available_visitor_id) = visitor_id {
            headers.insert("X-Goog-Visitor-Id", available_visitor_id);
        } else if let Some(selected_visitor_id) = self.select_visitor_data(&[&ytcfg]) {
            headers.insert("X-Goog-Visitor-Id", selected_visitor_id);
        }

        let innertube_client_context = innertube_client.innertube_context.get("client").unwrap();

        if let Some(user_agent) = innertube_client_context.get("userAgent") {
            headers.insert("User-Agent", user_agent.as_str().unwrap_or_default().into());
        }

        let cookie_headers = self.generate_cookie_auth_headers(
            ytcfg,
            delegated_session_id,
            user_session_id,
            session_index,
            origin,
        )?;

        headers.extend(cookie_headers);

        Ok(headers)
    }

    async fn call_api(
        &self,
        endpoint: YtEndpoint,
        query: HashMap<String, Value>,
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
        let mut real_headers =
            self.generate_api_headers(Default::default(), None, None, None, None, Some(client))?;
        let mut data: HashMap<String, Value> = HashMap::new();

        if let Some(ctx) = context {
            data.insert(
                "context".into(),
                json!({
                    "client": ctx
                }),
            );
        } else {
            data.insert(
                "context".into(),
                json!({
                    "client": self.select_context(None, Some(client))?,
                }),
            );
        };

        data.extend(query);

        if let Some(availabe_headers) = headers {
            real_headers.extend(availabe_headers);
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

        request_builder = request_builder.header("Content-Type", "application/json");

        let response = request_builder.send().await?;
        Ok(response.json().await?)
    }
}
