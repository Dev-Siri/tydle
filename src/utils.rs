use std::collections::HashMap;

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
