use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::http::HeaderMap;

const MAX_TRACKED_BUCKETS: usize = 10_000;

#[derive(Clone, Default)]
pub struct RequestRateLimiter {
    entries: Arc<Mutex<HashMap<String, Window>>>,
}

struct Window {
    started_at: Instant,
    count: u32,
}

impl RequestRateLimiter {
    pub fn check(&self, headers: &HeaderMap, bucket: &str, limit: u32, window: Duration) -> bool {
        self.check_key(bucket, &client_key(headers), limit, window)
    }

    pub fn check_key(&self, bucket: &str, subject: &str, limit: u32, window: Duration) -> bool {
        if limit == 0 {
            return false;
        }

        let now = Instant::now();
        let key = format!("{bucket}:{subject}");
        let Ok(mut entries) = self.entries.lock() else {
            return false;
        };

        if entries.len() >= MAX_TRACKED_BUCKETS && !entries.contains_key(&key) {
            entries.retain(|_, entry| now.duration_since(entry.started_at) < window);
            if entries.len() >= MAX_TRACKED_BUCKETS {
                return false;
            }
        }

        let entry = entries.entry(key).or_insert(Window {
            started_at: now,
            count: 0,
        });
        if now.duration_since(entry.started_at) >= window {
            entry.started_at = now;
            entry.count = 0;
        }
        if entry.count >= limit {
            return false;
        }
        entry.count += 1;
        true
    }
}

fn client_key(headers: &HeaderMap) -> String {
    for header_name in ["x-forwarded-for", "x-real-ip"] {
        if let Some(value) = headers
            .get(header_name)
            .and_then(|value| value.to_str().ok())
        {
            if let Some(client) = value
                .rsplit(',')
                .map(str::trim)
                .find(|value| !value.is_empty())
            {
                return client.to_string();
            }
        }
    }
    "direct".to_string()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::http::{HeaderMap, HeaderValue};

    use super::RequestRateLimiter;

    #[test]
    fn rejects_requests_after_bucket_limit() {
        let limiter = RequestRateLimiter::default();
        let headers = HeaderMap::new();

        assert!(limiter.check(&headers, "pairing", 2, Duration::from_secs(60)));
        assert!(limiter.check(&headers, "pairing", 2, Duration::from_secs(60)));
        assert!(!limiter.check(&headers, "pairing", 2, Duration::from_secs(60)));
    }

    #[test]
    fn uses_proxy_appended_client_ip_for_separate_buckets() {
        let limiter = RequestRateLimiter::default();
        let mut first = HeaderMap::new();
        first.insert(
            "x-forwarded-for",
            HeaderValue::from_static("spoofed, 203.0.113.10"),
        );
        let mut second = HeaderMap::new();
        second.insert(
            "x-forwarded-for",
            HeaderValue::from_static("spoofed, 203.0.113.11"),
        );

        assert!(limiter.check(&first, "google", 1, Duration::from_secs(60)));
        assert!(!limiter.check(&first, "google", 1, Duration::from_secs(60)));
        assert!(limiter.check(&second, "google", 1, Duration::from_secs(60)));
    }

    #[test]
    fn authenticated_subjects_have_separate_rate_limit_keys() {
        let limiter = RequestRateLimiter::default();
        assert!(limiter.check_key("chat", "user-a", 1, Duration::from_secs(60)));
        assert!(!limiter.check_key("chat", "user-a", 1, Duration::from_secs(60)));
        assert!(limiter.check_key("chat", "user-b", 1, Duration::from_secs(60)));
    }
}
