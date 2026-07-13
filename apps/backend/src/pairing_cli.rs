use std::env;
use std::time::Duration as StdDuration;

use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::auth::token_hash;
use crate::config::Config;
use crate::db;

pub async fn run_pair_command(config: Config, args: &[String]) -> Result<()> {
    let workspace_id = option_value(args, "--workspace")
        .or_else(|| env::var("ANTIROT_WORKSPACE_ID").ok())
        .unwrap_or_else(|| "default".to_string());
    let timeout_secs = option_value(args, "--timeout")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(60)
        .clamp(15, 300);
    let pool = db::create_pool(&config.database_url).await?;
    db::migrate(&pool).await?;
    let session = create_pairing_session(&pool, &workspace_id, timeout_secs).await?;

    println!("Open Antirot on your phone.");
    println!("Enter code: {}", session.code);
    println!("Expires in {timeout_secs} seconds.");
    println!();
    println!("Waiting for device...");

    match wait_for_claim(&pool, &session.id, timeout_secs).await? {
        Some(claim) => {
            println!(
                "Paired: {} ({})",
                claim.device_name.unwrap_or_else(|| "Phone".to_string()),
                claim.device_id
            );
        }
        None => {
            println!("Pairing expired. Run the command again.");
        }
    }

    Ok(())
}

struct PairingSession {
    id: String,
    code: String,
}

async fn create_pairing_session(
    pool: &Pool,
    workspace_id: &str,
    timeout_secs: u64,
) -> Result<PairingSession> {
    let code = Uuid::new_v4().simple().to_string();
    let code_hash = token_hash(&code);
    let id = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + Duration::seconds(timeout_secs as i64);
    let client = pool.get().await?;
    client
        .execute(
            "
            INSERT INTO pairing_sessions (id, workspace_id, code_hash, expires_at)
            VALUES ($1, $2, $3, $4)
            ",
            &[&id, &workspace_id, &code_hash, &expires_at],
        )
        .await
        .context("failed to create pairing session")?;
    Ok(PairingSession { id, code })
}

struct PairingClaim {
    device_id: String,
    device_name: Option<String>,
}

async fn wait_for_claim(
    pool: &Pool,
    session_id: &str,
    timeout_secs: u64,
) -> Result<Option<PairingClaim>> {
    let deadline = Utc::now() + Duration::seconds(timeout_secs as i64);
    while Utc::now() < deadline {
        let client = pool.get().await?;
        if let Some(row) = client
            .query_opt(
                "
                SELECT claimed_device_id, device_name
                FROM pairing_sessions
                WHERE id = $1
                  AND used_at IS NOT NULL
                ",
                &[&session_id],
            )
            .await?
        {
            let device_id: Option<String> = row.get("claimed_device_id");
            if let Some(device_id) = device_id {
                return Ok(Some(PairingClaim {
                    device_id,
                    device_name: row.get("device_name"),
                }));
            }
        }
        tokio::time::sleep(StdDuration::from_secs(1)).await;
    }
    Ok(None)
}

fn option_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find_map(|window| {
        if window[0] == name && !window[1].trim().is_empty() {
            Some(window[1].clone())
        } else {
            None
        }
    })
}
