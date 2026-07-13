use std::env;
use std::net::SocketAddr;

use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct Config {
    pub bind: SocketAddr,
    pub database_url: String,
    pub admin_token: String,
    pub device_token: String,
    pub jwt_secret: String,
    pub allow_anonymous_sessions: bool,
    pub allow_legacy_device_bootstrap: bool,
    pub cors_allowed_origins: Vec<String>,
    pub google_allowed_client_ids: Vec<String>,
    pub apns: Option<ApnsConfig>,
    pub memory_embeddings: MemoryEmbeddingConfig,
    pub speech: SpeechConfig,
    pub chat_concurrency_limit: usize,
    pub speech_concurrency_limit: usize,
    pub speech_daily_stt_bytes: i64,
    pub speech_daily_tts_chars: i64,
    pub memory_daily_embedding_calls: i64,
    pub byok_encryption_key: Option<ByokEncryptionKey>,
}

#[derive(Clone)]
pub struct ByokEncryptionKey(pub [u8; 32]);

impl std::fmt::Debug for ByokEncryptionKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ByokEncryptionKey([REDACTED])")
    }
}

#[derive(Clone, Debug)]
pub struct ApnsConfig {
    pub team_id: String,
    pub key_id: String,
    pub private_key_path: Option<String>,
    pub private_key_pem: Option<String>,
    pub topic: String,
    pub endpoint: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryEmbeddingConfig {
    pub provider: String,
    pub model: String,
    pub fallback_provider: String,
    pub fallback_model: String,
    pub gemini_api_key: Option<String>,
    pub voyage_api_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpeechConfig {
    pub smallest_stt_url: String,
    pub smallest_api_key: Option<String>,
    pub inworld_base_url: String,
    pub inworld_api_key: Option<String>,
    pub inworld_tts_model: String,
    pub inworld_tts_voice_id: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let bind = env::var("ANTIROT_BACKEND_BIND")
            .unwrap_or_else(|_| "127.0.0.1:8787".to_string())
            .parse()
            .context("ANTIROT_BACKEND_BIND must be a socket address like 127.0.0.1:8787")?;
        let database_url = env::var("DATABASE_URL")
            .context("DATABASE_URL is required, for example postgres://antirot:secret@localhost/antirot_backend")?;
        let admin_token =
            env::var("ANTIROT_ADMIN_TOKEN").context("ANTIROT_ADMIN_TOKEN is required")?;
        let device_token =
            env::var("ANTIROT_DEVICE_TOKEN").context("ANTIROT_DEVICE_TOKEN is required")?;
        let jwt_secret = env::var("ANTIROT_JWT_SECRET").unwrap_or_else(|_| admin_token.clone());
        let allow_anonymous_sessions =
            env::var("ANTIROT_ALLOW_ANONYMOUS_SESSIONS").ok().as_deref() == Some("1");
        let allow_legacy_device_bootstrap = env::var("ANTIROT_ALLOW_LEGACY_DEVICE_BOOTSTRAP")
            .ok()
            .as_deref()
            == Some("1");
        let cors_allowed_origins =
            parse_cors_allowed_origins(env::var("ANTIROT_CORS_ALLOWED_ORIGINS").ok().as_deref())?;
        let google_allowed_client_ids = google_allowed_client_ids();
        let apns = apns_config();
        let memory_embeddings = memory_embedding_config();
        let speech = speech_config();
        let chat_concurrency_limit =
            bounded_usize_env("ANTIROT_CHAT_CONCURRENCY_LIMIT", 12, 1, 64)?;
        let speech_concurrency_limit =
            bounded_usize_env("ANTIROT_SPEECH_CONCURRENCY_LIMIT", 4, 1, 32)?;
        let speech_daily_stt_bytes = bounded_i64_env(
            "ANTIROT_SPEECH_DAILY_STT_BYTES",
            100 * 1024 * 1024,
            1,
            10_000_000_000,
        )?;
        let speech_daily_tts_chars =
            bounded_i64_env("ANTIROT_SPEECH_DAILY_TTS_CHARS", 50_000, 1, 10_000_000)?;
        let memory_daily_embedding_calls =
            bounded_i64_env("ANTIROT_MEMORY_DAILY_EMBEDDING_CALLS", 500, 0, 100_000)?;
        let byok_encryption_key = byok_encryption_key()?;

        Ok(Self {
            bind,
            database_url,
            admin_token,
            device_token,
            jwt_secret,
            allow_anonymous_sessions,
            allow_legacy_device_bootstrap,
            cors_allowed_origins,
            google_allowed_client_ids,
            apns,
            memory_embeddings,
            speech,
            chat_concurrency_limit,
            speech_concurrency_limit,
            speech_daily_stt_bytes,
            speech_daily_tts_chars,
            memory_daily_embedding_calls,
            byok_encryption_key,
        })
    }
}

fn byok_encryption_key() -> Result<Option<ByokEncryptionKey>> {
    let Some(raw) = env::var("ANTIROT_BYOK_ENCRYPTION_KEY_HEX")
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(None);
    };
    let decoded = hex::decode(raw.trim())
        .context("ANTIROT_BYOK_ENCRYPTION_KEY_HEX must be exactly 64 hexadecimal characters")?;
    let bytes: [u8; 32] = decoded.try_into().map_err(|_| {
        anyhow::anyhow!("ANTIROT_BYOK_ENCRYPTION_KEY_HEX must decode to exactly 32 bytes")
    })?;
    Ok(Some(ByokEncryptionKey(bytes)))
}

