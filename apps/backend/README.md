# Antirot Backend

The Antirot backend API server. Handles device registration, Google auth, alarm delivery, APNs push, and user/workspace management.

The backend accepts alarms from admin services (including the optional OpenClaw plugin), stores them in Postgres, exposes pending alarms to mobile clients, and records ack/snooze/scheduled events. It supports the current iOS/Android app paths plus `/v1` aliases.

For iOS, the backend can also send a best-effort APNs background wake. APNs does not schedule the alarm directly; it wakes the app so the app can fetch pending alarms and schedule AlarmKit locally.

## Stack

- Rust
- Axum
- Tokio
- Postgres

## Environment

```bash
ANTIROT_BACKEND_BIND=127.0.0.1:8787
DATABASE_URL=postgres://antirot_backend:change-me@localhost/antirot_backend
ANTIROT_ADMIN_TOKEN=change-me-admin-token
ANTIROT_DEVICE_TOKEN=change-me-device-token
GOOGLE_IOS_CLIENT_ID=973993815360-7q908kk99vtbvv07648prppfdbacqddr.apps.googleusercontent.com
ANTIROT_WORKSPACE_ID=main
RUST_LOG=antirot_bridge=info,tower_http=info
ANTIROT_APNS_ENV=sandbox
ANTIROT_APNS_TEAM_ID=TEAMID1234
ANTIROT_APNS_KEY_ID=KEYID1234
ANTIROT_APNS_PRIVATE_KEY_PATH=/etc/antirot/AuthKey_KEYID1234.p8
ANTIROT_APNS_TOPIC=com.mehulhere.Antirot
```

Use `ANTIROT_ADMIN_TOKEN` for admin operations like creating alarms (used by the OpenClaw plugin or future backend services). Use `ANTIROT_DEVICE_TOKEN` in the iOS/Android app while the app still has a single API-token field.
Set `GOOGLE_IOS_CLIENT_ID` to enable native Google Sign-In at `/v1/auth/google`; the backend verifies the Google ID token and returns a per-device Antirot token.
Set APNs variables to enable VPS-to-iPhone wake delivery. Use `sandbox` for development/sideload builds and `production` for App Store/TestFlight production-signed builds.

## Endpoints

```text
GET  /health
POST /v1/auth/google
POST /v1/pairing/claim
POST /devices/register
POST /alarms
GET  /alarms/pending?deviceId=...
POST /alarms/{alarmId}/{action}
GET  /v1/workspaces/{workspaceId}/devices
```

Each endpoint also exists under `/v1`.

## Pair A Phone

After the phone signs in with Google, create a short-lived pairing code on the VPS:

```bash
set -a
. /etc/antirot/backend.env
set +a
/opt/antirot/apps/bridge/antirot-bridge pair --workspace main --timeout 60
```

The command prints a 6-digit code, waits for the app to claim it, then prints the paired device. The app must enter the code within the timeout. Codes are one-time use, hashed in Postgres, and mapped to the workspace id.

After pairing, admin services can resolve the phone with:

```bash
curl -H "Authorization: Bearer $ANTIROT_ADMIN_TOKEN" \
  http://127.0.0.1:8787/v1/workspaces/main/devices
```

## Create An Alarm

```bash
curl -X POST http://127.0.0.1:8787/alarms \
  -H "Authorization: Bearer $ANTIROT_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "deviceId": "iphone-device-id",
    "kind": "normal_wake",
    "severity": "normal",
    "title": "Wake up",
    "message": "Enough negotiation. Start the day.",
    "fireAt": "2026-05-30T09:00:00Z",
    "hiddenBufferApplied": false,
    "requiresAcknowledgement": true
  }'
```

## Deployment Shape

Recommended:

```text
homepage user/process -> antirot.org
backend user/process  -> api.antirot.org -> 127.0.0.1:8787
```

Run the backend with a dedicated Linux user and `systemd` resource limits so it cannot starve the homepage.

## VPS Deployment With `git push production main`

This deployment assumes:

- VPS SSH user: `antirot`
- VPS IP: `187.77.25.228`
- bare deployment repo: `/srv/git/antirot.git`
- checkout directory: `/opt/antirot`
- backend service user: `antirot-bridge`
- backend port: `127.0.0.1:8787`
- public API domain: `api.antirot.org`

### 1. Install Packages

```bash
ssh antirot@187.77.25.228
sudo apt update
sudo apt install -y git curl build-essential pkg-config libssl-dev postgresql nginx
```

Install Rust for the `antirot` user if it is not installed:

