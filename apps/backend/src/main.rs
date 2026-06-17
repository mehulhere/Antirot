mod apns;
mod auth;
mod config;
mod db;
mod error;
mod llm;
mod memory;
mod models;
mod pairing_cli;
mod prompt;
mod routes;

use std::env;

use anyhow::Result;
use axum::Router;
use deadpool_postgres::Pool;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub pool: Pool,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("antirot_bridge=info".parse()?),
        )
        .json()
        .init();

    let config = Config::from_env()?;
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|arg| arg == "pair") {
        return pairing_cli::run_pair_command(config, &args[1..]).await;
    }

    let pool = db::create_pool(&config.database_url).await?;
    db::migrate(&pool).await?;

    let bind = config.bind;
    let state = AppState { config, pool };
    spawn_nightly_distillation_worker(state.clone());
    let app = Router::new()
        .merge(routes::router())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = TcpListener::bind(bind).await?;
    let local_addr = listener.local_addr()?;
    info!(bind = %local_addr, "Antirot backend listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

fn spawn_nightly_distillation_worker(state: AppState) {
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(300)).await;
            match crate::memory::distill_all_idle_users_due(&state.pool, &state.config).await {
                Ok(outcomes) => {
                    for outcome in outcomes {
                        info!(
                            user_id = %outcome.user_id,
                            date = %outcome.date,
                            "background nightly memory distillation completed"
                        );
                    }
                }
                Err(err) => {
                    warn!(
                        error = %err,
                        "🔴 FALLBACK: background nightly memory distillation skipped - Reason: worker scan failed - Impact: distillation will retry on next scan or chat"
                    );
                }
            }
        }
    });
}
