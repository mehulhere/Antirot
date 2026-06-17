use std::env;
use std::net::SocketAddr;

use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct Config {
    pub bind: SocketAddr,
    pub database_url: String,
    pub admin_token: String,
    pub device_token: String,
    pub google_allowed_client_ids: Vec<String>,
    pub apns: Option<ApnsConfig>,
    pub memory_embeddings: MemoryEmbeddingConfig,
    pub speech: SpeechConfig,
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
    pub fireworks_base_url: String,
    pub fireworks_audio_base_url: String,
    pub fireworks_api_key: Option<String>,
    pub fireworks_stt_model: String,
    pub async_base_url: String,
    pub async_api_key: Option<String>,
    pub async_tts_model: String,
    pub async_tts_voice_id: Option<String>,
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
        let google_allowed_client_ids = google_allowed_client_ids();
        let apns = apns_config();
        let memory_embeddings = memory_embedding_config();
        let speech = speech_config();

        Ok(Self {
            bind,
            database_url,
            admin_token,
            device_token,
            google_allowed_client_ids,
            apns,
            memory_embeddings,
            speech,
        })
    }
}

fn speech_config() -> SpeechConfig {
    SpeechConfig {
        fireworks_base_url: env::var("FIREWORKS_BASE_URL")
            .unwrap_or_else(|_| "https://api.fireworks.ai/inference/v1".to_string()),
        fireworks_audio_base_url: env::var("FIREWORKS_AUDIO_BASE_URL")
            .unwrap_or_else(|_| "https://audio-prod.api.fireworks.ai/v1".to_string()),
        fireworks_api_key: env::var("FIREWORKS_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty()),
        fireworks_stt_model: env::var("FIREWORKS_STT_MODEL")
            .unwrap_or_else(|_| "whisper-v3".to_string()),
        async_base_url: env::var("ASYNC_BASE_URL")
            .unwrap_or_else(|_| "https://api.async.com".to_string()),
        async_api_key: env::var("ASYNC_API_KEY")
            .or_else(|_| env::var("ASYNC_TTS_API_KEY"))
            .ok()
            .filter(|value| !value.trim().is_empty()),
        async_tts_model: env::var("ASYNC_TTS_MODEL")
            .unwrap_or_else(|_| "async_flash_v1.5".to_string()),
        async_tts_voice_id: env::var("ASYNC_TTS_VOICE_ID")
            .ok()
            .filter(|value| !value.trim().is_empty()),
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
        "production" | "prod" => "https://api.push.apple.com",
        _ => "https://api.sandbox.push.apple.com",
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
    use super::{apns_endpoint_for_environment, memory_embedding_config, speech_config};
    use std::env;

    #[test]
    fn apns_endpoint_uses_sandbox_by_default_for_unknown_values() {
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
            "https://api.sandbox.push.apple.com"
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
    fn speech_config_uses_fireworks_whisper_and_async_flash_defaults() {
        env::remove_var("FIREWORKS_BASE_URL");
        env::remove_var("FIREWORKS_AUDIO_BASE_URL");
        env::remove_var("FIREWORKS_API_KEY");
        env::remove_var("FIREWORKS_STT_MODEL");
        env::remove_var("ASYNC_BASE_URL");
        env::remove_var("ASYNC_API_KEY");
        env::remove_var("ASYNC_TTS_API_KEY");
        env::remove_var("ASYNC_TTS_MODEL");
        env::remove_var("ASYNC_TTS_VOICE_ID");

        let config = speech_config();
        assert_eq!(
            config.fireworks_base_url,
            "https://api.fireworks.ai/inference/v1"
        );
        assert_eq!(
            config.fireworks_audio_base_url,
            "https://audio-prod.api.fireworks.ai/v1"
        );
        assert_eq!(config.fireworks_api_key, None);
        assert_eq!(config.fireworks_stt_model, "whisper-v3");
        assert_eq!(config.async_base_url, "https://api.async.com");
        assert_eq!(config.async_api_key, None);
        assert_eq!(config.async_tts_model, "async_flash_v1.5");
        assert_eq!(config.async_tts_voice_id, None);
    }
}
