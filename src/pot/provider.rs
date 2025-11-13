use std::collections::HashMap;

use crate::cookies::CookieJar;

pub struct PoTokenRequest<'a> {
    // YouTube parameters
    // context: PoTokenContext,
    // innertube_context: InnertubeContext,
    innertube_host: Option<&'a str>,
    session_index: Option<&'a str>,
    player_url: Option<&'a str>,
    is_authenticated: bool,
    video_webpage: Option<&'a str>,
    internal_client_name: Option<&'a str>,
    // Content binding parameters
    visitor_data: Option<&'a str>,
    data_sync_id: Option<&'a str>,
    video_id: Option<&'a str>,
    /// Internal, YouTube experiment on whether to bind GVS PO Token to video_id.
    _gvs_bind_to_video_id: bool,
    // Networking parameters
    request_cookiejar: CookieJar,
    request_proxy: Option<&'a str>,
    request_headers: HashMap<&'a str, String>,
    request_timeout: Option<f32>,
    request_source_address: Option<&'a str>,
    request_verify_tls: bool,
    /// Generate a new token, do not used a cached token
    /// The token should still be cached for future requests
    bypass_cache: bool,
}
