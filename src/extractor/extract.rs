use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, atomic::AtomicBool},
};

use anyhow::{Result, anyhow, bail};
use fancy_regex::Regex;
use maplit::hashmap;
use serde_json::{Map, Value};

use crate::{
    TydleOptions,
    cache::{CacheAccess, PlayerCacheHandle},
    cookies::CookieJar,
    extractor::{
        auth::ExtractorAuthHandle, client::INNERTUBE_CLIENTS, download::ExtractorDownloadHandle,
        json::ExtractorJsonHandle, player::ExtractorPlayerHandle, ytcfg::ExtractorYtCfgHandle,
    },
    utils::{file_size_from_tbr, mime_type_to_ext, parse_codecs},
    yt_interface::{
        AudioTrackInfo, Codec, Ext, STREAMING_DATA_CLIENT_NAME, VideoId, YT_SUB_DOMAIN, YtAgeLimit,
        YtChannel, YtClient, YtManifest, YtMediaType, YtStream, YtStreamResponse, YtStreamSource,
        YtThumbnail, YtVideoInfo,
    },
};

pub struct YtExtractor<P, C>
where
    P: CacheAccess<(String, String)>,
    C: CacheAccess,
{
    pub passed_auth_cookies: AtomicBool,
    pub http_client: reqwest::Client,
    pub cookie_jar: CookieJar,
    pub player_cache: Arc<P>,
    pub code_cache: Arc<C>,
    pub tydle_options: TydleOptions,
}

pub trait InfoExtractor {
    fn http_scheme(&self) -> &str;
    async fn extract_video_info_from_manifest(&self, manifest: &YtManifest) -> Result<YtVideoInfo>;
    fn extract_metadata(
        &self,
        player_responses: Vec<HashMap<String, Value>>,
    ) -> Result<YtVideoInfo>;
    async fn extract_video_info(&self, video_id: &VideoId) -> Result<YtVideoInfo>;
    async fn extract_streams_from_manifest(
        &self,
        manifest: &YtManifest,
    ) -> Result<YtStreamResponse>;
    async fn extract_manifest(&self, video_id: &VideoId) -> Result<YtManifest>;
    fn extract_formats(
        &self,
        player_responses: Vec<HashMap<String, Value>>,
    ) -> Result<Vec<YtStream>>;
    async fn extract_streams(&self, video_id: &VideoId) -> Result<YtStreamResponse>;
    fn generate_checkok_params(&self) -> HashMap<String, Value>;
    fn is_premium_subscriber(&self, initial_data: &HashMap<String, Value>) -> Result<bool>;
    fn extract_ytcfg(&self, webpage_content: String) -> Result<HashMap<String, Value>>;
    fn extract_yt_initial_data(&self, webpage_content: &String) -> Result<HashMap<String, Value>>;
    fn get_clients(&self, is_premium_subscriber: bool) -> Result<Vec<YtClient>>;
    async fn extract(
        &self,
        webpage_url: &str,
        webpage_client: &YtClient,
        video_id: &VideoId,
    ) -> Result<(Vec<HashMap<String, Value>>, String)>;
}

impl<P, C> YtExtractor<P, C>
where
    P: CacheAccess<(String, String)> + PlayerCacheHandle,
    C: CacheAccess,
{
    pub fn new(
        player_cache: Arc<P>,
        code_cache: Arc<C>,
        tydle_options: TydleOptions,
    ) -> Result<Self> {
        let cookie_jar = CookieJar::new_with_cookies(tydle_options.auth_cookies.clone());

        let extractor = Self {
            passed_auth_cookies: AtomicBool::new(false),
            http_client: reqwest::Client::new(),
            cookie_jar,
            player_cache,
            code_cache,
            tydle_options, // x_forwarded_for_ip: None,
        };

        extractor.initialize_pref()?;
        extractor.initialize_consent()?;
        extractor.initialize_cookie_auth()?;

        Ok(extractor)
    }
}