fn bounded_usize_env(name: &str, default: usize, min: usize, max: usize) -> Result<usize> {
    let value = match env::var(name) {
        Ok(raw) => raw
            .parse::<usize>()
            .with_context(|| format!("{name} must be an integer between {min} and {max}"))?,
        Err(_) => default,
    };
    anyhow::ensure!(
        (min..=max).contains(&value),
        "{name} must be between {min} and {max}"
    );
    Ok(value)
}

fn bounded_i64_env(name: &str, default: i64, min: i64, max: i64) -> Result<i64> {
    let value = match env::var(name) {
        Ok(raw) => raw
            .parse::<i64>()
            .with_context(|| format!("{name} must be an integer between {min} and {max}"))?,
        Err(_) => default,
    };
    anyhow::ensure!(
        (min..=max).contains(&value),
        "{name} must be between {min} and {max}"
    );
    Ok(value)
}

fn parse_cors_allowed_origins(raw: Option<&str>) -> Result<Vec<String>> {
    const DEFAULT_ORIGINS: &str = "https://antirot.org,https://www.antirot.org,http://localhost:3000,http://127.0.0.1:3000,http://localhost:3001,http://127.0.0.1:3001";
    let mut origins = Vec::new();

    for value in raw.unwrap_or(DEFAULT_ORIGINS).split(',') {
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        anyhow::ensure!(
            value != "*",
            "ANTIROT_CORS_ALLOWED_ORIGINS cannot contain *"
        );
        let url =
            reqwest::Url::parse(value).with_context(|| format!("invalid CORS origin {value}"))?;
        anyhow::ensure!(
            matches!(url.scheme(), "http" | "https")
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none()
                && url.path() == "/",
            "CORS origin must contain only an http(s) scheme, host, and optional port: {value}"
        );
        let origin = url.origin().ascii_serialization();
        anyhow::ensure!(origin != "null", "invalid CORS origin {value}");
        if !origins.contains(&origin) {
            origins.push(origin);
        }
    }

    Ok(origins)
}

fn speech_config() -> SpeechConfig {
    SpeechConfig {
        smallest_stt_url: env::var("SMALLEST_STT_URL")
            .unwrap_or_else(|_| "https://api.smallest.ai/waves/v1/stt/".to_string()),
        smallest_api_key: env::var("SMALLEST_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty()),
        inworld_base_url: env::var("INWORLD_BASE_URL")
            .unwrap_or_else(|_| "https://api.inworld.ai".to_string()),
        inworld_api_key: env::var("INWORLD_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty()),
        inworld_tts_model: env::var("INWORLD_TTS_MODEL")
            .unwrap_or_else(|_| "inworld-tts-1.5-mini".to_string()),
        inworld_tts_voice_id: env::var("INWORLD_TTS_VOICE_ID")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| Some("Dennis".to_string())),
    }
}

fn memory_embedding_config() -> MemoryEmbeddingConfig {
    MemoryEmbeddingConfig {
        provider: "gemini".to_string(),
        model: env::var("ANTIROT_MEMORY_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "gemini-embedding-001".to_string()),
        fallback_provider: "voyage".to_string(),
        fallback_model: env::var("ANTIROT_MEMORY_EMBEDDING_FALLBACK_MODEL")
            .unwrap_or_else(|_| "voyage-4-large".to_string()),
        gemini_api_key: env::var("ANTIROT_MEMORY_GEMINI_API_KEY")
            .or_else(|_| env::var("GEMINI_API_KEY"))
            .ok()
            .filter(|value| !value.trim().is_empty()),
        voyage_api_key: env::var("ANTIROT_MEMORY_VOYAGE_API_KEY")
            .or_else(|_| env::var("VOYAGE_API_KEY"))
            .ok()
            .filter(|value| !value.trim().is_empty()),
    }
}

fn apns_config() -> Option<ApnsConfig> {
    let team_id = env::var("ANTIROT_APNS_TEAM_ID").ok()?;
    let key_id = env::var("ANTIROT_APNS_KEY_ID").ok()?;
    let private_key_path = env::var("ANTIROT_APNS_PRIVATE_KEY_PATH")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let private_key_pem = env::var("ANTIROT_APNS_PRIVATE_KEY_PEM")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let topic =
        env::var("ANTIROT_APNS_TOPIC").unwrap_or_else(|_| "com.mehulhere.Antirot".to_string());
    let environment = env::var("ANTIROT_APNS_ENV")
        .unwrap_or_else(|_| "sandbox".to_string())
        .to_ascii_lowercase();
    let endpoint = apns_endpoint_for_environment(&environment).to_string();

    Some(ApnsConfig {
        team_id,
        key_id,
        private_key_path,
        private_key_pem,
        topic,
        endpoint,
    })
}

fn apns_endpoint_for_environment(environment: &str) -> &'static str {
    match environment {
        "sandbox" | "development" | "dev" => "https://api.sandbox.push.apple.com",
        _ => "https://api.push.apple.com",
    }
}

fn google_allowed_client_ids() -> Vec<String> {
    let mut values = Vec::new();
    for key in [
        "GOOGLE_ALLOWED_CLIENT_IDS",
        "GOOGLE_IOS_CLIENT_ID",
        "GOOGLE_ANDROID_CLIENT_ID",
        "GOOGLE_WEB_CLIENT_ID",
    ] {
        if let Ok(raw) = env::var(key) {
            values.extend(
                raw.split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned),
            );
        }
    }
    values.sort();
    values.dedup();
    values
}

