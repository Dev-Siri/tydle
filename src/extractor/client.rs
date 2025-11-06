use anyhow::Result;
use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::extractor::{
    token_policy::{
        GvsPoTokenPolicy, PlayerPoTokenPolicy, StreamingProtocol, SubsPoTokenPolicy,
        WEB_PO_TOKEN_POLICIES, create_default_gvs_po_token_policy,
    },
    yt_interface::{PREFERRED_LOCALE, YtClient},
};

#[derive(Debug, Clone, Serialize)]
pub struct InnerTubeClient {
    #[serde(rename = "INNERTUBE_CONTEXT")]
    pub innertube_context: HashMap<&'static str, HashMap<&'static str, Value>>,
    #[serde(rename = "INNERTUBE_HOST")]
    pub innertube_host: &'static str,
    #[serde(rename = "INNERTUBE_CONTEXT_CLIENT_NAME")]
    pub innertube_context_client_name: i32,
    #[serde(rename = "SUPPORTS_COOKIES")]
    pub supports_cookies: bool,
    #[serde(rename = "REQUIRE_JS_PLAYER")]
    pub require_js_player: bool,
    #[serde(rename = "REQUIRE_AUTH")]
    pub require_auth: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authenticated_user_agent: Option<&'static str>,
    #[serde(rename = "GVS_PO_TOKEN_POLICY")]
    pub gvs_po_token_policy: HashMap<StreamingProtocol, GvsPoTokenPolicy>,
    #[serde(rename = "PLAYER_PO_TOKEN_POLICY")]
    pub player_po_token_policy: PlayerPoTokenPolicy,
    #[serde(rename = "SUBS_PO_TOKEN_POLICY")]
    pub subs_po_token_policy: SubsPoTokenPolicy,
    #[serde(skip_serializing)]
    pub priority: isize,
}

impl InnerTubeClient {
    pub fn to_json_val_hashmap(&self) -> Result<HashMap<String, Value>> {
        let serialized = serde_json::to_value(self)?;

        if let Value::Object(obj) = serialized {
            let mut hashmap = HashMap::new();
            for (k, v) in obj {
                hashmap.insert(k, v);
            }

            return Ok(hashmap);
        }

        Ok(HashMap::new())
    }
}

