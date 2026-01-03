use axum::{
    body::Body,
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
};
use reqwest::Client;

pub async fn download(
    State(client): State<Client>,
    Path(url): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, String> {
    let resp = match client.get(url).headers(headers).send().await {
        Ok(resp) => resp,
        Err(err) => {
            // TODO: do we want to hide the url from log?
            tracing::warn!("failed to send request: {err}");
            return Err(err.to_string());
        }
    };

    let status = resp.status();
    let headers = resp.headers().clone();
    let body = Body::from_stream(resp.bytes_stream());

    tracing::info!("streaming response");

    Ok((status, headers, body))
}
