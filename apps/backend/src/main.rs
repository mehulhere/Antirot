mod alarm;
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
mod rate_limit;
mod routes;
mod secrets;

use std::env;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderValue, Method};
use axum::Router;
use deadpool_postgres::Pool;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::rate_limit::RequestRateLimiter;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub pool: Pool,
    pub rate_limiter: RequestRateLimiter,
    pub chat_concurrency: Arc<Semaphore>,
    pub speech_concurrency: Arc<Semaphore>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("antirot_backend=info".parse()?),
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
    let cors = cors_layer(&config.cors_allowed_origins)?;
    let chat_concurrency = Arc::new(Semaphore::new(config.chat_concurrency_limit));
    let speech_concurrency = Arc::new(Semaphore::new(config.speech_concurrency_limit));
    let state = AppState {
        config,
        pool,
        rate_limiter: RequestRateLimiter::default(),
        chat_concurrency,
        speech_concurrency,
    };
    spawn_nightly_distillation_worker(state.clone());
    spawn_memory_index_worker(state.clone());
    spawn_alarm_wake_worker(state.clone());
    let app = Router::new()
        .merge(routes::router())
        .layer(cors)
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

fn spawn_alarm_wake_worker(state: AppState) {
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(5)).await;
            for _ in 0..50 {
                match crate::llm::process_next_alarm_wake_outbox(&state.pool, &state.config).await {
                    Ok(true) => continue,
                    Ok(false) => break,
                    Err(err) => {
                        warn!(
                            error = %err,
                            "🔴 FALLBACK: background alarm wake deferred - Reason: outbox worker failed - Impact: pending or expired APNs wakes retry on the next bounded scan and clients may poll"
                        );
                        break;
                    }
                }
            }
        }
    });
}

fn spawn_memory_index_worker(state: AppState) {
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(5)).await;
            loop {
                match crate::memory::process_next_memory_index_job(&state.pool, &state.config).await
                {
                    Ok(true) => continue,
                    Ok(false) => break,
                    Err(err) => {
                        warn!(
                            error = %err,
                            "🔴 FALLBACK: background memory indexing deferred - Reason: index worker failed - Impact: canonical memory remains available through lexical fallback and indexing retries on the next scan"
                        );
                        break;
                    }
                }
            }
        }
    });
}

fn cors_layer(allowed_origins: &[String]) -> Result<CorsLayer> {
    let origins = allowed_origins
        .iter()
        .map(|origin| {
            HeaderValue::from_str(origin)
                .with_context(|| format!("invalid CORS origin header value {origin}"))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST, Method::PUT])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE]))
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
