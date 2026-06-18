# Antirot Backend

The Antirot backend is the single Rust API server for mobile apps, the frontend tester, alarms, speech, memory, and coach chat.

## Stack

- Rust
- Axum
- Tokio
- Postgres

## Environment

Start from `env.example.txt` at the repo root.

```bash
ANTIROT_BACKEND_BIND=127.0.0.1:8787
DATABASE_URL=postgres://antirot_backend:change-me@localhost/antirot_backend
ANTIROT_ADMIN_TOKEN=change-me-admin-token
ANTIROT_DEVICE_TOKEN=change-me-device-token
GOOGLE_IOS_CLIENT_ID=973993815360-7q908kk99vtbvv07648prppfdbacqddr.apps.googleusercontent.com
ANTIROT_WORKSPACE_ID=main
GOOGLE_CLOUD_CREDENTIALS={...vertex service account json...}
RUST_LOG=antirot_backend=info,tower_http=info
```

Provider-backed features also use Smallest STT, Inworld TTS, Gemini embeddings, and Voyage fallback embeddings.

## Run Locally

```bash
cd apps/backend
cargo run
```

Health:

```bash
curl http://127.0.0.1:8787/v1/health
```

Chat:

```bash
curl -X POST http://127.0.0.1:8787/v1/chat \
  -H "Authorization: Bearer test-admin-token" \
  -H "Content-Type: application/json" \
  -d '{"message":"Hello Coach!"}'
```

## Endpoints

```text
GET  /health
GET  /v1/health
POST /v1/auth/google
POST /v1/chat
POST /v1/speech/transcribe
POST /v1/speech/synthesize
PUT  /v1/memory/{key}
GET  /v1/memory/{key}
POST /devices/register
POST /alarms
GET  /alarms/pending?deviceId=...
POST /alarms/{alarmId}/{action}
GET  /v1/workspaces/{workspaceId}/devices
```

## Pair A Phone

After the phone signs in with Google, create a short-lived pairing code:

```bash
set -a
. /etc/antirot/backend.env
set +a
/opt/antirot/apps/backend/antirot-backend pair --workspace main --timeout 60
```

The command prints a 6-digit code, waits for the app to claim it, then prints the paired device.

## Test

```bash
cargo check --manifest-path apps/backend/Cargo.toml
cargo test --manifest-path apps/backend/Cargo.toml
npm run test:backend-userflows
npm run test:prompt-snapshots
```

Provider smoke test against a running backend:

```bash
node scripts/test-backend-integrations.mjs \
  --env-file /etc/antirot/backend.env \
  --base-url http://127.0.0.1:8787
```

## VPS

Use [docs/backend-vps-new-user.md](../../docs/backend-vps-new-user.md) for a fresh VPS setup. The systemd service is `antirot-backend`.
