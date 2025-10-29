use once_cell::sync::Lazy;
use std::collections::HashMap;

use crate::extractor::yt_interface::YtClient;

#[derive(Debug)]
pub struct InnerTubeClient {
    pub innertube_context: HashMap<&'static str, HashMap<&'static str, &'static str>>,
    pub innertube_host: &'static str,
    pub innertube_context_client_name: i32,
    pub supports_cookies: bool,
    pub require_js_player: bool,
}

pub const CONFIGURATION_ARG_KEY: &str = "youtube";

pub static INNERTUBE_CLIENTS: Lazy<HashMap<YtClient, InnerTubeClient>> = Lazy::new(|| {
    const DEFAULT_INNERTUBE_HOST: &str = "www.youtube.com";
    const DEFAULT_LOCALE: &str = "en";

    let mut m = HashMap::new();

    let mut web_context = HashMap::new();
    let mut web_context_client = HashMap::new();

    web_context_client.insert("clientName", "WEB");
    web_context_client.insert("clientVersion", "2.20250925.01.00");
    web_context_client.insert("hl", DEFAULT_LOCALE);

    web_context.insert("client", web_context_client);
    m.insert(
        YtClient::Web,
        InnerTubeClient {
            innertube_context: web_context,
            innertube_host: DEFAULT_INNERTUBE_HOST,
            innertube_context_client_name: 1,
            supports_cookies: true,
            require_js_player: true,
        },
    );

    m
});
