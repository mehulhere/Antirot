# Backend VPS Setup For A New Linux User

This launches the managed Antirot backend only.

Assumptions:

- Deploy/build Linux user: `antirot`
- Backend runtime Linux user: `antirot-backend`
- App checkout: `/opt/antirot`
- Backend port: `127.0.0.1:8787`
- Public API domain: `api.yourdomain.com`
- Backend env file: `/etc/antirot/backend.env`
- systemd service: `antirot-backend`

## 1. Install Server Packages

Run as `root` or a sudo-capable user:

```bash
apt update
apt install -y git curl build-essential pkg-config libssl-dev postgresql nginx certbot python3-certbot-nginx nodejs npm
```

## 2. Create Linux Users

Create the deploy user if it does not already exist:

```bash
adduser antirot
usermod -aG sudo antirot
```

Create the locked runtime user:

```bash
sudo useradd --system --home /var/lib/antirot-backend --shell /usr/sbin/nologin antirot-backend
sudo mkdir -p /var/lib/antirot-backend /etc/antirot /opt/antirot
sudo chown antirot-backend:antirot-backend /var/lib/antirot-backend
sudo chown antirot:antirot /opt/antirot
```

Placeholder notes:

- `antirot` is the deploy user you log in as.
- `antirot-backend` is a locked system user for systemd. Do not log in as this user.
- These commands need `sudo` when run from the `antirot` user.
- If `useradd` says the user already exists, continue with the `mkdir` and `chown` commands.

## 3. Install Rust For The Deploy User

```bash
su - antirot
curl https://sh.rustup.rs -sSf | sh
. "$HOME/.cargo/env"
cargo --version
```

## 4. Clone And Build Backend

Fresh setup:

```bash
cd /opt
sudo rm -rf /opt/antirot
sudo chown antirot:antirot /opt
git clone https://github.com/mehulhere/Antirot.git antirot
cd /opt/antirot

cargo build --release --manifest-path apps/backend/Cargo.toml
cp apps/backend/target/release/antirot-backend apps/backend/antirot-backend
```

Placeholder notes:

- The repo URL is already filled in: `https://github.com/mehulhere/Antirot.git`.
- Run `sudo rm -rf /opt/antirot` only when you intentionally want a fresh checkout.

## 5. Create Postgres Database

```bash
sudo -u postgres createuser antirot_backend
sudo -u postgres createdb antirot_backend -O antirot_backend
sudo -u postgres psql -c "ALTER USER antirot_backend WITH PASSWORD 'CHANGE_DB_PASSWORD';"
```

Placeholder notes:

- Replace `CHANGE_DB_PASSWORD` with a long random database password.
- Reuse the exact same password in `DATABASE_URL`.
- If the role or database already exists, continue with the password command.

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

GOOGLE_CLOUD_CREDENTIALS=PASTE_VERTEX_SERVICE_ACCOUNT_JSON_ON_ONE_LINE

SMALLEST_STT_URL=https://api.smallest.ai/waves/v1/stt/
SMALLEST_API_KEY=YOUR_SMALLEST_KEY

INWORLD_BASE_URL=https://api.inworld.ai
INWORLD_API_KEY=YOUR_INWORLD_KEY
INWORLD_TTS_MODEL=inworld-tts-1.5-mini
INWORLD_TTS_VOICE_ID=Dennis

ANTIROT_MEMORY_EMBEDDING_MODEL=gemini-embedding-001
ANTIROT_MEMORY_EMBEDDING_FALLBACK_MODEL=voyage-4-large
ANTIROT_MEMORY_GEMINI_API_KEY=YOUR_GEMINI_KEY
ANTIROT_MEMORY_VOYAGE_API_KEY=YOUR_VOYAGE_KEY

