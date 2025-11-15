use anyhow::Result;
use maplit::hashmap;
use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::{
    extractor::token_policy::{
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

    let web_context = hashmap! {
        "client" => hashmap! {
            "clientName" => "WEB".into(),
            "clientVersion" => "2.20250925.01.00".into(),
            "hl" => PREFERRED_LOCALE.into()
        }
    };

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

    let web_safari_context = hashmap! {
        "client" => hashmap! {
            "clientName" => "WEB".into(),
            "clientVersion" => "2.20250925.01.00".into(),
            "userAgent" => "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.5 Safari/605.1.15,gzip(gfe)".into(),
            "hl" => PREFERRED_LOCALE.into()
        },
    };

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

    let web_embedded_context = hashmap! {
        "client" => hashmap! {
            "clientName" => "WEB_EMBEDDED_PLAYER".into(),
            "clientVersion" => "1.20250923.21.00".into(),
            "hl" => PREFERRED_LOCALE.into()
        }
    };

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

    let web_music_context = hashmap! {
        "client" => hashmap! {
            "clientName" => "WEB_REMIX".into(),
            "clientVersion" => "1.20250922.03.00".into(),
            "hl" => PREFERRED_LOCALE.into(),
        }
    };

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
            player_po_token_policy: Default::default(),
            subs_po_token_policy: Default::default(),
        },
    );

    let web_creator_context = hashmap! {
        "client" => hashmap! {
            "clientName" => "WEB_CREATOR".into(),
            "clientVersion" => "1.20250922.03.00".into(),
            "hl" => PREFERRED_LOCALE.into(),
        }
    };

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
            player_po_token_policy: Default::default(),
            subs_po_token_policy: Default::default(),
        },
    );

    let android_context = hashmap! {
        "client" => hashmap! {
            "clientName" => "ANDROID".into(),
            "clientVersion" => "20.10.38".into(),
            "androidSdkVersion" => 30.into(),
            "userAgent" => "com.google.android.youtube/20.10.38 (Linux; U; Android 11) gzip".into(),
            "osName" => "Android".into(),
            "osVersion" => "11".into(),
            "hl" => PREFERRED_LOCALE.into(),
        }
    };

    let android_gvs_po_token_policy = hashmap! {
        StreamingProtocol::Https => GvsPoTokenPolicy {
            required: true,
            recommended: true,
            not_required_for_premium: false,
            not_required_with_player_token: true,
        },
        StreamingProtocol::Dash => GvsPoTokenPolicy {
            required: true,
            recommended: true,
            not_required_for_premium: false,
            not_required_with_player_token: true,
        },
        StreamingProtocol::Hls =>  GvsPoTokenPolicy {
            required: false,
            recommended: true,
            not_required_for_premium: false,
            not_required_with_player_token: true,
        }
    };

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
            subs_po_token_policy: Default::default(),
        },
    );

    let android_sdkless_context = hashmap! {
        "client" => hashmap!  {
            "clientName" => "ANDROID".into(),
            "clientVersion" => "20.10.38".into(),
            "userAgent" => "com.google.android.youtube/20.10.38 (Linux; U; Android 11) gzip".into(),
            "osName" =>"Android".into(),
            "osVersion" => "11".into(),
            "hl" => PREFERRED_LOCALE.into(),
        }
    };

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
            player_po_token_policy: Default::default(),
            subs_po_token_policy: Default::default(),
        },
    );

    let android_vr_context = hashmap! {
        "client" => hashmap! {
            "clientName" => "ANDROID_VR".into(),
            "clientVersion" => "1.65.10".into(),
            "deviceMake" => "Oculus".into(),
            "deviceModel" => "Quest 3".into(),
            "androidSdkVersion" => 32.into(),
            "userAgent" => "com.google.android.apps.youtube.vr.oculus/1.65.10 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip".into(),
            "osName" => "Android".into(),
            "osVersion" => "12L".into(),
            "hl" => PREFERRED_LOCALE.into(),
        }
    };

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
            player_po_token_policy: Default::default(),
            subs_po_token_policy: Default::default(),
        },
    );

    let ios_context = hashmap! {
        "client" => hashmap! {
            "clientName" => "IOS".into(),
            "clientVersion" => "20.10.4".into(),
            "deviceMake" => "Apple".into(),
            "deviceModel" => "iPhone16,2".into(),
            "userAgent" => "com.google.ios.youtube/20.10.4 (iPhone16,2; U; CPU iOS 18_3_2 like Mac OS X;)".into(),
            "osName" => "iPhone".into(),
            "osVersion" => "18.3.2.22D82".into(),
            "hl" => PREFERRED_LOCALE.into(),
        }
    };
    let ios_gvs_po_token_policy = hashmap! {
        StreamingProtocol::Https =>GvsPoTokenPolicy {
            required: true,
            recommended: true,
            not_required_for_premium: false,
            not_required_with_player_token: true,
        },
        // HLS Livestreams require POT 30 seconds in.
        StreamingProtocol::Hls => GvsPoTokenPolicy {
            required: true,
            recommended: true,
            not_required_for_premium: false,
            not_required_with_player_token: true,
        }
    };

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
            subs_po_token_policy: Default::default(),
        },
    );

    let mweb_context = hashmap! {
        "client" => hashmap! {
            "clientName" => "MWEB".into(),
            "clientVersion" => "2.20250925.01.00".into(),
            "userAgent" => "Mozilla/5.0 (iPad; CPU OS 16_7_10 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Mobile/15E148 Safari/604.1,gzip(gfe)".into(),
            "hl" => PREFERRED_LOCALE.into(),
        }
    };

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
            player_po_token_policy: Default::default(),
            subs_po_token_policy: Default::default(),
        },
    );

    let tv_context = hashmap! {
        "client" => hashmap! {
            "clientName" => "TVHTML5".into(),
            "clientVersion" => "7.20250923.13.00".into(),
            "userAgent" => "Mozilla/5.0 (ChromiumStylePlatform) Cobalt/Version".into(),
            "hl" => PREFERRED_LOCALE.into(),
        }
    };

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
            player_po_token_policy: Default::default(),
            subs_po_token_policy: Default::default(),
        },
    );

    let tv_simply_context = hashmap! {
        "client" =>  hashmap! {
            "clientName" => "TVHTML5_SIMPLY".into(),
            "clientVersion" => "1.0".into(),
            "hl" => PREFERRED_LOCALE.into(),
        }
    };
    let tv_simply_gvs_po_token_policy = hashmap! {
        StreamingProtocol::Https => GvsPoTokenPolicy {
            required: true,
            recommended: true,
            not_required_for_premium: false,
            not_required_with_player_token: false,
        },
        StreamingProtocol::Dash => GvsPoTokenPolicy {
            required: true,
            recommended: true,
            not_required_for_premium: false,
            not_required_with_player_token: false,
        },
        StreamingProtocol::Hls => GvsPoTokenPolicy {
            required: false,
            recommended: true,
            not_required_for_premium: false,
            not_required_with_player_token: false,
        }
    };

    m.insert(
        YtClient::TvSimply,
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
            player_po_token_policy: Default::default(),
            subs_po_token_policy: Default::default(),
        },
    );

    let tv_embedded_context = hashmap! {
        "client" => hashmap! {
            "clientName" => "TVHTML5_SIMPLY_EMBEDDED_PLAYER".into(),
            "clientVersion" => "2.0".into(),
            "hl" => PREFERRED_LOCALE.into(),
        }
    };

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
            player_po_token_policy: Default::default(),
            subs_po_token_policy: Default::default(),
        },
    );

    let third_party: HashMap<&str, Value> = hashmap! {
        // Can be any valid URL.
        "embedUrl" => "https://www.youtube.com/".into(),
    };

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
