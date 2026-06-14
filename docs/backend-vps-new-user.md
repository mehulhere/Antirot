# Backend-Only VPS Setup For A New Linux User

This launches only the managed Antirot backend. Do not install the OpenClaw plugin path for this setup.

Assumptions:

- Deploy/build Linux user: `antirot`
- Backend runtime Linux user: `antirot-backend`
- App checkout: `/opt/antirot`
- Backend port: `127.0.0.1:8787`
- Public API domain: `api.yourdomain.com`
- Backend env file: `/etc/antirot/backend.env`
- systemd service: `antirot-backend`

The Rust binary is still named `antirot-bridge` in the current crate. That is only the compiled binary name; the deployed service/user/env are backend-only.

## 1. Install Server Packages

Run as `root` or a sudo-capable user:

```bash
apt update
apt install -y git curl build-essential pkg-config libssl-dev postgresql nginx certbot python3-certbot-nginx
```

## 2. Create Linux Users

Create a normal deploy user:

```bash
adduser antirot
usermod -aG sudo antirot
```

Create a locked runtime user for the backend:

```bash
sudo useradd --system --home /var/lib/antirot-backend --shell /usr/sbin/nologin antirot-backend
sudo mkdir -p /var/lib/antirot-backend /etc/antirot /opt/antirot
sudo chown antirot-backend:antirot-backend /var/lib/antirot-backend
sudo chown antirot:antirot /opt/antirot
```

Placeholder notes:

- `antirot` is the deploy user you are logged in as.
- `antirot-backend` is a new locked system user. Do not log in as this user.
- If `useradd` says the user already exists, continue with the `mkdir` and `chown` commands.
- These commands need `sudo` when run from the `antirot` user.

## 3. Install Rust For The Deploy User

```bash
su - antirot
curl https://sh.rustup.rs -sSf | sh
. "$HOME/.cargo/env"
cargo --version
```

## 4. Clone And Build Backend

Still as `antirot`:

```bash
cd /opt
rm -rf /opt/antirot
git clone https://github.com/mehulhere/Antirot.git antirot
cd /opt/antirot

cargo build --release --manifest-path apps/bridge/Cargo.toml
cp apps/bridge/target/release/antirot-bridge apps/bridge/antirot-bridge
```

Placeholder notes:

- The repo URL is already filled in: `https://github.com/mehulhere/Antirot.git`.
- Do not type `YOUR_REPO_URL`; that was an older placeholder.
- Run `rm -rf /opt/antirot` only when `/opt/antirot` is not a real git checkout or you intentionally want a fresh clone.
- If removing `/opt/antirot` says permission denied, run `sudo rm -rf /opt/antirot`, then `sudo chown antirot:antirot /opt`.

## 5. Create Postgres Database

Run as a sudo-capable user:

```bash
sudo -u postgres createuser antirot_backend
sudo -u postgres createdb antirot_backend -O antirot_backend
sudo -u postgres psql -c "ALTER USER antirot_backend WITH PASSWORD 'CHANGE_DB_PASSWORD';"
```

Placeholder notes:

- Replace `CHANGE_DB_PASSWORD` with a long random database password.
- Reuse the exact same password in `DATABASE_URL` in `/etc/antirot/backend.env`.
- If `createuser` or `createdb` says the role/database already exists, continue with the password command.

## 6. Create Backend Environment

```bash
sudo nano /etc/antirot/backend.env
```

Use this shape:

```bash
ANTIROT_BACKEND_BIND=127.0.0.1:8787
DATABASE_URL=postgres://antirot_backend:CHANGE_DB_PASSWORD@localhost/antirot_backend
ANTIROT_ADMIN_TOKEN=CHANGE_LONG_ADMIN_TOKEN
ANTIROT_DEVICE_TOKEN=CHANGE_LONG_DEVICE_TOKEN
GOOGLE_IOS_CLIENT_ID=973993815360-7q908kk99vtbvv07648prppfdbacqddr.apps.googleusercontent.com
ANTIROT_WORKSPACE_ID=main

FIREWORKS_BASE_URL=https://api.fireworks.ai/inference/v1
FIREWORKS_AUDIO_BASE_URL=https://audio-prod.api.fireworks.ai/v1
FIREWORKS_API_KEY=YOUR_FIREWORKS_KEY
FIREWORKS_STT_MODEL=whisper-v3

ASYNC_BASE_URL=https://api.async.com
ASYNC_API_KEY=YOUR_ASYNC_KEY
ASYNC_TTS_MODEL=async_flash_v1.5
ASYNC_TTS_VOICE_ID=YOUR_ASYNC_VOICE_ID

ANTIROT_MEMORY_EMBEDDING_MODEL=gemini-embedding-001
ANTIROT_MEMORY_EMBEDDING_FALLBACK_MODEL=voyage-4-large
ANTIROT_MEMORY_GEMINI_API_KEY=YOUR_GEMINI_KEY
ANTIROT_MEMORY_VOYAGE_API_KEY=YOUR_VOYAGE_KEY

RUST_LOG=antirot_bridge=info,tower_http=info
```