RUST_LOG=antirot_backend=info,tower_http=info
```

Placeholder notes:

- Replace `CHANGE_DB_PASSWORD` with the Postgres password from step 5.
- Replace `CHANGE_LONG_ADMIN_TOKEN` with a long random admin token.
- Replace `CHANGE_LONG_DEVICE_TOKEN` with a different long random device token.
- Replace `PASTE_VERTEX_SERVICE_ACCOUNT_JSON_ON_ONE_LINE` with the full Google Vertex service-account JSON content. This is required for coach chat.
- Replace `YOUR_SMALLEST_KEY` with the Smallest API key for Pulse HTTP speech-to-text.
- Replace `YOUR_INWORLD_KEY` with the Inworld API key for streaming text-to-speech.
- `INWORLD_TTS_VOICE_ID` uses Inworld's `Dennis` voice by default; replace it with another Inworld voice id if you want a different coach voice.
- Replace `YOUR_GEMINI_KEY` with the Gemini API key used for memory embeddings.
- Replace `YOUR_VOYAGE_KEY` with the Voyage fallback key.

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

## 7. Install And Start systemd

```bash
sudo cp /opt/antirot/apps/backend/deploy/antirot-backend.service /etc/systemd/system/antirot-backend.service
sudo systemctl daemon-reload
sudo systemctl enable antirot-backend
sudo systemctl restart antirot-backend
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

Enable HTTPS:

```bash
sudo ln -sf /etc/nginx/sites-available/antirot-api /etc/nginx/sites-enabled/antirot-api
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
curl https://api.yourdomain.com/v1/health
```

Chat:

```bash
curl -X POST https://api.yourdomain.com/v1/chat \
  -H "Authorization: Bearer CHANGE_LONG_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"message":"Hello Coach!"}'
```

Provider smoke test:

```bash
node scripts/test-backend-integrations.mjs \
  --env-file /etc/antirot/backend.env \
  --base-url https://api.yourdomain.com
```

Placeholder notes:

- Replace `api.yourdomain.com` with the real API domain.
- Replace `CHANGE_LONG_ADMIN_TOKEN` with the exact `ANTIROT_ADMIN_TOKEN`.
- The smoke test checks health, TTS, STT, embeddings, and coach chat.
- If TTS is not configured yet, pass a real speech file to still test STT: `--audio-file voice.m4a`.

## 10. Updating The Backend Later

GitHub Actions can deploy backend changes automatically through `.github/workflows/deploy-backend-vps.yml`.

Add these GitHub repository settings:

- Secret `ANTIROT_VPS_SSH_KEY`: private SSH key that can log in as the deploy user.
- Variable `ANTIROT_VPS_HOST`: VPS hostname, for example `antirot.org`.
- Variable `ANTIROT_VPS_USER`: deploy Linux user, usually `antirot`.

Placeholder notes:

- `ANTIROT_VPS_SSH_KEY` must be the private half of a key whose public half is in `/home/antirot/.ssh/authorized_keys`.
- `ANTIROT_VPS_HOST` should be the SSH host for the VPS.
- `ANTIROT_VPS_USER` should be `antirot` unless you intentionally use another deploy user.

Allow the deploy user to restart only the backend service without an interactive password:

```bash
sudo visudo -f /etc/sudoers.d/antirot-backend-deploy
```

Add this exact line:

```text
antirot ALL=(root) NOPASSWD: /usr/bin/systemctl restart antirot-backend.service, /usr/bin/systemctl status antirot-backend.service --no-pager --full
```

Placeholder notes:

- Replace `antirot` only if your deploy Linux user has a different name.
- The GitHub Action uses `sudo -n`, so deployment fails instead of hanging if this sudoers rule is missing.
- The Action refuses to deploy over a dirty `/opt/antirot` checkout; keep VPS-only experiments outside that worktree.

Manual update path:

```bash
su - antirot
cd /opt/antirot
git pull origin main
cargo build --release --manifest-path apps/backend/Cargo.toml
cp apps/backend/target/release/antirot-backend apps/backend/antirot-backend
sudo systemctl restart antirot-backend
sudo journalctl -u antirot-backend -n 100 --no-pager
```

## 11. Reset Existing VPS Cleanly

When old services or paths are causing pointless errors, stop them and start clean:

```bash
sudo systemctl disable --now antirot-backend || true
sudo rm -f /etc/systemd/system/antirot-backend.service
sudo systemctl daemon-reload
sudo rm -rf /opt/antirot
```

Then repeat steps 4, 7, and 9.

## Bottom Line

Use `antirot` to clone and build. Use `antirot-backend` to run the backend. Keep API keys in `/etc/antirot/backend.env`. Expose the backend through Nginx HTTPS only.