#[cfg(test)]
mod tests {
    use super::{
        apns_endpoint_for_environment, memory_embedding_config, parse_cors_allowed_origins,
        speech_config,
    };
    use std::env;

    #[test]
    fn apns_endpoint_uses_sandbox_only_when_explicit() {
        assert_eq!(
            apns_endpoint_for_environment("sandbox"),
            "https://api.sandbox.push.apple.com"
        );
        assert_eq!(
            apns_endpoint_for_environment("development"),
            "https://api.sandbox.push.apple.com"
        );
        assert_eq!(
            apns_endpoint_for_environment(""),
            "https://api.push.apple.com"
        );
    }

    #[test]
    fn apns_endpoint_accepts_production_aliases() {
        assert_eq!(
            apns_endpoint_for_environment("production"),
            "https://api.push.apple.com"
        );
        assert_eq!(
            apns_endpoint_for_environment("prod"),
            "https://api.push.apple.com"
        );
    }

    #[test]
    fn memory_embeddings_use_gemini_with_voyage_fallback() {
        env::remove_var("ANTIROT_MEMORY_EMBEDDING_MODEL");
        env::remove_var("ANTIROT_MEMORY_EMBEDDING_FALLBACK_MODEL");
        env::remove_var("ANTIROT_MEMORY_GEMINI_API_KEY");
        env::remove_var("ANTIROT_MEMORY_VOYAGE_API_KEY");
        env::remove_var("GEMINI_API_KEY");
        env::remove_var("VOYAGE_API_KEY");

        let config = memory_embedding_config();
        assert_eq!(config.provider, "gemini");
        assert_eq!(config.model, "gemini-embedding-001");
        assert_eq!(config.fallback_provider, "voyage");
        assert_eq!(config.fallback_model, "voyage-4-large");
        assert_eq!(config.gemini_api_key, None);
        assert_eq!(config.voyage_api_key, None);
    }

    #[test]
    fn speech_config_uses_smallest_and_inworld_defaults() {
        env::remove_var("SMALLEST_STT_URL");
        env::remove_var("SMALLEST_API_KEY");
        env::remove_var("INWORLD_BASE_URL");
        env::remove_var("INWORLD_API_KEY");
        env::remove_var("INWORLD_TTS_MODEL");
        env::remove_var("INWORLD_TTS_VOICE_ID");

        let config = speech_config();
        assert_eq!(
            config.smallest_stt_url,
            "https://api.smallest.ai/waves/v1/stt/"
        );
        assert_eq!(config.smallest_api_key, None);
        assert_eq!(config.inworld_base_url, "https://api.inworld.ai");
        assert_eq!(config.inworld_api_key, None);
        assert_eq!(config.inworld_tts_model, "inworld-tts-1.5-mini");
        assert_eq!(config.inworld_tts_voice_id, Some("Dennis".to_string()));
    }

    #[test]
    fn cors_defaults_to_known_product_and_local_lab_origins() {
        assert_eq!(
            parse_cors_allowed_origins(None).unwrap(),
            vec![
                "https://antirot.org",
                "https://www.antirot.org",
                "http://localhost:3000",
                "http://127.0.0.1:3000",
                "http://localhost:3001",
                "http://127.0.0.1:3001",
            ]
        );
    }

    #[test]
    fn cors_allowlist_rejects_wildcards_and_deduplicates_origins() {
        assert!(parse_cors_allowed_origins(Some("*")).is_err());
        assert_eq!(
            parse_cors_allowed_origins(Some("https://lab.antirot.org, https://lab.antirot.org"))
                .unwrap(),
            vec!["https://lab.antirot.org"]
        );
    }
}
