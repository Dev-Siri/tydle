use std::collections::HashMap;

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
