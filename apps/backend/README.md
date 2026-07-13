# Antirot Backend

The Antirot backend is the single Rust API server for mobile apps, the loopback-only frontend lab, alarms, speech, memory, and coach chat.

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
ANTIROT_ALLOW_ANONYMOUS_SESSIONS=0
ANTIROT_ALLOW_LEGACY_DEVICE_BOOTSTRAP=0
ANTIROT_CORS_ALLOWED_ORIGINS=https://antirot.org,https://www.antirot.org,http://localhost:3000,http://127.0.0.1:3000
GOOGLE_IOS_CLIENT_ID=your-google-ios-client-id.apps.googleusercontent.com
ANTIROT_WORKSPACE_ID=main
GOOGLE_CLOUD_CREDENTIALS={...vertex service account json...}
RUST_LOG=antirot_backend=info,tower_http=info
```

`your-google-ios-client-id.apps.googleusercontent.com` means the OAuth client identifier created for the iOS app. `{...vertex service account json...}` means the complete service-account JSON supplied through the protected backend environment, never a browser variable.

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
  -H "Authorization: Bearer $ANTIROT_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"message":"Hello Coach!"}'
```

## Endpoints

```text
GET  /v1/health
POST /v1/auth/google
POST /v1/chat
POST /v1/speech/transcribe
POST /v1/speech/synthesize
PUT  /v1/memory/{key}
GET  /v1/memory/{key}
POST /v1/devices/register
POST /v1/alarms
GET  /v1/alarms/pending?deviceId=...&reconcile=true&limit=200
POST /v1/alarms/reconcile
POST /v1/alarms/{alarmId}/{action}
GET  /v1/workspaces/{workspaceId}/devices
```

Production clients use `/v1` exclusively. Legacy alias compatibility window ends 2026-10-31. Until removal, unversioned aliases emit a warning and return `x-antirot-legacy-alias` plus the process-local `x-antirot-legacy-hit-count`; use those signals to find remaining callers.

## Database Migrations

Before binding the HTTP listener, startup runs ordered, versioned migrations recorded in `schema_migrations`. A transaction-scoped Postgres advisory lock serializes concurrent backend startups so each version is applied and recorded exactly once.

- Fresh databases apply the v1 baseline followed by ordered v2-v5 hardening migrations.
- Existing installations without a migration ledger are baselined only when the complete required v1 schema exists; partial schemas stop startup with the missing objects listed.
- Once all versions are recorded, later restarts perform no schema work.
- The v2 migration idempotently queues one current-version index job for canonical memory that has no matching active index generation.
- Remote PostgreSQL hosts use verified system-root TLS; only loopback and Unix-socket databases use `NoTls`.

## Limits And Derived Workers

- Coach messages: 12,000 characters; request IDs are required UUIDs; provider concurrency defaults to 12.
- Pending alarm reconciliation: at most 200 rows per fetch.
- Speech transcription: 25 MB per audio file, with 1 MB multipart overhead permitted at the route boundary.
- Speech synthesis text: 1,200 characters; synthesized audio: 10 MB; streaming provider buffer: 15 MB.
- Speech requires an active entitlement, defaults to four concurrent provider calls, and reserves persistent daily STT-byte/TTS-character budgets.
- Canonical memory is capped at 100,000 characters per document and 1,000,000 characters per user. BYOK users use lexical memory search/indexing unless a future explicit semantic-processing consent flow is added.
- The autonomous alarm wake worker scans every 5 seconds and drains at most 50 leased outbox effects per pass. Claims use `FOR UPDATE SKIP LOCKED`; pending and expired APNs wakes retry without requiring a chat, alarm fetch, or other client request.
- Missing APNs configuration, transport failures, and Apple non-success responses leave the wake retryable with bounded exponential backoff; repeated provider failures dead-letter after ten attempts.
- The memory index worker claims durable jobs every 5 seconds with `FOR UPDATE SKIP LOCKED`. Canonical memory remains available through lexical fallback; failures back off, do not block newer work, and dead-letter after five attempts.
- Nightly distillation scans every 5 minutes. Each user failure is isolated and retries on a later scan.

The memory index worker and distillation worker are derived processing only: provider failure never rolls back canonical memory, runtime state, alarms, or visible coach turns.

## Pair A Phone

After the phone signs in with Google, create a short-lived pairing code:

```bash
set -a
. /etc/antirot/backend.env
set +a
/opt/antirot/apps/backend/antirot-backend pair --workspace main --timeout 60
```

The command prints a 128-bit hexadecimal code, waits for the app to claim it, then prints the paired device.

## Required Production Secrets

- `ANTIROT_BYOK_ENCRYPTION_KEY_HEX`: 64 hexadecimal characters representing 32 random bytes. Generate once with `openssl rand -hex 32`; rotating it requires re-encrypting stored BYOK credentials.
- `ANTIROT_VPS_HOST_KEY` (GitHub Actions secret): the reviewed `known_hosts` line for the VPS. Obtain it through a trusted channel and verify its fingerprint before storing it; deploys never use trust-on-first-use.
- Android release secrets: `ANDROID_SIGNING_KEYSTORE_BASE64`, `ANDROID_SIGNING_KEY_ALIAS`, `ANDROID_SIGNING_STORE_PASSWORD`, and `ANDROID_SIGNING_KEY_PASSWORD`. Tag builds publish only after `apksigner` verifies the release APK.

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
