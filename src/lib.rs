use std::sync::LazyLock;

use axum::{
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use regex::Regex;

/// filter `headers` according to the `wanted_headers`.
pub fn filter_headers<'a>(headers: HeaderMap, wanted_headers: impl AsRef<[&'a str]>) -> HeaderMap {
    let wanted_headers = wanted_headers.as_ref();
    headers
        .into_iter()
        .filter_map(|(h, v)| match h {
            Some(h) if wanted_headers.contains(&h.as_str()) => Some((h, v)),
            _ => None,
        })
        .collect()
}

/// send the `request` and get its the `Response`
///
/// # Errors
///
/// if there was an error sending the request, return it wrapped in an `axum::response::Response`
pub async fn handle_forward_request(
    request: reqwest::RequestBuilder,
) -> Result<reqwest::Response, axum::response::Response> {
    request.send().await.map_err(|err| {
        // TODO: do we want to hide the url from log?
        tracing::warn!("failed to send request: {err}");

        let status = if err.is_request() {
            err.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };

        (status, err.to_string()).into_response()
    })
}

const URL_ATTRIBUTES: &str = "(src|href|action)";

static URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r#"{URL_ATTRIBUTES}=(")http"#)).expect("should be able to compile regex")
});

static REL_URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r#"{URL_ATTRIBUTES}="(/)"#)).expect("should be able to compile regex")
});

pub fn init_regexes() {
    LazyLock::force(&URL_REGEX);
    LazyLock::force(&REL_URL_REGEX);
}

fn rolling_replace(html: &mut String, regex: &Regex, replacement: &str, offset: usize) {
    let mut matches = 0;
    let mut new_offset = 0;
    for captures in regex.captures_iter(&html.clone()) {
        let url = captures.get(2).expect("there should be two capture groups");
        matches += 1;
        let idx = url.start() + new_offset + offset;
        tracing::trace!(
            "found url at {idx} (og: {}) ({})",
            url.start() + offset,
            captures.get_match().as_str()
        );
        html.insert_str(idx, replacement);
        new_offset += replacement.len();
    }
    tracing::debug!("{matches} urls matched");
}

pub fn rewrite_html_urls(mut html: String, target_url: &str, proxy_url_path: &str) -> String {
    rolling_replace(&mut html, &URL_REGEX, proxy_url_path, 1);

    let rel_new_url = format!("{proxy_url_path}{target_url}/");
    rolling_replace(&mut html, &REL_URL_REGEX, &rel_new_url, 0);

    html
}

#[cfg(test)]
mod rewrite_urls_tests {
    use tracing_test::traced_test;

    macro_rules! test_url {
        ($name:ident, $url:literal) => {
            #[test]
            #[traced_test]
            fn $name() {
                let html = reqwest::blocking::Client::new()
                    .get($url)
                    .send()
                    .expect("should be able to get url")
                    .text()
                    .expect("should be able to get html text");
                super::rewrite_html_urls(html, $url, "localhost");
            }
        };
    }

    test_url!(google, "https://www.google.com");
    test_url!(github, "https://github.com");
}
