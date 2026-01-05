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

static URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https?://(www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}(?-u:\b)([-a-zA-Z0-9()@:%_\+.~#?&/=]*)").expect("should be able to compile regex")
});

static REL_URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:((src|href)="))(/)"#).expect("should be able to compile regex")
});

pub fn rewrite_html_urls(mut html: String, target_url: &str, proxy_url_path: &str) -> String {
    for url in URL_REGEX.find_iter(&html.clone()) {
        tracing::debug!("found url at {} ({})", url.start(), url.as_str());
        html.insert_str(url.start(), proxy_url_path);
    }

    let rel_new_url = std::path::Path::new(proxy_url_path).join(target_url);
    let rel_new_url = rel_new_url.to_string_lossy();

    // let html = html
    //     .replace(r#"src="/"#, &format!(r#"src="{rel_new_url}/"#))
    //     .replace(r#"href="/"#, &format!(r#"href="{rel_new_url}/"#));

    for captures in REL_URL_REGEX.captures_iter(&html.clone()) {
        let url = captures
            .get(3)
            .expect("there should be three capture groups");
        tracing::debug!(
            "found rel url at {} ({})",
            url.start(),
            captures.get_match().as_str()
        );
        html.insert_str(url.start(), &rel_new_url);
    }

    html
}

#[cfg(test)]
mod tests {
    use tracing_test::traced_test;

    #[test]
    #[traced_test]
    fn google_rewrite_urls() {
        let html = reqwest::blocking::Client::new()
            .get("https://www.google.com")
            .send()
            .expect("should be able to get google")
            .text()
            .expect("should be able to get html text");
        super::rewrite_html_urls(html, "https://www.google.com", "localhost");
    }
}
