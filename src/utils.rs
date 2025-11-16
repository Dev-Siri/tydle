use anyhow::Result;
use maplit::hashmap;
use std::collections::HashMap;

use crate::yt_interface::Ext;
use url::{Url, form_urlencoded};

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
