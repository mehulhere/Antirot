# Antirot Android App

Antirot is the Android client for the Antirot coach. The app connects to the Antirot backend (`api.antirot.org`) for auth, alarms, speech, and coach intelligence.

## Setup

Open `apps/android` in Android Studio.

Minimum useful manual test:

1. Run the app on a real Android phone.
2. Grant notification permission.
3. Allow exact alarms if Android prompts.
4. Tap "Schedule normal test alarm".
5. Tap "Schedule loud test alarm".
6. Test "I'm awake", "Snooze", and "Need more time".
7. Tap "Speak", talk for at least 10 seconds, pause, and verify the transcript is sent to coach chat.
8. Tap "Refresh state options" and verify coach chips change with idle, working, break, sleeping, vacation, or unknown state.
9. Open usage-access settings and grant Antirot access if you want last-30-minute app usage summaries.

## Build APK

With Android Studio, use:

```text
Build -> Build App Bundle(s) / APK(s) -> Build APK(s)
```

Or run:

```bash
cd apps/android
gradle assembleDebug
```

## Current MVP

- Configure Antirot backend URL and API token.
- Register device with the backend.
- Coach chat with state-aware quick actions.
- Onboarding starts with a name popup; the coach collects the rest conversationally through chat or voice.
- Gentle local voice activity detection before Smallest Pulse transcription through the backend.
- Schedule normal and loud test alarms.
- Alarm screen with acknowledgement actions.
- Callback API client for `ack`, `snooze`, and `clear`.
- Usage-access permission shortcut.
- Last-30-minute usage summary using Android UsageStats.

## Expected API

```text
POST /devices/register
GET /alarms/pending?deviceId=<id>
POST /alarms/{id}/ack
POST /alarms/{id}/snooze
POST /alarms/{id}/clear
POST /usage/recent
POST /v1/chat
POST /v1/speech/transcribe
GET /v1/test/state?userId=<id>&deviceId=<id>
```
