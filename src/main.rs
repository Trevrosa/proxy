mod download;
mod proxy;

use std::{
    env,
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};

use axum::{
    Router,
    http::StatusCode,
    routing::{any, get},
};
use tokio::{net::TcpListener, signal};
use tower_http::timeout::TimeoutLayer;
use tracing::{instrument, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

use download::download;
use proxy::proxy;

const DEFAULT_PORT: u16 = 8888;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if let Err(err) = dotenvy::dotenv() {
        eprintln!("could not load .env: {err}");
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let app = Router::new()
        .route("/dl/{*url}", get(download))
        .route("/web/{*url}", any(proxy))
        .with_state(reqwest::Client::new())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(10),
        ));

    let port = env::var("PROXY_PORT")
        .map(|p| p.parse().expect("configured port is not an int"))
        .unwrap_or(DEFAULT_PORT);
    let ip = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);

    tracing::info!("running server at :{port}");

    let listener = TcpListener::bind(ip).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await?;

    Ok(())
}

#[instrument(skip_all)]
async fn shutdown() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    tracing::info!("shutting down..");
}
