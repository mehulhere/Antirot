# Antirot Bridge

Low-resource Rust alarm forwarding bridge for Antirot mobile clients.

The bridge accepts alarms from OpenClaw or future Antirot services, stores them in Postgres, exposes pending alarms to mobile clients, and records ack/snooze/scheduled events. It supports the current iOS/Android app paths plus `/v1` aliases.

## Stack

- Rust
- Axum
- Tokio
- Postgres

## Environment

```bash
ANTIROT_BRIDGE_BIND=127.0.0.1:8787
DATABASE_URL=postgres://antirot_bridge:change-me@localhost/antirot_bridge
ANTIROT_ADMIN_TOKEN=change-me-admin-token
ANTIROT_DEVICE_TOKEN=change-me-device-token
RUST_LOG=antirot_bridge=info,tower_http=info
```

Use `ANTIROT_ADMIN_TOKEN` from the OpenClaw plugin or future backend when creating alarms. Use `ANTIROT_DEVICE_TOKEN` in the iOS/Android app while the app still has a single API-token field.

## Endpoints

```text
GET  /health
POST /devices/register
POST /alarms
GET  /alarms/pending?deviceId=...
POST /alarms/{alarmId}/{action}
```

Each endpoint also exists under `/v1`.

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
bridge user/process   -> api.antirot.org -> 127.0.0.1:8787
```

Run the bridge with a dedicated Linux user and `systemd` resource limits so it cannot starve the homepage.

## Notes

This MVP uses pending-fetch delivery because the current iOS/Android apps already support it. APNs/FCM push delivery should be added later behind the same `POST /alarms` path, using the device push fields already accepted by `/devices/register`.