Placeholder notes:

- Replace `CHANGE_DB_PASSWORD` with the Postgres password from step 5.
- Replace `CHANGE_LONG_ADMIN_TOKEN` with a long random admin token. Keep it private.
- Replace `CHANGE_LONG_DEVICE_TOKEN` with a different long random device token. Keep it private.
- Replace `YOUR_FIREWORKS_KEY` with the Fireworks API key for Whisper speech-to-text.
- Replace `YOUR_ASYNC_KEY` with the Async API key for text-to-speech.
- Replace `YOUR_ASYNC_VOICE_ID` with an Async voice id. TTS will fail until this is real.
- Replace `YOUR_GEMINI_KEY` with the Gemini API key used for memory embeddings.
- Replace `YOUR_VOYAGE_KEY` with the Voyage fallback key. Leave it blank only if fallback embeddings are intentionally disabled.
- `GOOGLE_IOS_CLIENT_ID` is already filled for the current iOS app.
- `api.yourdomain.com` does not go in this env block unless you later add a public URL variable.

Lock it down:

```bash
sudo chmod 600 /etc/antirot/backend.env
sudo chown root:antirot-backend /etc/antirot/backend.env
```

Check what is still missing:

```bash
cd /opt/antirot
node scripts/check-env.mjs /etc/antirot/backend.env env.example.txt
```

## 7. Install systemd Service

```bash
sudo cp /opt/antirot/apps/bridge/deploy/antirot-backend.service /etc/systemd/system/antirot-backend.service
sudo systemctl daemon-reload
sudo systemctl enable antirot-backend
sudo systemctl start antirot-backend
sudo systemctl status antirot-backend --no-pager
```

Logs:

```bash
sudo journalctl -u antirot-backend -n 100 --no-pager
```

## 8. Configure Nginx

```bash
sudo nano /etc/nginx/sites-available/antirot-api
```

Paste:

```nginx
server {
    server_name api.yourdomain.com;

    location / {
        proxy_pass http://127.0.0.1:8787;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

Enable it:

```bash
sudo ln -s /etc/nginx/sites-available/antirot-api /etc/nginx/sites-enabled/antirot-api
sudo nginx -t
sudo systemctl reload nginx
sudo certbot --nginx -d api.yourdomain.com
```

Placeholder notes:

- Replace every `api.yourdomain.com` with the real API domain, for example `api.antirot.org`.
- Create the DNS `A` record for that domain before running `certbot`.

## 9. Verify Backend

Health:

```bash
curl https://api.yourdomain.com/health
```

Placeholder notes:

- Replace `api.yourdomain.com` with the real API domain.

Expected:

```json
{"ok":true,"service":"antirot-backend"}
```

Chat:

```bash
curl -X POST https://api.yourdomain.com/v1/chat \
  -H "Authorization: Bearer CHANGE_LONG_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"message":"Hello Coach!"}'
```

Placeholder notes:

- Replace `api.yourdomain.com` with the real API domain.
- Replace `CHANGE_LONG_ADMIN_TOKEN` with the exact `ANTIROT_ADMIN_TOKEN` from `/etc/antirot/backend.env`.

Expected shape:

```json
{"ok":true,"reply":"..."}
```

Speech-to-text needs a real audio file:

```bash
curl -X POST https://api.yourdomain.com/v1/speech/transcribe \
  -H "Authorization: Bearer CHANGE_LONG_ADMIN_TOKEN" \
  -F "file=@voice.m4a;type=audio/mp4"
```

Placeholder notes:

- Replace `api.yourdomain.com` with the real API domain.
- Replace `CHANGE_LONG_ADMIN_TOKEN` with the exact `ANTIROT_ADMIN_TOKEN`.
- Replace `voice.m4a` with the path to a real local audio file on the machine running `curl`.

Expected shape:

```json
{"ok":true,"text":"..."}
```

## 10. Updating The Backend Later

```bash
su - antirot
cd /opt/antirot
git pull
cargo build --release --manifest-path apps/bridge/Cargo.toml
cp apps/bridge/target/release/antirot-bridge apps/bridge/antirot-bridge
sudo systemctl restart antirot-backend
sudo journalctl -u antirot-backend -n 100 --no-pager
```

## Bottom Line

Use `antirot` only to clone/build. Use `antirot-backend` only to run the backend. Keep API keys in `/etc/antirot/backend.env`. Expose the backend through Nginx HTTPS only.
