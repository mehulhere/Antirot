use chrono::Utc;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize)]
struct GcpClaims {
    iss: String,
    scope: String,
    aud: String,
    exp: i64,
    iat: i64,
}

#[derive(Deserialize)]
struct GcpTokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct GcpCredentials {
    project_id: String,
    private_key: String,
    client_email: String,
    token_uri: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let creds_json =
        std::env::var("GOOGLE_CLOUD_CREDENTIALS").expect("GOOGLE_CLOUD_CREDENTIALS not set");

    let creds: GcpCredentials = serde_json::from_str(&creds_json)?;

    let iat = Utc::now().timestamp();
    let exp = iat + 3600;

    let claims = GcpClaims {
        iss: creds.client_email.clone(),
        scope: "https://www.googleapis.com/auth/cloud-platform".to_string(),
        aud: creds.token_uri.clone(),
        exp,
        iat,
    };

    let private_key = creds.private_key.replace("\\n", "\n");
    let key = EncodingKey::from_rsa_pem(private_key.as_bytes())?;

    let header = Header::new(Algorithm::RS256);
    let jwt = jsonwebtoken::encode(&header, &claims, &key)?;

    let client = Client::new();
    let res = client
        .post(&creds.token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &jwt),
        ])
        .send()
        .await?;

    if !res.status().is_success() {
        panic!("Token server returned error: {}", res.text().await?);
    }

    let token_resp: GcpTokenResponse = res.json().await?;
    println!("Obtained access token successfully.");

    let models_to_test = vec![
        "google/gemini-3.5-flash",
        "google/gemini-2.5-flash",
        "google/gemini-2.0-flash",
        "google/gemini-1.5-flash",
    ];

    let chat_url = format!(
        "https://aiplatform.googleapis.com/v1/projects/{}/locations/global/endpoints/openapi/chat/completions",
        creds.project_id
    );

    for model in models_to_test {
        println!("Testing model: {} ...", model);
        let payload = json!({
            "model": model,
            "messages": [
                {"role": "user", "content": "Hi"}
            ]
        });

        let response = client
            .post(&chat_url)
            .header(
                "Authorization",
                format!("Bearer {}", token_resp.access_token),
            )
            .json(&payload)
            .send()
            .await?;

        println!("Result for {}: {}", model, response.status());
        if response.status().is_success() {
            println!(
                "SUCCESS! Model {} is available. Body: {}",
                model,
                response.text().await?
            );
            break;
        } else {
            let err_body = response.text().await?;
            if err_body.contains("quota") || err_body.contains("Quota") {
                println!(
                    "SUCCESS (Quota limited)! Model {} is available but quota failed. Body: {}",
                    model, err_body
                );
            } else {
                println!("Error body: {}", err_body);
            }
        }
        println!("--------------------------------------------------");
    }

    Ok(())
}
