# Antirot VPS Setup

This guide configures the Antirot Rust bridge, OpenClaw plugin, iOS pairing, and phone alarm escalation on the VPS.

Assumptions:

- VPS user: `antirot`
- App checkout: `/opt/antirot`
- Bridge API: `https://api.antirot.org`
- Bridge systemd service: `antirot-bridge`
- OpenClaw is already installed on the VPS

## 1. Build Current Code

```bash
ssh antirot@187.77.25.228
cd /opt/antirot

git status
git log --oneline -3

npm ci
npm run build
cargo build --release --manifest-path apps/bridge/Cargo.toml
```

## 2. Configure Bridge Environment

```bash
sudo nano /etc/antirot/bridge.env
```

Use this shape:

```bash
ANTIROT_BRIDGE_BIND=127.0.0.1:8787
DATABASE_URL=postgres://antirot_bridge:YOUR_DB_PASSWORD@localhost/antirot_bridge
ANTIROT_ADMIN_TOKEN=YOUR_LONG_ADMIN_TOKEN
ANTIROT_DEVICE_TOKEN=YOUR_LONG_DEVICE_TOKEN
ANTIROT_BRIDGE_URL=https://api.antirot.org
ANTIROT_WORKSPACE_ID=main

GOOGLE_IOS_CLIENT_ID=973993815360-7q908kk99vtbvv07648prppfdbacqddr.apps.googleusercontent.com

ANTIROT_APNS_ENV=sandbox
ANTIROT_APNS_TEAM_ID=YOUR_APPLE_TEAM_ID
ANTIROT_APNS_KEY_ID=YOUR_APNS_KEY_ID
ANTIROT_APNS_PRIVATE_KEY_PATH=/etc/antirot/AuthKey_YOUR_APNS_KEY_ID.p8
ANTIROT_APNS_TOPIC=com.mehulhere.Antirot

RUST_LOG=antirot_bridge=info,tower_http=info
```

Use `ANTIROT_APNS_ENV=sandbox` for development/sideload-style builds. Use `production` for production-signed/TestFlight/App Store builds.

## 3. Install APNs Key

Copy the Apple APNs `.p8` key to the VPS, then run:

```bash
sudo cp ~/AuthKey_YOUR_APNS_KEY_ID.p8 /etc/antirot/
sudo chmod 600 /etc/antirot/AuthKey_YOUR_APNS_KEY_ID.p8
sudo chown antirot-bridge:antirot-bridge /etc/antirot/AuthKey_YOUR_APNS_KEY_ID.p8
```

## 4. Restart And Verify Bridge

```bash
sudo systemctl restart antirot-bridge
sudo systemctl status antirot-bridge --no-pager
curl https://api.antirot.org/health
```

Expected:

```json
{"ok":true,"service":"antirot-bridge"}
```

## 5. Install Or Update OpenClaw Plugin

```bash
openclaw plugins install --link /opt/antirot
openclaw plugins enable antirot
```

## 6. Configure Plugin Bridge Settings

```bash
set -a
. /etc/antirot/bridge.env
set +a

python3 - <<'PY'
import json, os
from pathlib import Path

p = Path.home() / ".openclaw" / "openclaw.json"
data = json.loads(p.read_text())

plugins = data.setdefault("plugins", {}).setdefault("entries", {})
entry = plugins.setdefault("antirot", {})
if isinstance(entry, bool):
    entry = {"enabled": entry}

entry["enabled"] = True
cfg = entry.setdefault("config", {})
cfg["bridgeUrl"] = "https://api.antirot.org"
cfg["bridgeAdminToken"] = os.environ["ANTIROT_ADMIN_TOKEN"]
cfg["bridgeWorkspaceId"] = "main"
cfg["enableCron"] = True

plugins["antirot"] = entry
p.write_text(json.dumps(data, indent=2) + "\n")
PY

openclaw config validate
openclaw gateway restart
```

If paired-device lookup has trouble, add this manually to the same plugin config:

```json
{
    "bridgeDeviceId": "YOUR_IPHONE_DEVICE_ID"
}
```

## 7. Pair The iPhone

The phone must be signed in with Google inside the Antirot iOS app before pairing.

```bash
set -a
. /etc/antirot/bridge.env
set +a

/opt/antirot/apps/bridge/antirot-bridge pair --workspace main --timeout 60
```

Enter the printed 6-digit code in the iOS app.

Verify pairing:

```bash
curl -H "Authorization: Bearer $ANTIROT_ADMIN_TOKEN" \
  https://api.antirot.org/v1/workspaces/main/devices
```

## 8. Test Phone Alarm Escalation

In OpenClaw chat, ask:

```text
Use startAlarm now for test non-response.
```

Expected flow:

1. Plugin calls `startAlarm`.
2. Bridge queues a normal phone alarm for roughly one minute later.
3. Bridge sends APNs wake when APNs is configured and the app has an APNs token.
4. iOS app fetches pending alarms and schedules AlarmKit/local fallback.
5. Plugin arms a hidden 10-minute `alarm_escalation` trigger.
6. LLM decides later whether to clear, repeat `startAlarm`, or call `startLoudAlarm`.

User replies do not automatically stop escalation. The LLM must explicitly call `clear_active_trigger` when it decides the situation is resolved.

## 9. Useful Validation Commands

From `/opt/antirot`:

```bash
npm run build
npm run test:recent
cargo test --manifest-path apps/bridge/Cargo.toml
cargo check --manifest-path apps/bridge/Cargo.toml
```

Real-agent chat test, only when provider API keys are available:

```bash
ANTIROT_RUN_REAL_AGENT_TESTS=1 npm run test:agent-real
```

## 10. Logs

```bash
sudo journalctl -u antirot-bridge -n 100 --no-pager
openclaw logs --tail 100
```

## Bottom Line

Configure bridge env, install APNs key, restart the bridge, configure the OpenClaw plugin with `bridgeAdminToken`, pair the iPhone, then test `startAlarm`.
