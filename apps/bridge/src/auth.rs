use axum::http::HeaderMap;

use crate::config::Config;
use crate::error::{AppError, AppResult};

#[derive(Clone, Copy)]
pub enum AuthScope {
    Admin,
    Device,
}

pub fn require_auth(headers: &HeaderMap, config: &Config, scope: AuthScope) -> AppResult<()> {
    let token = bearer_token(headers).ok_or(AppError::Unauthorized)?;
    let allowed = match scope {
        AuthScope::Admin => constant_time_eq(token, &config.admin_token),
        AuthScope::Device => {
            constant_time_eq(token, &config.device_token)
                || constant_time_eq(token, &config.admin_token)
        }
    };
    if allowed {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
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
