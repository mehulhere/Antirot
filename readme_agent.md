# Agent Orientation

## Product Context

Antirot is an adaptive behavioral operating system for people with ADHD-like attention drift, hyperfocus, inconsistent executive function, and strong response to challenge-based accountability.

The product should feel like a strict but intelligent sports coach: demanding, skeptical of excuses, emotionally restrained, rarely impressed, but capable of sharp praise when the user performs exceptionally well. Its purpose is not generic positivity. It motivates through identity reinforcement, capability framing, pressure, standards, and memory of past high-performance work.

The system should enforce self-justification more than obedience. When the user wants to do something low-value, the product should make them explain how it serves their primary goals. If they can justify it, the schedule adapts. If not, the friction should interrupt unconscious drift.

## Architecture

Antirot has one supported product architecture:

- `apps/backend/`: Rust API at `api.antirot.org`; owns auth, chat, alarms, APNs, speech, semantic memory, runtime state, and provider routing.
- `apps/ios/`: native SwiftUI app with AlarmKit, Screen Time, widgets, speech-to-text, text-to-speech, and in-app chat.
- `apps/android/`: Android APK client for auth, alarms, and coach interaction.
- `apps/frontend/`: Next.js Antirot Lab for testing backend and app-like flows before TestFlight/APK builds.
- `website/tester.html`: legacy static frontend tester.

LLM routing for tailored/default users should use Vertex with Gemini 3.5 Flash whenever `GOOGLE_CLOUD_CREDENTIALS` is present. Do not reintroduce alternative runtime architectures unless the user explicitly asks for a new product surface.

For product testing, assume the backend should run on the VPS via `ssh antirot` unless the user explicitly asks for a local backend. The Next.js lab targets the VPS API by default; use local backend URLs only for narrow local debugging.

## VPS First, No Token-Wasting Detours

When debugging backend behavior, deployment, frontend-to-backend failures, LLM prompts, speech endpoints, or anything visible through `api.antirot.org`, use the VPS first. Do not spend time fixing local Postgres, local Docker/Podman, local systemd, or a local backend unless the user explicitly asks for local backend testing.

Use these exact commands because the VPS sudoers rule allows exact command paths:

```bash
ssh antirot
cd /opt/antirot
sudo -n /usr/bin/systemctl status antirot-backend.service --no-pager --full
sudo -n /usr/bin/systemctl restart antirot-backend.service
curl -fsS https://api.antirot.org/v1/health
```

If debugging logs are needed and journal access is configured, use:

```bash
sudo -n /usr/bin/journalctl -u antirot-backend.service -n 120 --no-pager
```

Important: do not replace `/usr/bin/systemctl` with bare `systemctl` in deploy scripts or GitHub Actions. The sudoers rule may reject command forms that do not match exactly.

For manual backend hot deploys on the VPS, non-interactive SSH usually does not have Cargo on `PATH`, and the systemd service runs the binary from `/opt/antirot/apps/backend/antirot-backend`. Use the full Cargo path and install to that service path:

```bash
ssh antirot 'cd /opt/antirot && /home/antirot/.cargo/bin/cargo build --manifest-path apps/backend/Cargo.toml --release && install -m 755 apps/backend/target/release/antirot-backend apps/backend/antirot-backend && sudo -n /usr/bin/systemctl restart antirot-backend.service'
```

If a local test fails with `Local Postgres is not listening` or `failed to get Postgres client`, report that local DB tooling is missing and continue with VPS verification when possible. Do not install or debug local DB/container tooling unless the user asks for it.

Preferred local SSH setup for agents and deploy work:

```sshconfig
Host antirot
    HostName antirot.org
    User antirot
    IdentityFile ~/.ssh/antirot_vps
    IdentitiesOnly yes
```

The key was created with:

```bash
ssh-keygen -t ed25519 -C "antirot-vps" -f ~/.ssh/antirot_vps
ssh-copy-id -i ~/.ssh/antirot_vps.pub antirot@antirot.org
chmod 700 ~/.ssh
chmod 600 ~/.ssh/config ~/.ssh/antirot_vps
chmod 644 ~/.ssh/antirot_vps.pub
```

After setup, prefer simple commands like `ssh antirot`, `scp file antirot:/tmp/`, and `rsync -avz apps/backend/src/ antirot:/opt/antirot/apps/backend/src/`. If the key has a passphrase and repeated prompts get in the way, run `ssh-add ~/.ssh/antirot_vps`.

