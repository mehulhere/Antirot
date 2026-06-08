# Antirot

Antirot is an AI-powered ADHD accountability coach. It acts as a strict, intelligent sports coach — high standards, rare praise, sharp reminders, and adaptive behavioral strategies.

The system provides:

- Morning planning and session tracking
- Escalating alarm reminders via AlarmKit (iOS 26+)
- Behavioral memory and strategy adaptation
- Screen Time awareness (when authorized by Apple)
- In-app chat with a persistent coaching personality
- Home screen widget showing the current task

## Architecture

Antirot runs as a native iOS app backed by a lightweight Rust API server.

```text
iOS App (SwiftUI)        Antirot Backend (Rust)       LLM Provider
  AlarmKit alarms    <-->   api.antirot.org        <-->  OpenAI / Gemini
  Screen Time               Postgres                    OpenRouter
  Widget                    APNs
  Chat UI                   Auth (Google)
```

**For self-hosted power users**, Antirot also ships an OpenClaw plugin that runs on your own VPS and uses the iOS app as a message relay. See the [Self-Hosted Setup](#self-hosted-with-openclaw) section below.

## iOS App

The native SwiftUI app lives in `apps/ios/`. It uses XcodeGen to generate the Xcode project.

```bash
cd apps/ios
brew install xcodegen
xcodegen generate
open Antirot.xcodeproj
```

See [apps/ios/README.md](apps/ios/README.md) for capabilities, alarm setup, widget usage, and Screen Time.

## Backend

The Rust API server lives in `apps/bridge/`. It handles device registration, alarm delivery, APNs push, Google auth, and user/workspace management.

```bash
cd apps/bridge
cp ../../env.example.txt .env
# Edit .env with your Postgres and APNs credentials
cargo run
```

Required environment:

```text
DATABASE_URL=postgres://antirot_bridge:secret@localhost/antirot_bridge
ANTIROT_ADMIN_TOKEN=long-random-admin-token
ANTIROT_DEVICE_TOKEN=long-random-device-token
ANTIROT_BRIDGE_BIND=127.0.0.1:8787
GOOGLE_IOS_CLIENT_ID=your-google-client-id
```

See [apps/bridge/README.md](apps/bridge/README.md) for the full API, pairing flow, and APNs configuration.

## Website

The landing page lives in `website/`. Served from `antirot.org`.

## CI/CD

GitHub Actions workflows in `.github/workflows/`:

- `deploy-ios-testflight.yml`: Builds and uploads to TestFlight on every push to main.
- `build-ios-ipa.yml`: Builds an unsigned IPA artifact for sideloading.
- `build-android-apk.yml`: Builds the Android APK.

## VPS Deployment

See [setup_VPS.md](setup_VPS.md) for full server setup including the backend, Nginx, APNs keys, and iPhone pairing.

## Self-Hosted with OpenClaw

For power users who want to run the coaching brain on their own VPS using OpenClaw:

```bash
npm install
npm run build
npx openclaw plugins install --link .
npx openclaw plugins enable antirot
npx openclaw gateway restart
```

The iOS app acts as a message relay in this mode — it displays coach messages from your OpenClaw instance and relays your responses back through the bridge.

See [setup_VPS.md](setup_VPS.md) for OpenClaw plugin configuration on your server.

## Testing

```bash
# Plugin lint and typecheck
npm run lint
npm run typecheck
npm run build

# Bridge
cargo test --manifest-path apps/bridge/Cargo.toml

# Alarm escalation flow
npm run test:recent

# Real agent chat (requires provider API keys)
ANTIROT_RUN_REAL_AGENT_TESTS=1 npm run test:agent-real
```

## License

Antirot is dual licensed:

- AGPL-3.0-or-later for open source use.
- A separate commercial license for users who do not want to comply with AGPL terms.

See `LICENSE`, `LICENSE-DUAL.md`, and `COMMERCIAL-LICENSE.md`.
