# Weekly Repository Review - 2026-06-27

## Scope
- Reviewed repository guidance, the agent orientation file, package scripts, TypeScript/ESLint config, backend Rust config/tests, frontend build path, environment validation, backend userflow harness, Android project entrypoint, and recurring TODO/risk markers.
- Baseline commands run before fixes: `npm run lint`, `npx tsc --noEmit`, `npx tsc --noEmit -p apps/frontend/tsconfig.json`, `cargo check --manifest-path apps/backend/Cargo.toml`, `cargo test --manifest-path apps/backend/Cargo.toml`, `npm run frontend:build`, `npm run check:env`, `npm run test:backend-userflows`, Node syntax checks for scripts, and Android wrapper detection.

## Prioritized Findings

| Priority | Finding | Evidence | Status |
| --- | --- | --- | --- |
| P1 | Root TypeScript validation command failed before checking real code. | `npx tsc --noEmit` exited with `TS18003: No inputs were found in config file`. `apps/frontend/tsconfig.json` already owns the actual TS/TSX surface and passes when invoked directly. | Fixed: root `tsconfig.json` now uses a project reference to `apps/frontend`. |
| P1 | Lint failed on a stale unused LLM onboarding helper. | `npm run lint` reported `assertOnboardingQuality` defined but never used in `scripts/test-backend-userflows-llm.mjs`. Later cases use more specific onboarding assertions. | Fixed: removed the dead helper without changing active LLM assertions. |
| P2 | `.env` does not pass repo env validation. | `npm run check:env` reports missing backend keys and placeholder `DATABASE_URL`. `.env` is ignored and contains local values, so this is a local configuration issue rather than a commit-safe source change. | Blocked/local: requires real local or VPS credentials and database choice. |
| P2 | Local backend userflow tests cannot start because Postgres/container runtime is unavailable. | `npm run test:backend-userflows` fails with `Local Postgres is not listening on localhost:5432...`. Agent orientation says not to spend time fixing local DB tooling unless requested. | Blocked/local: use VPS-backed validation or provide/start local Postgres. |
| P3 | Android cannot be built with the documented wrapper command. | No executable `apps/android/gradlew` exists, while root docs and agent orientation pointed to `./gradlew assembleDebug`. | Fixed docs: build instructions now say to use Android Studio or a locally installed `gradle` binary. Adding a checked-in wrapper remains a deliberate toolchain decision. |
| P3 | Frontend production build emits expected VAD/onnx warnings. | `npm run frontend:build` passes but warns that `onnxruntime-web` uses dynamic `require`; import trace starts from `@ricky0123/vad-web`. | Open/monitor: warning is third-party VAD packaging, not a runtime failure. |

## Progress Log
- 2026-06-27: Created baseline review, fixed root TypeScript validation, removed stale unused LLM helper, and aligned Android build docs with the current no-wrapper repo state.

## Validation Results
- Passing after fixes: `npm run lint`, `npx tsc --noEmit`, `node --check scripts/test-backend-userflows-llm.mjs`, `node --check scripts/check-env.mjs`, `node --check scripts/test-backend-integrations.mjs`, `node --check scripts/backend-userflow-test-lib.mjs`, `node --check scripts/test-backend-userflows-no-llm.mjs`, `cargo check --manifest-path apps/backend/Cargo.toml`, `cargo test --manifest-path apps/backend/Cargo.toml`, and `npm run frontend:build`.
- Known warnings: `npm run frontend:build` emits `onnxruntime-web` dynamic `require` warnings through `@ricky0123/vad-web`, and Next reports that the Next.js ESLint plugin is not detected in the flat ESLint config.
- Blocked locally: `npm run check:env` still fails against ignored local `.env` until real local backend/database credentials replace placeholders; `npm run test:backend-userflows` still needs local Postgres/container runtime or a VPS-backed test target.
