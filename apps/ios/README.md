# Antirot iOS App

Antirot is the iOS client for the Antirot coach. The app connects to the Antirot backend (`api.antirot.org`) for auth, alarms, speech, memory, and coach intelligence.

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

## Build An IPA Without A Mac

Use the GitHub Actions workflow:

```text
Actions -> Build iOS IPA -> Run workflow
```

The workflow uploads:

```text
Antirot-unsigned-ipa
```

Download the artifact, unzip it if GitHub wraps it, then install the IPA through SideStore or AltStore. The workflow intentionally creates an unsigned IPA because free Apple-ID sideloading tools sign it on your behalf.

Limitations:

- Free Apple-ID sideloads expire unless refreshed.
- Screen Time and push-notification entitlements will not work through ordinary free sideloading.
- Local notification test alarms should still be the first thing to verify.

## Capabilities

Enable these capabilities for the app target:

- AlarmKit permission is requested inside the app on iOS 26+.
- Push Notifications, when APNs delivery is added.
- Background Modes -> Remote notifications, when APNs delivery is added.
- App Groups, if sharing data with Screen Time extensions.
- Family Controls, only after Apple grants the entitlement.

Critical Alerts are not part of the MVP. They require Apple approval and should be treated as an entitlement-dependent upgrade.

## Real Alarms On iOS 26+

Antirot uses AlarmKit when the app is built with an iOS 26 SDK and running on iOS 26 or newer. AlarmKit is the real iOS alarm path: it can present prominent system alarms instead of ordinary notification-only reminders.

The app falls back to local notifications when AlarmKit is unavailable, such as older iOS versions, older Xcode/iOS SDK builds, or sideloaded builds that were produced without the AlarmKit framework.

In the app:

```text
Request real alarm permission
Schedule normal test alarm
Schedule loud test alarm
```

If the status says AlarmKit is unavailable, rebuild the IPA with an Xcode/iOS SDK version that includes AlarmKit.

## MVP Features

- Voice-first coach home screen with Fireworks Whisper transcription through the backend.
- Quick actions that send normal chat messages instead of bypassing coach policy.
- Done, Start Working, Need Break, Log Work, Good Night, and Awake actions.
- Plan page for routine anchors, state actions, and daily review.
- Async Flash text-to-speech playback through the backend when `ASYNC_TTS_VOICE_ID` is configured.
- Register device with the managed backend.
- Request notification permission.
- Schedule normal and loud local alarm notifications.
- Schedule real AlarmKit alarms on iOS 26+ when available.
- Show the current task in the Antirot iOS widget through shared app-group state.
- Handle Stop, Snooze, and Need More Time actions.
- Send acknowledgement callbacks to the backend.
- Request Screen Time authorization when entitlement is available.
- Show clear capability status so the coach knows what this device can actually do.

## Product Surface

The first build has four pages:

- Coach: Siri-style orb, transcript, microphone, typed fallback, and action chips.
- Plan: routine anchors, state/action buttons, and daily review.
- Alarms: pending alarms, local alarm tests, and notification/AlarmKit checks.
- Settings: account, permissions, widget status, device details, and hidden developer tools.

Buttons and chat intentionally share one backend path. A button sends a short, explicit user message to `/v1/chat`; voice input transcribes through `/v1/speech/transcribe` and then sends the transcript to `/v1/chat`; typed fallback does the same. This keeps state transitions backend-owned and lets the coach challenge or accept the user's intent through the existing tool policy.

Voice provider defaults:

- STT: Fireworks `whisper-v3` via `FIREWORKS_AUDIO_BASE_URL`.
- TTS: Async `async_flash_v1.5` via `/text_to_speech/streaming`.

Do not put provider API keys in the iOS bundle. Keep them on the backend and expose only authenticated Antirot speech endpoints to the app.

## Expected API

The Antirot server side should provide:

```text
POST /devices/register
GET /alarms/pending?deviceId=<id>
POST /alarms/{id}/ack
POST /alarms/{id}/snooze
POST /alarms/{id}/clear
POST /usage/recent
POST /v1/chat
POST /v1/speech/transcribe
POST /v1/speech/synthesize
```

The app works with local test alarms before the backend alarm API is reachable.

## Current Task Widget

The iOS app includes a WidgetKit extension named `Current Task`.

After installing Antirot:

```text
Home Screen -> long press -> Edit -> Add Widget -> Antirot -> Current Task
```

The app updates the widget when an alarm/task is scheduled. The test button `Show current task in widget` writes a sample current task so you can verify the widget before the backend alarm API is reachable.
