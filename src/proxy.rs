use std::{env, sync::LazyLock};

use axum::{
    body::{self, Body},
    extract::{Path, Request, State},
    http::{StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use proxy::{filter_headers, handle_forward_request, rewrite_html_urls};
use reqwest::Client;

const WANTED_HEADERS: &[&str] = &["range", "user-agent", "authentication", "cookies", "accept"];

static PROXY_URL_PATH: LazyLock<String> = LazyLock::new(|| {
    let hostname = env::var("PROXY_HOSTNAME").expect("PROXY_HOSTNAME should be set");
    Uri::builder()
        .scheme("https")
        .authority(hostname)
        .path_and_query("/proxy/")
        .build()
        .expect("built uri should be valid")
        .to_string()
});

pub async fn proxy(
    State(client): State<Client>,
    Path(url): Path<String>,
    cookies: CookieJar,
    request: Request,
) -> Response {
    let (request, body) = request.into_parts();

    if cookies.get("trev-proxy-service-worker-installed").is_none() {
        return Redirect::temporary(&format!("/?back={url}")).into_response();
    }

    let body_limit = request
        .headers
        .get("content-length")
        .and_then(|k| k.to_str().ok())
        .and_then(|k| k.parse().ok())
        .unwrap_or(100_000_000); // 100 mb

    let body = match body::to_bytes(body, body_limit).await {
        Ok(body) => body,
        Err(err) => {
            tracing::warn!("failed to convert request body to bytes: {err}");
            return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
        }
    };

    let request = client
        .request(request.method, &url)
        .headers(filter_headers(request.headers, WANTED_HEADERS))
        .version(request.version)
        .body(body);

    tracing::info!("proxying request");

    let resp = match handle_forward_request(request).await {
        Ok(resp) => resp,
        Err(err) => return err,
    };

    let status = resp.status();
    let headers = resp.headers().clone();

    let body = if headers
        .get("content-type")
        .is_some_and(|h| h.to_str().is_ok_and(|h| h.contains("text/html")))
    {
        let Ok(body) = resp.text().await else {
            tracing::warn!("could not get response text");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "could not get the response text",
            )
                .into_response();
        };

        let body = rewrite_html_urls(body, &url, &PROXY_URL_PATH);

        tracing::debug!("rewrote html resource");

        Body::new(body)
    } else {
        Body::from_stream(resp.bytes_stream())
    };

    tracing::info!("sending response");

    (status, headers, body).into_response()
}
