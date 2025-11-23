use anyhow::Result;
use fancy_regex::Regex;
use maplit::hashmap;
use std::collections::HashMap;

use crate::yt_interface::Ext;
#[cfg(feature = "cipher")]
use url::Url;
use url::form_urlencoded;

pub fn parse_query_string(qs: &str) -> Option<HashMap<String, String>> {
    std::panic::catch_unwind(|| form_urlencoded::parse(qs.as_bytes()).into_owned().collect()).ok()
}

pub fn convert_to_query_string(map: &HashMap<String, String>) -> String {
    let mut serializer = form_urlencoded::Serializer::new(String::new());
    for (key, value) in map {
        serializer.append_pair(key, value);
    }

    serializer.finish()
}

#[cfg(feature = "cipher")]
pub fn replace_n_sig_query_param(
    url_with_sig: &str,
    deciphered_n: String,
) -> Result<String, url::ParseError> {
    let mut url = Url::parse(url_with_sig)?;

    let mut query_pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();

    if let Some(_) = query_pairs.remove("n") {
        query_pairs.insert("n".to_string(), deciphered_n);
    }
    url.query_pairs_mut().clear().extend_pairs(query_pairs);

    Ok(url.to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn unix_timestamp_secs() -> f64 {
    js_sys::Date::now() / 1000.0
}

#[cfg(not(target_arch = "wasm32"))]
pub fn unix_timestamp_secs() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now();
    let epoch = now.duration_since(UNIX_EPOCH).unwrap();
    epoch.as_secs_f64()
}

/// Returns the file size in bytes.
pub fn file_size_from_tbr(tbr: f64, duration: f64) -> f64 {
    duration * tbr * (1000 / 8) as f64
}

pub fn mime_type_to_ext(mime_type: &str) -> Ext {
    let mime_type_map = hashmap! {
        // Video
        "3gpp" => Ext::ThreeGp,
        "mp2t" => Ext::Ts,
        "mp4" => Ext::Mp4,
        "mpeg" => Ext::Mpeg,
        "mpegurl" => Ext::M3u8,
        "quicktime" => Ext::Mov,
        "webm" => Ext::Webm,
        "vp9" => Ext::Vp9,
        "video/ogg" => Ext::Ogv,
        "x-flv" => Ext::Flv,
        "x-m4v" => Ext::M4v,
        "x-matroska" => Ext::Mkv,
        "x-mng" => Ext::Mng,
        "x-mp4-fragmented" => Ext::Mp4,
        "x-ms-asf" => Ext::Asf,
        "x-ms-wmv" => Ext::Wmv,
        "x-msvideo" => Ext::Avi,
        "vnd.dlna.mpeg-tts" => Ext::Mpeg,
        // Application (streaming playlists)
        "dash+xml" => Ext::Mpd,
        "f4m+xml" => Ext::F4m,
        "hds+xml" => Ext::F4m,
        "vnd.apple.mpegurl" => Ext::M3u8,
        "vnd.ms-sstr+xml" => Ext::Ism,
        "x-mpegurl" => Ext::M3u8,
        // audio
        "audio/mp4" => Ext::M4a,
        // Per RFC 3003, audio/mpeg can be .mp1, .mp2 or .mp3.
        // Using .mp3 as it"s the most popular one
        "audio/mpeg" => Ext::Mp3,
        "audio/webm" => Ext::Webm,
        "audio/x-matroska" => Ext::Mka,
        "audio/x-mpegurl" => Ext::M3u,
        "aacp" => Ext::Aac,
        "flac" => Ext::Flac,
        "midi" => Ext::Mid,
        "ogg" => Ext::Ogg,
        "wav" => Ext::Wav,
        "wave" => Ext::Wav,
        "x-aac" => Ext::Aac,
        "x-flac" => Ext::Flac,
        "x-m4a" => Ext::M4a,
        "x-realaudio" => Ext::Ra,
        "x-wav" => Ext::Wav,
        // image
        "avif" => Ext::Avif,
        "bmp" => Ext::Bmp,
        "gif" => Ext::Gif,
        "jpeg" => Ext::Jpg,
        "png" => Ext::Png,
        "svg+xml" => Ext::Svg,
        "tiff" => Ext::Tif,
        "vnd.wap.wbmp" => Ext::Wbmp,
        "webp" => Ext::Webp,
        "x-icon" => Ext::Ico,
        "x-jng" => Ext::Jng,
        "x-ms-bmp" => Ext::Bmp,
        // Caption
        "filmstrip+json" => Ext::Fs,
        "smptett+xml" => Ext::Tt,
        "ttaf+xml" => Ext::Dfxp,
        "ttml+xml" => Ext::Ttml,
        "x-ms-sami" => Ext::Sami,
        // Misc
        "gzip" => Ext::Gz,
        "json" => Ext::Json,
        "xml" => Ext::Xml,
        "zip" => Ext::Zip,
    };

    let mime = mime_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_lowercase();

    let subtype = mime.rsplit('/').next().unwrap_or("");

    let subtype_plus = subtype.rsplit('+').next().unwrap_or("");

    mime_type_map
        .get(mime.as_str())
        .or_else(|| mime_type_map.get(subtype))
        .or_else(|| mime_type_map.get(subtype_plus))
        .copied()
        .unwrap_or_default()
}

/// Parses `codecs_str` to return (vcodec, acodec)
pub fn parse_codecs(codecs: &str) -> Result<(Option<String>, Option<String>)> {
    if codecs.trim().is_empty() {
        return Ok((None, None));
    }

    let split_re = Regex::new(r"\s*,\s*").unwrap();
    let trimmed = codecs.trim().trim_matches(',');

    let split_codecs: Vec<&str> = split_re.split(trimmed).filter_map(|s| s.ok()).collect();

    let re = Regex::new(r"(?P<cp>[A-Za-z0-9]+)(?:\.(?P<params>.+))?$").unwrap();

    let mut vcodec: Option<String> = None;
    let mut acodec: Option<String> = None;

    for full_codec in split_codecs {
        let full_codec = full_codec.trim();
        if full_codec.is_empty() {
            continue;
        }

        let caps = match re.captures(full_codec).unwrap() {
            Some(c) => c,
            None => continue,
        };

        let codec_prefix = caps.name("cp").unwrap().as_str().to_lowercase();
        let params = caps.name("params").map(|m| m.as_str().to_string());

        // Build normalized "full" codec value
        let full = if let Some(param_str) = params {
            // strip leading zeros only for digit segments
            let cleaned_parts: Vec<String> = param_str
                .split('.')
                .map(|p| {
                    if p.chars().all(|c| c.is_ascii_digit()) {
                        let stripped = p.trim_start_matches('0');
                        if stripped.is_empty() {
                            "0".to_string()
                        } else {
                            stripped.to_string()
                        }
                    } else {
                        p.to_string()
                    }
                })
                .collect();

            format!("{}.{}", codec_prefix, cleaned_parts.join("."))
        } else {
            codec_prefix.clone()
        };

        // VIDEO prefixes
        const VIDEO_PREFIXES: &[&str] = &[
            "avc1", "avc2", "avc3", "avc4", "vp9", "vp8", "hev1", "hev2", "h263", "h264", "mp4v",
            "hvc1", "av01", "theora", "dvh1", "dvhe",
        ];

        // AUDIO prefixes
        const AUDIO_PREFIXES: &[&str] = &[
            "flac", "mp4a", "opus", "vorbis", "mp3", "aac", "ac-4", "ac-3", "ec-3", "eac3", "dtsc",
            "dtse", "dtsh", "dtsl",
        ];

        if VIDEO_PREFIXES.contains(&codec_prefix.as_str()) {
            if vcodec.is_none() {
                vcodec = Some(full);
            }
        } else if AUDIO_PREFIXES.contains(&codec_prefix.as_str()) {
            if acodec.is_none() {
                acodec = Some(full);
            }
        }
    }

    // Just trust me on this.
    let vcodec = vcodec.unwrap_or_else(|| "none".to_string());
    let acodec = acodec.unwrap_or_else(|| "none".to_string());

    Ok((Some(vcodec), Some(acodec)))
}
