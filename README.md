# Antirot

Antirot is an AI-powered ADHD accountability coach. It acts like a strict, intelligent sports coach: high standards, rare praise, sharp reminders, behavioral memory, and pressure that redirects drift into action.

The product has four supported surfaces:

- `apps/backend/`: Rust backend API for auth, chat, alarms, speech, memory, and mobile sync.
- `apps/ios/`: native SwiftUI iOS app with AlarmKit, Screen Time, widgets, chat, speech-to-text, and text-to-speech.
- `apps/android/`: Android APK client for auth, alarms, and coach interaction.
- `website/tester.html`: frontend tester for backend development.

## Architecture

```text
iOS app / Android app / tester frontend
              |
              v
      Antirot Backend (Rust)
              |
              v
 Vertex Gemini 3.5 Flash, Smallest STT, Inworld TTS, Gemini embeddings with Voyage fallback
```

State, alarms, memory maintenance, and provider routing are backend-owned. The LLM receives coaching context, not backend control architecture.

## Backend

```bash
cd apps/backend
cp ../../env.example.txt .env
cargo run
```

Key environment variables:

```text
ANTIROT_BACKEND_BIND=127.0.0.1:8787
DATABASE_URL=postgres://antirot_backend:secret@localhost/antirot_backend
ANTIROT_ADMIN_TOKEN=long-random-admin-token
ANTIROT_DEVICE_TOKEN=long-random-device-token
GOOGLE_CLOUD_CREDENTIALS={...vertex service account json...}
GOOGLE_IOS_CLIENT_ID=your-google-ios-client-id
```

See [apps/backend/README.md](apps/backend/README.md) and [docs/backend-vps-new-user.md](docs/backend-vps-new-user.md).

## Apps

iOS:

```bash
cd apps/ios
brew install xcodegen
xcodegen generate
open Antirot.xcodeproj
```

Android:

```bash
cd apps/android
./gradlew assembleDebug
```

## Frontend Tester

Serve the repo locally and open `website/tester.html`.

```bash
python3 -m http.server 8000
```

Then load `http://localhost:8000/website/tester.html`.

## Testing

```bash
npm run lint
cargo check --manifest-path apps/backend/Cargo.toml
cargo test --manifest-path apps/backend/Cargo.toml
npm run test:backend-userflows
npm run test:prompt-snapshots
```

Provider-backed smoke test against a running backend:

```bash
node scripts/test-backend-integrations.mjs \
  --env-file /etc/antirot/backend.env \
  --base-url http://127.0.0.1:8787
```

## License

Antirot is dual licensed:

- AGPL-3.0-or-later for open source use.
- A separate commercial license for users who do not want to comply with AGPL terms.

See `LICENSE`, `LICENSE-DUAL.md`, and `COMMERCIAL-LICENSE.md`.
