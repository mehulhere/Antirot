use axum::http::HeaderMap;
use chrono::{Duration, Utc};
use deadpool_postgres::Pool;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::Config;
use crate::error::{AppError, AppResult};

pub const SESSION_COOKIE_NAME: &str = "antirot_session";
pub const SESSION_DAYS: i64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClaims {
    pub sub: String,
    pub device_id: String,
    pub exp: usize,
}

pub fn require_admin_auth(headers: &HeaderMap, config: &Config) -> AppResult<()> {
    let token = bearer_token(headers).ok_or(AppError::Unauthorized)?;
    if constant_time_eq(token, &config.admin_token) {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}

pub async fn require_device_auth(
    headers: &HeaderMap,
    config: &Config,
    pool: &Pool,
) -> AppResult<()> {
    if let Some(token) = bearer_token(headers) {
        if constant_time_eq(token, &config.device_token)
            || constant_time_eq(token, &config.admin_token)
        {
            return Ok(());
        }

        let client = pool.get().await?;
        let token_hash = token_hash(token);
        let exists = client
            .query_opt(
                "SELECT 1 FROM devices WHERE api_token_hash = $1",
                &[&token_hash],
            )
            .await?
            .is_some();
        if exists {
            return Ok(());
        }
    }

    if session_from_headers(headers, config).is_some() {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}

pub async fn require_device_auth_for(
    headers: &HeaderMap,
    config: &Config,
    pool: &Pool,
    device_id: &str,
) -> AppResult<()> {
    if let Some(token) = bearer_token(headers) {
        if constant_time_eq(token, &config.device_token)
            || constant_time_eq(token, &config.admin_token)
        {
            return Ok(());
        }

        let client = pool.get().await?;
        let token_hash = token_hash(token);
        let matches = client
            .query_opt(
                "SELECT 1 FROM devices WHERE api_token_hash = $1 AND device_id = $2",
                &[&token_hash, &device_id],
            )
            .await?
            .is_some();
        if matches {
            return Ok(());
        }
    }

    if session_from_headers(headers, config).is_some_and(|claims| claims.device_id == device_id) {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}

pub fn token_hash(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    value.strip_prefix("Bearer ").map(str::trim)
}

fn constant_time_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    let max_len = left.len().max(right.len());
    let mut diff = left.len() ^ right.len();
    for index in 0..max_len {
        let a = *left.get(index).unwrap_or(&0);
        let b = *right.get(index).unwrap_or(&0);
        diff |= (a ^ b) as usize;
    }
    diff == 0
}

pub async fn get_user_id_from_auth(
    headers: &HeaderMap,
    config: &Config,
    pool: &Pool,
) -> AppResult<String> {
    if let Some(token) = bearer_token(headers) {
        if constant_time_eq(token, &config.admin_token)
            || constant_time_eq(token, &config.device_token)
        {
            return Ok("admin".to_string());
        }

        let client = pool.get().await?;
        let token_hash = token_hash(token);
        let row = client
            .query_opt(
                "SELECT user_id FROM devices WHERE api_token_hash = $1",
                &[&token_hash],
            )
            .await?;

        if let Some(row) = row {
            let user_id: Option<String> = row.get("user_id");
            if let Some(uid) = user_id {
                return Ok(uid);
            }
        }
    }

    if let Some(claims) = session_from_headers(headers, config) {
        return Ok(claims.sub);
    }

    Err(AppError::Unauthorized)
}

pub fn issue_session_jwt(config: &Config, user_id: &str, device_id: &str) -> AppResult<String> {
    let expires_at = Utc::now() + Duration::days(SESSION_DAYS);
    let claims = SessionClaims {
        sub: user_id.to_string(),
        device_id: device_id.to_string(),
        exp: expires_at.timestamp() as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
    .map_err(|_| AppError::Unauthorized)
}

pub fn session_cookie_header(token: &str, max_age_seconds: i64) -> String {
    format!(
        "{SESSION_COOKIE_NAME}={token}; Max-Age={max_age_seconds}; Path=/; HttpOnly; SameSite=Lax; Secure"
    )
}

pub fn expired_session_cookie_header() -> String {
    format!("{SESSION_COOKIE_NAME}=; Max-Age=0; Path=/; HttpOnly; SameSite=Lax; Secure")
}

pub fn session_from_headers(headers: &HeaderMap, config: &Config) -> Option<SessionClaims> {
    let token = session_cookie(headers)?;
    decode::<SessionClaims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .ok()
    .map(|data| data.claims)
}

fn session_cookie(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    value.split(';').find_map(|part| {
        let trimmed = part.trim();
        let (name, token) = trimmed.split_once('=')?;
        (name == SESSION_COOKIE_NAME).then_some(token.trim())
    })
}
