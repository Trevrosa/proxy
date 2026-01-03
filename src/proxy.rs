use axum::{
    body::{self, Body},
    extract::{Path, Request, State},
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use proxy::{filter_headers, handle_forward_request};
use reqwest::Client;

const WANTED_HEADERS: &[&str] = &["range", "user-agent", "authentication", "cookies"];

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
        .request(request.method, url)
        .headers(filter_headers(request.headers, WANTED_HEADERS))
        .version(request.version)
        .body(body);

    let resp = match handle_forward_request(request).await {
        Ok(resp) => resp,
        Err(err) => return err,
    };

    let status = resp.status();
    let mut headers = resp.headers().clone();
    headers.append(
        "service-worker-allowed",
        HeaderValue::from_str("/").expect("should be able to construct header value from str"),
    );
    let body = Body::from_stream(resp.bytes_stream());

    tracing::info!("proxying request");

    (status, headers, body).into_response()
}