impl<P, C> InfoExtractor for YtExtractor<P, C>
where
    P: CacheAccess<(String, String)> + PlayerCacheHandle + Send + Sync,
    C: CacheAccess + Send + Sync,
{
    fn generate_checkok_params(&self) -> HashMap<String, Value> {
        let checkout_params_map = hashmap! {
            "contentCheckOk".into() => true.into(),
            "racyCheckOk".into() => true.into(),
        };

        checkout_params_map
    }

    fn is_premium_subscriber(&self, initial_data: &HashMap<String, Value>) -> Result<bool> {
        if !self.is_authenticated()? || initial_data.is_empty() {
            return Ok(false);
        }

        let tlr = initial_data
            .get("topbar")
            .and_then(|v| v.get("desktopTopbarRenderer"))
            .and_then(|v| v.get("logo"))
            .and_then(|v| v.get("topbarLogoRenderer"));
        let logo_match = tlr
            .and_then(|v| v.get("iconImage"))
            .and_then(|v| v.get("iconType"))
            .unwrap_or(&Value::Null);
        let logo_match_str = logo_match.as_str().unwrap_or_default();

        Ok(logo_match_str == "YOUTUBE_PREMIUM_LOGO"
            || self
                .get_text(
                    tlr.unwrap_or_default(),
                    Some(vec![vec!["tooltipText"]]),
                    None,
                )
                .unwrap_or_default()
                .to_lowercase()
                .contains("premium"))
    }

    fn extract_ytcfg(&self, webpage_content: String) -> Result<HashMap<String, Value>> {
        if webpage_content.is_empty() {
            return Ok(HashMap::new());
        }

        let search_re = Regex::new(r"ytcfg\.set\s*\(\s*({.+?})\s*\)\s*;")?;
        let json_str = search_re
            .captures(&webpage_content)?
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str())
            .unwrap_or("{}");

        let ytcfg: HashMap<String, Value> = serde_json::from_str(json_str)?;

        Ok(ytcfg)
    }

    fn extract_yt_initial_data(&self, webpage_content: &String) -> Result<HashMap<String, Value>> {
        let re = Regex::new(
            r#"(?:window\s*\[\s*["']ytInitialData["']\s*\]|ytInitialData)\s*=\s*(\{.*?\})\s*(?:;|</script>)"#,
        )?;
        let json_str = re
            .captures(&webpage_content)?
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str())
            .ok_or_else(|| anyhow!("ytInitialData not found"))?;

        let json_val: HashMap<String, Value> = serde_json::from_str(json_str)?;
        Ok(json_val)
    }

    fn get_clients(&self, is_premium_subscriber: bool) -> Result<Vec<YtClient>> {
        if self.tydle_options.force_default_client {
            return Ok(vec![self.tydle_options.default_client]);
        }

        let mut clients = if is_premium_subscriber {
            // Premium does not require POT. (except for subtitles)
            vec![YtClient::TvDowngraded, YtClient::WebCreator]
        } else if self.is_authenticated()? {
            vec![YtClient::TvDowngraded, YtClient::WebSafari]
        } else {
            vec![YtClient::AndroidVr, YtClient::WebSafari]
        };

        if self.is_authenticated()? {
            let mut unsupported_clients = Vec::new();

            for client in &clients {
                if !INNERTUBE_CLIENTS.get(&client).unwrap().supports_cookies {
                    unsupported_clients.push(*client);
                }
            }

            for _client in &unsupported_clients {
                #[cfg(feature = "logging")]
                log::warn!(
                    "Skipping client \"{}\" since it does not support cookies.",
                    _client.as_str()
                );

                clients.retain(|c| !unsupported_clients.iter().any(|u| u.as_str() == c.as_str()));
            }
        }

        let mut seen = HashSet::new();
        let unique_clients: Vec<_> = clients.into_iter().filter(|c| seen.insert(*c)).collect();

        Ok(unique_clients)
    }

    fn extract_formats(
        &self,
        player_responses: Vec<HashMap<String, Value>>,
    ) -> Result<Vec<YtStream>> {
        let mut streams: Vec<YtStream> = vec![];

        for player_response in &player_responses {
            let streaming_formats = player_response.get("streamingData").unwrap_or_default();

            if streaming_formats.is_null() {
                continue;
            }

            let client_name = player_response
                .get(STREAMING_DATA_CLIENT_NAME)
                .and_then(|c| c.as_str())
                .unwrap_or("UNKNOWN");

            let mut all_formats = Vec::new();

            if let Some(streaming_data) = player_response.get("streamingData") {
                if let Some(formats) = streaming_data.get("formats").and_then(|v| v.as_array()) {
                    all_formats.extend(formats.clone());
                }
                if let Some(adaptive_formats) = streaming_data
                    .get("adaptiveFormats")
                    .and_then(|v| v.as_array())
                {
                    all_formats.extend(adaptive_formats.clone());
                }
            }

            for fmt in all_formats {
                let target_duration_sec = fmt.get("targetDurationSec");

                // Skip livestream.
                if target_duration_sec.is_some() {
                    #[cfg(feature = "logging")]
                    log::info!(
                        "Skipped a format. Found livestream because livestreams are not supported."
                    );
                    continue;
                }

                let audio_track = fmt
                    .get("audioTrack")
                    .unwrap_or_default()
                    .as_object()
                    .cloned()
                    .unwrap_or(Map::new());

                let itag = fmt
                    .get("itag")
                    .unwrap_or_default()
                    .as_u64()
                    .unwrap_or_default();

                let mut quality = fmt
                    .get("quality")
                    .and_then(|s| Some(s.as_str().unwrap_or_default().to_string().to_lowercase()));

                if quality.is_none() || quality.clone().is_some_and(|q| q == "tiny") {
                    let audio_quality = fmt
                        .get("audioQuality")
                        .unwrap_or_default()
                        .as_str()
                        .unwrap_or_default()
                        .to_string()
                        .to_lowercase();
                    quality = Some(audio_quality);
                }

                // The 3gp format (17) in android client has a quality of "small", but is actually worse than other formats.
                if itag == 17 {
                    quality = Some("tiny".to_string());
                }

                let has_drm = fmt.get("drmFamilies").is_some();

                #[cfg(feature = "logging")]
                if has_drm {
                    let mut warn_msg = format!(
                        "Some {} client https formats have been skipped as they are DRM protected.",
                        client_name
                    );

                    if client_name == "tv" {
                        warn_msg += format!(
                            "{} may have an experiment that applies DRM to all videos on the `tv` client.\nSee  https://github.com/yt-dlp/yt-dlp/issues/12563  for more details.",
                            if self.is_authenticated()? {
                                "Your account"
                            } else {
                                "The current session"
                            }
                        ).as_str();
                    }

                    log::warn!("{warn_msg}");
                }

                let mut stream_source = None;

                if let Some(fmt_url) = fmt.get("url").clone() {
                    stream_source = Some(YtStreamSource::URL(
                        fmt_url.as_str().unwrap_or_default().to_string(),
                    ));
                }

                if let Some(sc) = fmt.get("signatureCipher").unwrap_or_default().as_str() {
                    stream_source = Some(YtStreamSource::Signature(sc.to_string()));
                }

                let Some(source) = stream_source else {
                    continue;
                };

                let format_duration = fmt
                    .get("approxDurationMs")
                    .and_then(|d| d.as_str())
                    .and_then(|ds| Some(ds.parse::<f64>().unwrap_or_default()))
                    .unwrap_or_default();

                let tbr = fmt
                    .get("averageBitrate")
                    .or_else(|| fmt.get("bitrate"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1000 as f64);

                let name = fmt
                    .get("qualityLabel")
                    .and_then(|ql| ql.as_str())
                    .and_then(|qls| Some(qls.to_string()))
                    .unwrap_or(quality.unwrap_or_default().replace("audio_quality_", ""));

                let audio_display = audio_track
                    .get("displayName")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let is_default = audio_track
                    .get("audioIsDefault")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let projection = fmt
                    .get("projectionType")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_lowercase());

                let spatial_audio = fmt
                    .get("spatialAudioType")
                    .and_then(|v| v.as_str())
                    .map(|s| s.replace("SPATIAL_AUDIO_TYPE_", "").to_lowercase());

                let re = Regex::new(r#"((?:[^/]+)/(?:[^;]+))(?:;\s*codecs="([^"]+)")?"#)?;

                let (ext, (vcodec, acodec)) = match re.captures(
                    fmt.get("mimeType")
                        .unwrap_or_default()
                        .as_str()
                        .unwrap_or_default(),
                )? {
                    Some(mime_mobj_captures) => {
                        let mime_type = mime_mobj_captures
                            .get(1)
                            .and_then(|mt| Some(mt.as_str()))
                            .unwrap_or_default();
                        let codec = mime_mobj_captures
                            .get(2)
                            .and_then(|mt| Some(mt.as_str()))
                            .unwrap_or_default();

                        (mime_type_to_ext(mime_type), parse_codecs(codec)?)
                    }
                    None => (Ext::Unknown, (None, None)),
                };

                let fps = fmt
                    .get("fps")
                    .unwrap_or_default()
                    .as_u64()
                    .unwrap_or_default() as u16;

                streams.push(YtStream {
                    asr: fmt
                        .get("audioSampleRate")
                        .and_then(|v| v.as_str())
                        .and_then(|a| a.parse().ok()),
                    file_size: fmt
                        .get("contentLength")
                        .and_then(|v| v.as_str().and_then(|s| s.parse().ok())),
                    file_size_approx: file_size_from_tbr(tbr, format_duration),
                    height: fmt.get("height").and_then(|h| h.as_u64()),
                    width: fmt.get("width").and_then(|w| w.as_u64()),
                    format_duration,
                    has_drm,
                    itag: itag as u16,
                    source,
                    // Format 22 is likely to be damaged. See https://github.com/yt-dlp/yt-dlp/issues/3372
                    source_preference: match itag == 22 {
                        true => -5,
                        false => -1,
                    } + match name.contains("Premium") {
                        true => 100,
                        false => 0,
                    },
                    tbr,
                    fps,
                    quality_label: name,
                    audio_track: AudioTrackInfo {
                        display_name: audio_display,
                        is_default,
                    },
                    projection,
                    spatial_audio,
                    client: YtClient::from_str(client_name),
                    is_drc: fmt
                        .get("isDrc")
                        .and_then(|dr| dr.as_bool())
                        .unwrap_or_default(),
                    ext,
                    is_dash: acodec.as_ref().is_some_and(|ac| ac == "none")
                        || vcodec.as_ref().is_some_and(|vc| vc == "none"),
                    codec: Codec { vcodec, acodec },
                });
            }
        }

        Ok(streams)
    }

    fn extract_metadata(
        &self,
        player_responses: Vec<HashMap<String, Value>>,
    ) -> Result<YtVideoInfo> {
        let mut extracted_title: Option<String> = None;
        let mut extracted_length_seconds: Option<u64> = None;
        let mut extracted_channel_id: Option<String> = None;
        let mut extracted_channel_name: Option<String> = None;
        let mut extracted_keywords: Option<Vec<String>> = None;
        let mut extracted_media_type: Option<YtMediaType> = None;
        let mut extracted_view_count: Option<u64> = None;
        let mut extracted_thumbnails: Vec<YtThumbnail> = vec![];
        let mut extracted_description: Option<String> = None;
        let mut extracted_age_limit: Option<YtAgeLimit> = None;

        for player_response in player_responses {
            let Some(vd_value) = player_response.get("videoDetails") else {
                bail!(
                    "Could not extract video info (metadata) because YouTube didn't return a `videoDetails` value in response."
                )
            };

            let video_details = vd_value.as_object().cloned().unwrap_or(Map::new());
            let microformats = player_response
                .get("microformat")
                .and_then(|mf| mf.get("playerMicroformatRenderer"))
                .unwrap_or_default()
                .as_object()
                .cloned()
                .unwrap_or(Map::new());

            if extracted_title.is_none() {
                extracted_title = video_details
                    .get("title")
                    .and_then(|s| s.as_str())
                    .and_then(|s| Some(s.to_string()))
                    .clone();
            }

            if extracted_length_seconds.is_none() {
                extracted_length_seconds = video_details
                    .get("lengthSeconds")
                    .and_then(|s| s.as_str())
                    .and_then(|s| s.parse().ok())
                    .clone();
            }

            if extracted_view_count.is_none() {
                extracted_view_count = video_details
                    .get("viewCount")
                    .and_then(|s| s.as_str())
                    .and_then(|s| s.parse().ok())
                    .clone();
            }

            if extracted_channel_id.is_none() {
                extracted_channel_id = video_details
                    .get("channelId")
                    .and_then(|s| s.as_str())
                    .and_then(|s| s.parse().ok())
                    .clone();
            }

            if extracted_keywords.is_none() {
                extracted_keywords = video_details
                    .get("keywords")
                    .and_then(|s| s.as_array())
                    .and_then(|v| {
                        Some(
                            v.iter()
                                .map(|k| k.as_str().unwrap_or_default().to_string())
                                .collect(),
                        )
                    })
                    .clone();
            }

            if extracted_channel_name.is_none() {
                extracted_channel_name = video_details
                    .get("author")
                    .and_then(|s| s.as_str().and_then(|s| Some(s.to_string())))
                    .clone();
            }

            if extracted_media_type.is_none() {
                extracted_media_type = Some(
                    if video_details
                        .get("isLiveContent")
                        .and_then(|isc| isc.as_bool())
                        .unwrap_or_default()
                    {
                        YtMediaType::LiveStream
                    } else if microformats
                        .get("isShortsEligible")
                        .and_then(|ise| ise.as_bool())
                        .unwrap_or_default()
                    {
                        YtMediaType::Short
                    } else {
                        YtMediaType::Video
                    },
                );
            }

            if extracted_thumbnails.is_empty() {
                extracted_thumbnails = video_details
                    .get("thumbnail")
                    .and_then(|t| t.get("thumbnails"))
                    .and_then(|t| t.as_array())
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|t| {
                        t.get("url")
                            .and_then(|v| v.as_str())
                            .map(|url| YtThumbnail {
                                url: url.to_string(),
                                height: t.get("height").and_then(|h| h.as_u64()),
                                width: t.get("width").and_then(|w| w.as_u64()),
                            })
                    })
                    .collect();
            }

            if extracted_description.is_none() {
                extracted_description = video_details
                    .get("shortDescription")
                    .and_then(|s| s.as_str())
                    .and_then(|s| Some(s.to_string()))
                    .clone();
            }

            if extracted_age_limit.is_none() {
                extracted_age_limit = Some(
                    match microformats
                        .get("isFamilySafe")
                        .unwrap_or_default()
                        .as_bool()
                        .unwrap_or_default()
                    {
                        true => YtAgeLimit::Adult,
                        false => YtAgeLimit::None,
                    },
                )
            }
        }

        if let (
            Some(title),
            Some(description),
            Some(length_seconds),
            Some(view_count),
            Some(channel_id),
        ) = (
            extracted_title,
            extracted_description,
            extracted_length_seconds,
            extracted_view_count,
            extracted_channel_id,
        ) {
            return Ok(YtVideoInfo {
                title,
                description,
                duration: length_seconds,
                view_count,
                channel: YtChannel::new(channel_id, extracted_channel_name)?,
                keywords: extracted_keywords.unwrap_or_default(),
                thumbnails: extracted_thumbnails,
                age_limit: extracted_age_limit.unwrap_or_default(),
                media_type: extracted_media_type.unwrap_or_default(),
            });
        }

        bail!(
            "Extracting video info (metadata) failed because not all required keys were returned by YouTube."
        )
    }

    async fn extract(
        &self,
        webpage_url: &str,
        webpage_client: &YtClient,
        video_id: &VideoId,
    ) -> Result<(Vec<HashMap<String, Value>>, String)> {
        let webpage = self
            .download_webpage(webpage_url, webpage_client, video_id)
            .await?;

        let mut webpage_ytcfg = self.extract_ytcfg(webpage.clone())?;

        if webpage_ytcfg.is_empty() {
            webpage_ytcfg = self
                .select_default_ytcfg(Some(webpage_client))?
                .to_json_val_hashmap()?;
        }

        let initial_data = self
            .download_initial_data(video_id, &webpage, webpage_client, &webpage_ytcfg)
            .await?;

        let is_premium_subscriber = self.is_premium_subscriber(&initial_data)?;
        let clients = self.get_clients(is_premium_subscriber)?;
        let player_responses = self
            .extract_player_responses(&clients, video_id, &webpage, webpage_client, &webpage_ytcfg)
            .await?;

        Ok(player_responses)
    }

    fn http_scheme(&self) -> &str {
        match self.tydle_options.prefer_insecure {
            true => "http",
            false => "https",
        }
    }

    async fn extract_manifest(&self, video_id: &VideoId) -> Result<YtManifest> {
        let request_address = if self.tydle_options.proxy_address.is_empty() {
            YT_SUB_DOMAIN
        } else {
            &self.tydle_options.proxy_address
        };

        let webpage_url = format!("{}://{}/watch", self.http_scheme(), request_address);
        let (initial_extracted_data, player_url) =
            self.extract(&webpage_url, &YtClient::Web, video_id).await?;

        Ok(YtManifest::new(initial_extracted_data, player_url))
    }

    async fn extract_streams(&self, video_id: &VideoId) -> Result<YtStreamResponse> {
        let yt_manifest = self.extract_manifest(video_id).await?;

        let formats = self.extract_formats(yt_manifest.extracted_manifest)?;
        let stream_response = YtStreamResponse::new(yt_manifest.player_url, formats);

        Ok(stream_response)
    }

    async fn extract_streams_from_manifest(
        &self,
        manifest: &YtManifest,
    ) -> Result<YtStreamResponse> {
        let formats = self.extract_formats(manifest.extracted_manifest.clone())?;
        Ok(YtStreamResponse::new(manifest.player_url.clone(), formats))
    }

    async fn extract_video_info(&self, video_id: &VideoId) -> Result<YtVideoInfo> {
        let yt_manifest = self.extract_manifest(video_id).await?;

        let yt_video_info = self.extract_metadata(yt_manifest.extracted_manifest)?;
        Ok(yt_video_info)
    }

    async fn extract_video_info_from_manifest(&self, manifest: &YtManifest) -> Result<YtVideoInfo> {
        let yt_video_info = self.extract_metadata(manifest.extracted_manifest.clone())?;
        Ok(yt_video_info)
    }
}