pub static INNERTUBE_CLIENTS: Lazy<HashMap<YtClient, InnerTubeClient>> = Lazy::new(|| {
    const DEFAULT_INNERTUBE_HOST: &str = "www.youtube.com";
    const BASE_CLIENTS: &[&str; 5] = &["android", "mweb", "tv", "web", "ios"];
    let base_client_indices: HashMap<&str, usize> = BASE_CLIENTS
        .iter()
        .enumerate()
        .map(|(i, &name)| (name, i))
        .collect();

    let mut m = HashMap::new();

    let mut web_context = HashMap::new();
    let mut web_context_client: HashMap<&str, Value> = HashMap::new();

    web_context_client.insert("clientName", "WEB".into());
    web_context_client.insert("clientVersion", "2.20250925.01.00".into());
    web_context_client.insert("hl", PREFERRED_LOCALE.into());

    web_context.insert("client", web_context_client);
    m.insert(
        YtClient::Web,
        InnerTubeClient {
            priority: 0,
            innertube_context: web_context,
            innertube_host: DEFAULT_INNERTUBE_HOST,
            innertube_context_client_name: 1,
            supports_cookies: true,
            require_js_player: true,
            require_auth: false,
            authenticated_user_agent: None,
            gvs_po_token_policy: WEB_PO_TOKEN_POLICIES.gvs_po_token_policy.clone(),
            player_po_token_policy: WEB_PO_TOKEN_POLICIES.player_po_token_policy,
            subs_po_token_policy: WEB_PO_TOKEN_POLICIES.subs_po_token_policy,
        },
    );

    let mut web_safari_context = HashMap::new();
    let mut web_safari_context_client: HashMap<&str, Value> = HashMap::new();

    web_safari_context_client.insert("clientName", "WEB".into());
    web_safari_context_client.insert("clientVersion", "2.20250925.01.00".into());
    web_safari_context_client.insert("userAgent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.5 Safari/605.1.15,gzip(gfe)".into());
    web_safari_context_client.insert("hl", PREFERRED_LOCALE.into());

    web_safari_context.insert("client", web_safari_context_client);
    m.insert(
        YtClient::WebSafari,
        InnerTubeClient {
            priority: 0,
            innertube_context: web_safari_context,
            innertube_host: DEFAULT_INNERTUBE_HOST,
            innertube_context_client_name: 1,
            supports_cookies: true,
            require_js_player: true,
            require_auth: false,
            authenticated_user_agent: None,
            gvs_po_token_policy: WEB_PO_TOKEN_POLICIES.gvs_po_token_policy.clone(),
            player_po_token_policy: WEB_PO_TOKEN_POLICIES.player_po_token_policy,
            subs_po_token_policy: WEB_PO_TOKEN_POLICIES.subs_po_token_policy,
        },
    );

    let mut web_embedded_context = HashMap::new();
    let mut web_embedded_context_client: HashMap<&str, Value> = HashMap::new();

    web_embedded_context_client.insert("clientName", "WEB_EMBEDDED_PLAYER".into());
    web_embedded_context_client.insert("clientVersion", "1.20250923.21.00".into());
    web_embedded_context_client.insert("hl", PREFERRED_LOCALE.into());

    web_embedded_context.insert("client", web_embedded_context_client);
    m.insert(
        YtClient::WebEmbedded,
        InnerTubeClient {
            priority: 0,
            innertube_context: web_embedded_context,
            innertube_host: DEFAULT_INNERTUBE_HOST,
            innertube_context_client_name: 56,
            supports_cookies: true,
            require_js_player: true,
            require_auth: false,
            authenticated_user_agent: None,
            gvs_po_token_policy: WEB_PO_TOKEN_POLICIES.gvs_po_token_policy.clone(),
            player_po_token_policy: WEB_PO_TOKEN_POLICIES.player_po_token_policy,
            subs_po_token_policy: WEB_PO_TOKEN_POLICIES.subs_po_token_policy,
        },
    );

    let mut web_music_context = HashMap::new();
    let mut web_music_context_client: HashMap<&str, Value> = HashMap::new();

    web_music_context_client.insert("clientName", "WEB_REMIX".into());
    web_music_context_client.insert("clientVersion", "1.20250922.03.00".into());
    web_music_context_client.insert("hl", PREFERRED_LOCALE.into());

    web_music_context.insert("client", web_music_context_client);
    m.insert(
        YtClient::WebMusic,
        InnerTubeClient {
            priority: 0,
            innertube_context: web_music_context,
            innertube_host: "music.youtube.com",
            innertube_context_client_name: 67,
            supports_cookies: true,
            require_js_player: true,
            require_auth: false,
            authenticated_user_agent: None,
            gvs_po_token_policy: WEB_PO_TOKEN_POLICIES.gvs_po_token_policy.clone(),
            player_po_token_policy: PlayerPoTokenPolicy::default(),
            subs_po_token_policy: SubsPoTokenPolicy::default(),
        },
    );

    let mut web_creator_context = HashMap::new();
    let mut web_creator_context_client: HashMap<&str, Value> = HashMap::new();

    web_creator_context_client.insert("clientName", "WEB_CREATOR".into());
    web_creator_context_client.insert("clientVersion", "1.20250922.03.00".into());
    web_creator_context_client.insert("hl", PREFERRED_LOCALE.into());

    web_creator_context.insert("client", web_creator_context_client);
    m.insert(
        YtClient::WebCreator,
        InnerTubeClient {
            priority: 0,
            innertube_context: web_creator_context,
            innertube_host: DEFAULT_INNERTUBE_HOST,
            innertube_context_client_name: 62,
            supports_cookies: true,
            require_js_player: true,
            require_auth: true,
            authenticated_user_agent: None,
            gvs_po_token_policy: WEB_PO_TOKEN_POLICIES.gvs_po_token_policy.clone(),
            player_po_token_policy: PlayerPoTokenPolicy::default(),
            subs_po_token_policy: SubsPoTokenPolicy::default(),
        },
    );

    let mut android_context = HashMap::new();
    let mut android_context_client: HashMap<&str, Value> = HashMap::new();

    android_context_client.insert("clientName", "ANDROID".into());
    android_context_client.insert("clientVersion", "20.10.38".into());
    android_context_client.insert("androidSdkVersion", 30.into());
    android_context_client.insert(
        "userAgent",
        "com.google.android.youtube/20.10.38 (Linux; U; Android 11) gzip".into(),
    );
    android_context_client.insert("osName", "Android".into());
    android_context_client.insert("osVersion", "11".into());
    android_context_client.insert("hl", PREFERRED_LOCALE.into());

    let mut android_gvs_po_token_policy = HashMap::new();
    android_gvs_po_token_policy.insert(
        StreamingProtocol::Https,
        GvsPoTokenPolicy {
            required: true,
            recommended: true,
            not_required_for_premium: false,
            not_required_with_player_token: true,
        },
    );

    android_gvs_po_token_policy.insert(
        StreamingProtocol::Dash,
        GvsPoTokenPolicy {
            required: true,
            recommended: true,
            not_required_for_premium: false,
            not_required_with_player_token: true,
        },
    );

    android_gvs_po_token_policy.insert(
        StreamingProtocol::Hls,
        GvsPoTokenPolicy {
            required: false,
            recommended: true,
            not_required_for_premium: false,
            not_required_with_player_token: true,
        },
    );

    android_context.insert("client", android_context_client);
    m.insert(
        YtClient::Android,
        InnerTubeClient {
            priority: 0,
            innertube_context: android_context,
            innertube_host: DEFAULT_INNERTUBE_HOST,
            innertube_context_client_name: 3,
            supports_cookies: false,
            require_js_player: false,
            require_auth: false,
            authenticated_user_agent: None,
            gvs_po_token_policy: android_gvs_po_token_policy,
            player_po_token_policy: PlayerPoTokenPolicy {
                required: false,
                recommended: true,
                not_required_for_premium: false,
            },
            subs_po_token_policy: SubsPoTokenPolicy::default(),
        },
    );

    let mut android_sdkless_context = HashMap::new();
    let mut android_sdkless_context_client: HashMap<&str, Value> = HashMap::new();

    android_sdkless_context_client.insert("clientName", "ANDROID".into());
    android_sdkless_context_client.insert("clientVersion", "20.10.38".into());
    android_sdkless_context_client.insert(
        "userAgent",
        "com.google.android.youtube/20.10.38 (Linux; U; Android 11) gzip".into(),
    );
    android_sdkless_context_client.insert("osName", "Android".into());
    android_sdkless_context_client.insert("osVersion", "11".into());
    android_sdkless_context_client.insert("hl", PREFERRED_LOCALE.into());

    android_sdkless_context.insert("client", android_sdkless_context_client);
    m.insert(
        YtClient::AndroidSdkless,
        InnerTubeClient {
            priority: 0,
            innertube_context: android_sdkless_context,
            innertube_host: DEFAULT_INNERTUBE_HOST,
            innertube_context_client_name: 3,
            supports_cookies: false,
            require_js_player: false,
            require_auth: false,
            authenticated_user_agent: None,
            gvs_po_token_policy: create_default_gvs_po_token_policy(),
            player_po_token_policy: PlayerPoTokenPolicy::default(),
            subs_po_token_policy: SubsPoTokenPolicy::default(),
        },
    );

    let mut android_vr_context = HashMap::new();
    let mut android_vr_context_client: HashMap<&str, Value> = HashMap::new();

    android_vr_context_client.insert("clientName", "ANDROID_VR".into());
    android_vr_context_client.insert("clientVersion", "1.65.10".into());
    android_vr_context_client.insert("deviceMake", "Oculus".into());
    android_vr_context_client.insert("deviceModel", "Quest 3".into());
    android_vr_context_client.insert("androidSdkVersion", 32.into());
    android_vr_context_client.insert(
        "userAgent",
        "com.google.android.apps.youtube.vr.oculus/1.65.10 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip".into(),
    );
    android_vr_context_client.insert("osName", "Android".into());
    android_vr_context_client.insert("osVersion", "12L".into());
    android_vr_context_client.insert("hl", PREFERRED_LOCALE.into());

    android_vr_context.insert("client", android_vr_context_client);
    m.insert(
        YtClient::AndroidVr,
        InnerTubeClient {
            priority: 0,
            innertube_context: android_vr_context,
            innertube_host: DEFAULT_INNERTUBE_HOST,
            innertube_context_client_name: 28,
            supports_cookies: false,
            require_js_player: false,
            require_auth: false,
            authenticated_user_agent: None,
            gvs_po_token_policy: create_default_gvs_po_token_policy(),
            player_po_token_policy: PlayerPoTokenPolicy::default(),
            subs_po_token_policy: SubsPoTokenPolicy::default(),
        },
    );

    let mut ios_context = HashMap::new();
    let mut ios_context_client: HashMap<&str, Value> = HashMap::new();
    let mut ios_gvs_po_token_policy = HashMap::new();

    ios_gvs_po_token_policy.insert(
        StreamingProtocol::Https,
        GvsPoTokenPolicy {
            required: true,
            recommended: true,
            not_required_for_premium: false,
            not_required_with_player_token: true,
        },
    );

    // HLS Livestreams require POT 30 seconds in.
    ios_gvs_po_token_policy.insert(
        StreamingProtocol::Hls,
        GvsPoTokenPolicy {
            required: true,
            recommended: true,
            not_required_for_premium: false,
            not_required_with_player_token: true,
        },
    );

    ios_context_client.insert("clientName", "IOS".into());
    ios_context_client.insert("clientVersion", "20.10.4".into());
    ios_context_client.insert("deviceMake", "Apple".into());
    ios_context_client.insert("deviceModel", "iPhone16,2".into());
    ios_context_client.insert(
        "userAgent",
        "com.google.ios.youtube/20.10.4 (iPhone16,2; U; CPU iOS 18_3_2 like Mac OS X;)".into(),
    );
    ios_context_client.insert("osName", "iPhone".into());
    ios_context_client.insert("osVersion", "18.3.2.22D82".into());
    ios_context_client.insert("hl", PREFERRED_LOCALE.into());

    ios_context.insert("client", ios_context_client);
    m.insert(
        YtClient::IOS,
        InnerTubeClient {
            priority: 0,
            innertube_context: ios_context,
            innertube_host: DEFAULT_INNERTUBE_HOST,
            innertube_context_client_name: 5,
            supports_cookies: false,
            require_js_player: false,
            require_auth: false,
            authenticated_user_agent: None,
            gvs_po_token_policy: ios_gvs_po_token_policy,
            player_po_token_policy: PlayerPoTokenPolicy {
                required: false,
                recommended: true,
                not_required_for_premium: false,
            },
            subs_po_token_policy: SubsPoTokenPolicy::default(),
        },
    );

    let mut mweb_context = HashMap::new();
    let mut mweb_context_client: HashMap<&str, Value> = HashMap::new();

    mweb_context_client.insert("clientName", "MWEB".into());
    mweb_context_client.insert("clientVersion", "2.20250925.01.00".into());
    mweb_context_client.insert(
        "userAgent",
        "Mozilla/5.0 (iPad; CPU OS 16_7_10 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Mobile/15E148 Safari/604.1,gzip(gfe)".into(),
    );
    mweb_context_client.insert("hl", PREFERRED_LOCALE.into());

    mweb_context.insert("client", mweb_context_client);
    m.insert(
        YtClient::MWeb,
        InnerTubeClient {
            priority: 0,
            innertube_context: mweb_context,
            innertube_host: DEFAULT_INNERTUBE_HOST,
            innertube_context_client_name: 2,
            supports_cookies: true,
            require_js_player: true,
            require_auth: false,
            authenticated_user_agent: None,
            gvs_po_token_policy: WEB_PO_TOKEN_POLICIES.gvs_po_token_policy.clone(),
            player_po_token_policy: PlayerPoTokenPolicy::default(),
            subs_po_token_policy: SubsPoTokenPolicy::default(),
        },
    );

    let mut tv_context = HashMap::new();
    let mut tv_context_client: HashMap<&str, Value> = HashMap::new();

    tv_context_client.insert("clientName", "TVHTML5".into());
    tv_context_client.insert("clientVersion", "7.20250923.13.00".into());
    tv_context_client.insert(
        "userAgent",
        "Mozilla/5.0 (ChromiumStylePlatform) Cobalt/Version".into(),
    );
    tv_context_client.insert("hl", PREFERRED_LOCALE.into());

    tv_context.insert("client", tv_context_client);
    m.insert(
        YtClient::Tv,
        InnerTubeClient {
            priority: 0,
            innertube_context: tv_context,
            innertube_host: DEFAULT_INNERTUBE_HOST,
            innertube_context_client_name: 7,
            supports_cookies: true,
            require_js_player: true,
            require_auth: false,
            authenticated_user_agent: Some("Mozilla/5.0 (ChromiumStylePlatform) Cobalt/25.lts.30.1034943-gold (unlike Gecko), Unknown_TV_Unknown_0/Unknown (Unknown, Unknown)"),
            gvs_po_token_policy: WEB_PO_TOKEN_POLICIES.gvs_po_token_policy.clone(),
            player_po_token_policy: PlayerPoTokenPolicy::default(),
            subs_po_token_policy: SubsPoTokenPolicy::default(),
        },
    );

    let mut tv_simply_context = HashMap::new();
    let mut tv_simply_context_client: HashMap<&str, Value> = HashMap::new();
    let mut tv_simply_gvs_po_token_policy = HashMap::new();

    tv_simply_gvs_po_token_policy.insert(
        StreamingProtocol::Https,
        GvsPoTokenPolicy {
            required: true,
            recommended: true,
            not_required_for_premium: false,
            not_required_with_player_token: false,
        },
    );

    tv_simply_gvs_po_token_policy.insert(
        StreamingProtocol::Dash,
        GvsPoTokenPolicy {
            required: true,
            recommended: true,
            not_required_for_premium: false,
            not_required_with_player_token: false,
        },
    );

    tv_simply_gvs_po_token_policy.insert(
        StreamingProtocol::Hls,
        GvsPoTokenPolicy {
            required: false,
            recommended: true,
            not_required_for_premium: false,
            not_required_with_player_token: false,
        },
    );

    tv_simply_context_client.insert("clientName", "TVHTML5_SIMPLY".into());
    tv_simply_context_client.insert("clientVersion", "1.0".into());
    tv_simply_context_client.insert("hl", PREFERRED_LOCALE.into());

    tv_simply_context.insert("client", tv_simply_context_client);
    m.insert(
        YtClient::Tv,
        InnerTubeClient {
            priority: 0,
            innertube_context: tv_simply_context,
            innertube_host: DEFAULT_INNERTUBE_HOST,
            innertube_context_client_name: 75,
            supports_cookies: false,
            require_js_player: true,
            require_auth: false,
            authenticated_user_agent: None,
            gvs_po_token_policy: tv_simply_gvs_po_token_policy,
            player_po_token_policy: PlayerPoTokenPolicy::default(),
            subs_po_token_policy: SubsPoTokenPolicy::default(),
        },
    );

    let mut tv_embedded_context = HashMap::new();
    let mut tv_embedded_context_client: HashMap<&str, Value> = HashMap::new();

    tv_embedded_context_client.insert("clientName", "TVHTML5_SIMPLY_EMBEDDED_PLAYER".into());
    tv_embedded_context_client.insert("clientVersion", "2.0".into());
    tv_embedded_context_client.insert("hl", PREFERRED_LOCALE.into());

    tv_embedded_context.insert("client", tv_embedded_context_client);
    m.insert(
        YtClient::TvEmbedded,
        InnerTubeClient {
            priority: 0,
            innertube_context: tv_embedded_context,
            innertube_host: DEFAULT_INNERTUBE_HOST,
            innertube_context_client_name: 85,
            supports_cookies: true,
            require_js_player: true,
            require_auth: true,
            authenticated_user_agent: None,
            gvs_po_token_policy: create_default_gvs_po_token_policy(),
            player_po_token_policy: PlayerPoTokenPolicy::default(),
            subs_po_token_policy: SubsPoTokenPolicy::default(),
        },
    );

    let mut third_party: HashMap<&str, Value> = HashMap::new();
    // Can be any valid URL.
    third_party.insert("embedUrl", "https://www.youtube.com/".into());

    for (yt_client, ytcfg) in &mut m {
        let client_base_name = yt_client.get_base();
        let priority_index = 10
            * base_client_indices
                .get(client_base_name)
                .map(|&i| i as isize)
                .unwrap_or(-1);

        if yt_client.get_variant() == "embedded" {
            ytcfg
                .innertube_context
                .insert("thirdParty", third_party.clone());
            ytcfg.priority = priority_index - 2;
        } else {
            ytcfg.priority = priority_index - 3;
        }
    }

    m
});
