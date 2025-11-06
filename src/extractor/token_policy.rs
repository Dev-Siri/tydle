use std::{collections::HashMap, hash::Hash};

use once_cell::sync::Lazy;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize)]
pub enum StreamingProtocol {
    Https,
    Dash,
    Hls,
}

impl StreamingProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Https => "https",
            Self::Dash => "dash",
            Self::Hls => "hls",
        }
    }
}

pub enum PoTokenContext {
    Player,
    Gvs,
    Subs,
}

impl PoTokenContext {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Gvs => "gvs",
            Self::Subs => "subs",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct GvsPoTokenPolicy {
    pub required: bool,
    /// Try to fetch a PO Token even if it is not required.
    pub recommended: bool,
    pub not_required_for_premium: bool,
    pub not_required_with_player_token: bool,
}

impl GvsPoTokenPolicy {
    pub fn default() -> Self {
        Self {
            required: false,
            recommended: false,
            not_required_for_premium: false,
            not_required_with_player_token: false,
        }
    }
}

pub fn create_default_gvs_po_token_policy() -> HashMap<StreamingProtocol, GvsPoTokenPolicy> {
    let mut gvs_po_token_policy = HashMap::new();

    for streaming_protocol in [
        StreamingProtocol::Https,
        StreamingProtocol::Hls,
        StreamingProtocol::Dash,
    ] {
        gvs_po_token_policy.insert(streaming_protocol, GvsPoTokenPolicy::default());
    }

    gvs_po_token_policy
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct PlayerPoTokenPolicy {
    pub required: bool,
    pub recommended: bool,
    pub not_required_for_premium: bool,
}

impl PlayerPoTokenPolicy {
    pub fn default() -> Self {
        Self {
            required: false,
            recommended: false,
            not_required_for_premium: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct SubsPoTokenPolicy {
    pub required: bool,
    pub recommended: bool,
    pub not_required_for_premium: bool,
}

impl SubsPoTokenPolicy {
    pub fn default() -> Self {
        Self {
            required: false,
            recommended: false,
            not_required_for_premium: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WebPoTokenPolicy {
    pub gvs_po_token_policy: HashMap<StreamingProtocol, GvsPoTokenPolicy>,
    pub player_po_token_policy: PlayerPoTokenPolicy,
    pub subs_po_token_policy: SubsPoTokenPolicy,
}

impl WebPoTokenPolicy {
    pub fn new(
        gvs_po_token_policy: HashMap<StreamingProtocol, GvsPoTokenPolicy>,
        player_po_token_policy: PlayerPoTokenPolicy,
        subs_po_token_policy: SubsPoTokenPolicy,
    ) -> Self {
        Self {
            gvs_po_token_policy,
            player_po_token_policy,
            subs_po_token_policy,
        }
    }
}

pub static WEB_PO_TOKEN_POLICIES: Lazy<WebPoTokenPolicy> = Lazy::new(|| {
    let mut web_gvs_po_token_policy = HashMap::new();

    web_gvs_po_token_policy.insert(
        StreamingProtocol::Https,
        GvsPoTokenPolicy {
            required: true,
            recommended: true,
            not_required_for_premium: true,
            not_required_with_player_token: false,
        },
    );

    web_gvs_po_token_policy.insert(
        StreamingProtocol::Dash,
        GvsPoTokenPolicy {
            required: true,
            recommended: true,
            not_required_for_premium: true,
            not_required_with_player_token: false,
        },
    );

    web_gvs_po_token_policy.insert(
        StreamingProtocol::Hls,
        GvsPoTokenPolicy {
            required: false,
            recommended: true,
            not_required_for_premium: false,
            not_required_with_player_token: false,
        },
    );

    WebPoTokenPolicy::new(
        web_gvs_po_token_policy,
        PlayerPoTokenPolicy {
            required: false,
            not_required_for_premium: false,
            recommended: false,
        },
        SubsPoTokenPolicy {
            required: false,
            not_required_for_premium: false,
            recommended: false,
        },
    )
});
