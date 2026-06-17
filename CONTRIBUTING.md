# Contributing

Thanks for helping build Antirot.

## Local Setup

Backend:

```bash
cd apps/backend
cp ../../env.example.txt .env
cargo build
cargo test
```

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

Frontend tester:

```bash
python3 -m http.server 8000
```

Then open `http://localhost:8000/website/tester.html`.

## Validation

Before opening a pull request, run the smallest relevant checks:

```bash
npm run lint
cargo check --manifest-path apps/backend/Cargo.toml
cargo test --manifest-path apps/backend/Cargo.toml
npm run test:backend-userflows
```

For provider-backed smoke tests against a running backend:

```bash
node scripts/test-backend-integrations.mjs \
  --env-file /etc/antirot/backend.env \
  --base-url http://127.0.0.1:8787
```

## Repository Hygiene

Do not commit local secrets or generated runtime artifacts:

- `.env`
- `.antirot/`
- Google client secret JSON files
- uploaded audio
- generated debug transcripts unless they are intentional test fixtures

Use Conventional Commit prefixes such as `feat:`, `fix:`, `docs:`, and `refactor:`.
