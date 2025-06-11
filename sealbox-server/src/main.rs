use sealbox_server::{config::SealboxConfig, create_app, error::Result};
use tracing::{error, info};
use tracing_subscriber::{self, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file if present
    dotenvy::dotenv().ok();

    // Enhanced tracing_subscriber initialization with log level filtering and formatting
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // axum logs rejections from built-in extractors with the `axum::rejection`
            // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
            format!(
                "{}=debug,tower_http=debug,axum::rejection=trace",
                env!("CARGO_CRATE_NAME")
            )
            .into()
        }))
        .with_target(true)
        .with_line_number(true)
        .init();

    info!("Sealbox Server starting up...");

    // Load configuration from environment variables
    let config = match SealboxConfig::from_env() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Build application routes (all routes are managed in api.rs)
    let app = create_app(&config)?;

    // Listening address from configuration
    let addr = &config.listen_addr;
    info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| {
            error!("Failed to bind address {}: {}", addr, e);
            std::process::exit(1);
        });
    if let Err(e) = axum::serve(listener, app).await {
        error!("Server crashed: {}", e);
    }

    Ok(())
}
