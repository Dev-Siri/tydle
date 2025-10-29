use std::collections::HashMap;

pub struct YtExtractor {
    http_client: reqwest::Client,
    x_forwarded_for_ip: Option<&'static str>,
}

trait InfoExtractor {
    fn request_webpage(self, url: &str, headers: HashMap<&str, &str>, query: HashMap<&str, &str>);
}

impl YtExtractor {
    fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
            x_forwarded_for_ip: None,
        }
    }
}

impl InfoExtractor for YtExtractor {
    fn request_webpage(self, url: &str, headers: HashMap<&str, &str>, query: HashMap<&str, &str>) {
        let mut headers_copy = headers.clone();
        // ! SKIPPED PART HERE

        // Some sites check X-Forwarded-For HTTP header in order to figure out the origin of the client behind proxy.
        // This allows bypassing geo restriction by faking this header's value to IP that belongs to some geo unrestricted country.
        // We will do so once we encounter any geo restriction error.
        if let Some(forwarded_ip) = self.x_forwarded_for_ip {
            headers_copy.insert("X-Forwarded-For", forwarded_ip);
        }

        let mut request_builder = self.http_client.get(url);

        for (key, value) in headers {
            request_builder = request_builder.header(key, value);
        }

        for (key, value) in query {
            request_builder = request_builder.query(&[key, value]);
        }

        ()
    }
}
