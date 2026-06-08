# Agent Orientation

## Product Context

Antirot is an adaptive behavioral operating system for people with ADHD-like attention drift, hyperfocus, inconsistent executive function, and strong response to challenge-based accountability.

The product should feel like a strict but intelligent sports coach: demanding, skeptical of excuses, emotionally restrained, rarely impressed, but capable of sharp praise when the user performs exceptionally well. Its purpose is not generic positivity. It motivates through identity reinforcement, capability framing, pressure, standards, and memory of past high-performance work.

The system should enforce self-justification more than obedience. When the user wants to do something low-value, the product should make them explain how it serves their primary goals. If they can justify it, the schedule adapts. If not, the friction should interrupt unconscious drift.

## Architecture

Antirot runs as a **standalone iOS app + managed backend**, with an optional OpenClaw plugin path for power users.

### Primary Path (Standalone)
- **iOS App** (`apps/ios/`): Native SwiftUI, AlarmKit, Screen Time, widgets, in-app chat
- **Backend** (`apps/bridge/`): Rust API at `api.antirot.org`, Postgres, APNs, Google auth
- **LLM Routing**: Backend proxies coaching conversations to LLM providers (OpenAI, Gemini, etc.)
- **User Memory**: Per-user behavioral memory stored in Postgres (longterm, shortterm, behavior, work, sleep, tasks, misc)

### Secondary Path (Self-Hosted OpenClaw)
- **OpenClaw Plugin** (`src/`, `openclaw.plugin.json`): Runs on user's VPS
- **iOS App as Relay**: Displays coach messages from OpenClaw, relays responses through the bridge
- **Memory Files**: Stored as markdown files in the OpenClaw workspace directory

## Core Files

- `AGENTS.md`: repository workflow, style, validation, response, and safety rules.
- `product_spec.md`: full product specification for the adaptive behavioral OS.
- `readme_agent.md`: this orientation file for future agents.
- `apps/ios/project.yml`: XcodeGen spec for the iOS app (3 targets: main app, widget, device activity report).
- `apps/bridge/src/`: Rust backend source code.
- `src/`: OpenClaw plugin code (secondary path, maintained but not the primary focus).

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

### Backend (primary)
```bash
cargo check --manifest-path apps/bridge/Cargo.toml
cargo test --manifest-path apps/bridge/Cargo.toml
```

### OpenClaw Plugin (secondary)
```bash
npm run lint
npm run typecheck
npm run build
```

### iOS App
- Build via GitHub Actions → TestFlight (`deploy-ios-testflight.yml`)
- Local: `cd apps/ios && xcodegen generate && open Antirot.xcodeproj`

## Gotchas

- Do not make the coach infinitely harsh. The system must allow negotiated breaks, recovery, vacation mode, sleep, and honest constraint changes.
- Avoid fake praise. Praise should be rare, specific, and grounded in work history.
- Fallbacks must never be silent. Use the repository's required fallback log format when adding runtime code.
- For non-trivial manual/product verification, add one crisp verification line to `Done.md`.
- The iOS app has no Mac requirement for development — all builds happen on GitHub Actions `macos-26` runners.