```bash
curl https://sh.rustup.rs -sSf | sh
. "$HOME/.cargo/env"
cargo --version
```

### 2. Create Postgres Database

```bash
sudo -u postgres createuser antirot_bridge
sudo -u postgres createdb antirot_bridge -O antirot_bridge
sudo -u postgres psql -c "ALTER USER antirot_bridge WITH PASSWORD 'CHANGE_DB_PASSWORD';"
```

### 3. Create Bridge User And Env

```bash
sudo useradd --system --home /var/lib/antirot-bridge --shell /usr/sbin/nologin antirot-bridge
sudo mkdir -p /etc/antirot /var/lib/antirot-bridge
sudo chown antirot-bridge:antirot-bridge /var/lib/antirot-bridge
sudo nano /etc/antirot/bridge.env
```

Put:

```bash
ANTIROT_BRIDGE_BIND=127.0.0.1:8787
DATABASE_URL=postgres://antirot_bridge:CHANGE_DB_PASSWORD@localhost/antirot_bridge
ANTIROT_ADMIN_TOKEN=CHANGE_LONG_ADMIN_TOKEN
ANTIROT_DEVICE_TOKEN=CHANGE_LONG_DEVICE_TOKEN
GOOGLE_IOS_CLIENT_ID=973993815360-7q908kk99vtbvv07648prppfdbacqddr.apps.googleusercontent.com
RUST_LOG=antirot_bridge=info,tower_http=info
```

Use `ANTIROT_DEVICE_TOKEN` as the API token in the iOS/Android app. Use `ANTIROT_ADMIN_TOKEN` for admin operations like creating alarms.

### 4. Create Bare Git Repo

```bash
sudo mkdir -p /srv/git /opt/antirot
sudo chown -R antirot:antirot /srv/git /opt/antirot
cd /srv/git
git init --bare antirot.git
```

Create the deploy hook:

```bash
nano /srv/git/antirot.git/hooks/post-receive
```

Paste:

```bash
#!/usr/bin/env bash
set -euo pipefail

APP_DIR="/opt/antirot"
REPO_DIR="/srv/git/antirot.git"

git --work-tree="$APP_DIR" --git-dir="$REPO_DIR" checkout -f main

cd "$APP_DIR/apps/bridge"
"$HOME/.cargo/bin/cargo" build --release

sudo install -m 0755 target/release/antirot-bridge /opt/antirot/apps/bridge/antirot-bridge
sudo systemctl restart antirot-bridge
```

Make it executable:

```bash
chmod +x /srv/git/antirot.git/hooks/post-receive
```

Allow the deploy hook to install and restart the bridge:

```bash
sudo visudo
```

Add:

```text
antirot ALL=(root) NOPASSWD: /usr/bin/systemctl restart antirot-bridge, /usr/bin/install
```

### 5. Install systemd Service

After the first push checks out `/opt/antirot`, install the service:

```bash
sudo cp /opt/antirot/apps/bridge/deploy/antirot-bridge.service /etc/systemd/system/antirot-bridge.service
sudo systemctl daemon-reload
sudo systemctl enable antirot-bridge
```

### 6. Configure Nginx

Create:

```bash
sudo nano /etc/nginx/sites-available/antirot-api
```

Put:

```nginx
server {
    listen 80;
    server_name api.antirot.org;

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

Enable:

```bash
sudo ln -s /etc/nginx/sites-available/antirot-api /etc/nginx/sites-enabled/antirot-api
sudo nginx -t
sudo systemctl reload nginx
```

Add HTTPS after DNS points `api.antirot.org` to the VPS:

```bash
sudo apt install -y certbot python3-certbot-nginx
sudo certbot --nginx -d api.antirot.org
```

### 7. Add Local Production Remote

On your local machine:

```bash
cd ~/Work/Antirot
git remote add production ssh://antirot@187.77.25.228/srv/git/antirot.git
git push production main
```

Future deploys:

```bash
git push production main
```

### 8. Smoke Test

On the VPS:

```bash
curl http://127.0.0.1:8787/health
```

From anywhere after DNS/SSL:

```bash
curl https://api.antirot.org/health
```

Expected:

```json
{"ok":true,"service":"antirot-bridge"}
```

## Notes

This MVP uses pending-fetch as the durable delivery queue. When APNs is configured and an iOS device has registered an APNs token, `POST /alarms` sends a best-effort background push so the app can wake, fetch the pending alarm, and schedule AlarmKit locally. If APNs fails or iOS delays the wake, the alarm remains queued for the app's next poll/open.