## Core Files

- `AGENTS.md`: repository workflow, style, validation, response, and safety rules.
- `product_spec.md`: full product specification for the adaptive behavioral OS.
- `readme_agent.md`: this orientation file for future agents.
- `apps/backend/src/`: Rust backend source code.
- `apps/ios/project.yml`: XcodeGen spec for the iOS app.
- `apps/android/`: Android project.
- `apps/frontend/app/`: React/Next.js frontend lab.
- `website/tester.html`: backend/frontend simulator.

## MVP Scope

Keep the first build narrow:

- morning planning
- session tracking
- reminders
- productive vs occupied time
- misc task queue
- nightly summary
- basic strategy adaptation

Do not overbuild multi-agent sophistication before validating the behavioral loop.

## Validation Commands

Backend:

```bash
cargo check --manifest-path apps/backend/Cargo.toml
cargo test --manifest-path apps/backend/Cargo.toml
npm run test:backend-userflows
npm run test:prompt-snapshots
```

Frontend/test utilities:

```bash
npm run lint
npm run frontend:build
node --check scripts/check-env.mjs
node --check scripts/test-backend-integrations.mjs
```

To test the VPS backend from the frontend lab without visible backend settings fields:

```bash
NEXT_PUBLIC_ANTIROT_ADMIN_TOKEN=<admin-token> NEXT_PUBLIC_ANTIROT_DEVICE_TOKEN=<device-token> npm run frontend:dev
```

Use tokens from `/etc/antirot/backend.env` on the VPS. Do not commit real token values.

## LLM Regression Gotchas

Before running long LLM suites, prune stale cases first. Do not keep old tests just because they once passed. Current user-facing behavior is the source of truth, especially for onboarding and coach-loop flows.

Prefer the VPS for LLM/userflow verification:

```bash
ANTIROT_ADMIN_TOKEN="$(ssh antirot 'set -a; . /etc/antirot/backend.env; set +a; printf %s "$ANTIROT_ADMIN_TOKEN"')" \
ANTIROT_BACKEND_URL=https://api.antirot.org \
ANTIROT_RUN_LLM_USERFLOW_TESTS=1 \
npm run test:backend-userflows-llm
```

Use `ANTIROT_LLM_TRANSCRIPT_CACHE_BYPASS=1` only when you truly need a fresh full run. Otherwise resume from `.antirot/llm-regression-progress.json`; provider retries can waste a lot of time.

Repeated LLM red flags to guard in code/tests:

- Hidden reasoning leaks such as `Reasoning Summary`, `Analytical assessment`, or prose about what the assistant "must" do.
- User-facing memory/update chatter such as `profile updated`, `timezone locked`, `personality updated`, `saved fields`, or `baseline parameters`.
- Numbered onboarding forms. First onboarding should feel like Antirot talking, not a questionnaire.
- Broad-goal parroting like "finalize the app" as an executable task. Suggest a specific next task instead.
- Explicit `Start a N minute session on X` must reliably start a session even if the LLM forgets the tool call.
- Do not turn coach judgment into keyword search helpers. Avoidance/fake-prep behavior belongs in prompt guidance and LLM quality tests, not hardcoded phrase detectors like "vibe" or "clean my desk".

When changing prompt behavior, run the relevant backend tests first, deploy to VPS, then run the pruned LLM suite. If a failure is from an obsolete scenario, delete or replace the test. If a failure exposes a real user-facing red flag, add a sanitizer or prompt rule plus a regression assertion.

iOS:

- Build via GitHub Actions TestFlight workflow.
- Local: `cd apps/ios && xcodegen generate && open Antirot.xcodeproj`

Android:

```bash
cd apps/android
gradle assembleDebug
```

No Gradle wrapper is currently checked in, so use Android Studio or a locally installed `gradle` binary.

## Gotchas

- Do not make the coach infinitely harsh. The system must allow negotiated breaks, recovery, vacation mode, sleep, and honest constraint changes.
- Avoid fake praise. Praise should be rare, specific, and grounded in work history.
- Backend state is architecture, not user-facing language.
- Fallbacks must never be silent. Use the repository's required fallback log format when adding runtime code.
- For non-trivial manual/product verification, add one crisp verification line to `Done.md`.
