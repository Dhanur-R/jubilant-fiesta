mod config;
mod db;
mod http_helpers;
mod middleware;
mod models;
mod routes;
mod validation;

use axum::{middleware as axum_middleware, routing::{get, post}, Router};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use middleware::RateLimiter;

#[derive(Clone)]
pub struct AppState {
    pub supabase: std::sync::Arc<db::SupabaseClient>,
    pub public_base_url: Option<String>,
}

async fn cleanup_task(supabase: std::sync::Arc<db::SupabaseClient>) {
    let mut interval = tokio::time::interval(config::Config::cleanup_interval());

    loop {
        interval.tick().await;
        tracing::info!("running cleanup task for unused links");

        // Cleanup links not accessed in 1 year
        match db::cleanup_unused_links(&supabase).await {
            Ok(count) => {
                if count > 0 {
                    tracing::info!("cleaned up {} unused links (not accessed in {} days)", count, config::Config::LINK_INACTIVE_DAYS);
                }
            }
            Err(e) => tracing::error!("cleanup task failed: {}", e),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| config::Config::DEFAULT_RUST_LOG.into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let supabase_url = std::env::var("SUPABASE_URL")
        .expect("SUPABASE_URL must be set (e.g., https://xxx.supabase.co)");
    let supabase_key =
        std::env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set (service_role key)");
    let public_base_url = std::env::var("PUBLIC_BASE_URL").ok();
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(config::Config::DEFAULT_PORT);

    let supabase = std::sync::Arc::new(db::SupabaseClient::new(supabase_url, supabase_key));

    let state = AppState {
        supabase: supabase.clone(),
        public_base_url,
    };

    tokio::spawn(cleanup_task(supabase));

    let rate_limiter = RateLimiter::new(
        config::Config::RATE_LIMIT_MAX_REQUESTS,
        config::Config::rate_limit_window(),
    );

    let app = Router::new()
        .route("/", get(routes::index))
        .route(
            "/shorten",
            post(routes::shorten).layer(axum_middleware::from_fn(move |req, next| {
                let limiter = rate_limiter.clone();
                async move { limiter.middleware(req, next).await }
            })),
        )
        .route("/:code", get(routes::redirect))
        .route("/health", get(routes::health))
        .nest_service("/static", tower_http::services::ServeDir::new("static"))
        .with_state(state)
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(axum_middleware::from_fn(middleware::security_headers));

    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
