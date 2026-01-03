use axum::{
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

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
