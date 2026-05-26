# Antirot iOS App

Antirot is the iOS client for the Antirot coach. The VPS/OpenClaw plugin remains the brain; this app is the local notification, wake acknowledgement, and future Screen Time permission surface.

## Setup

This folder uses XcodeGen so the generated `.xcodeproj` does not need to be committed.

```bash
cd apps/ios
brew install xcodegen
xcodegen generate
open Antirot.xcodeproj
```

In Xcode:

1. Set your Apple team.
2. Change the bundle identifier from `com.mehulhere.Antirot` if needed.
3. Run on a real device for notification and Screen Time behavior.

## Capabilities

Enable these capabilities for the app target:

- Push Notifications, when APNs delivery is added.
- Background Modes -> Remote notifications, when APNs delivery is added.
- App Groups, if sharing data with Screen Time extensions.
- Family Controls, only after Apple grants the entitlement.

Critical Alerts are not part of the MVP. They require Apple approval and should be treated as an entitlement-dependent upgrade.

## MVP Features

- Configure Antirot VPS URL and API token.
- Register device with the VPS.
- Request notification permission.
- Schedule normal and loud local alarm notifications.
- Handle Stop, Snooze, and Need More Time actions.
- Send acknowledgement callbacks to the VPS.
- Request Screen Time authorization when entitlement is available.
- Show clear capability status so the coach knows what this device can actually do.

## Expected API

The Antirot server side should provide:

```text
POST /devices/register
GET /alarms/pending?deviceId=<id>
POST /alarms/{id}/ack
POST /alarms/{id}/snooze
POST /alarms/{id}/clear
POST /usage/recent
```

The app works with local test alarms before the VPS alarm API exists.
