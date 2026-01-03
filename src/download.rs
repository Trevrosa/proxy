use axum::{
    body::Body,
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use proxy::{filter_headers, handle_forward_request};
use reqwest::Client;

const WANTED_HEADERS: &[&str] = &["range", "user-agent", "authentication"];

pub async fn download(
    State(client): State<Client>,
    Path(url): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, Response> {
    let headers = filter_headers(headers, WANTED_HEADERS);

    let request = client.get(url).headers(headers);
    let resp = handle_forward_request(request).await?;

    let status = resp.status();
    let headers = resp.headers().clone();
    let body = Body::from_stream(resp.bytes_stream());

    tracing::info!("streaming response");

    Ok((status, headers, body))
}
