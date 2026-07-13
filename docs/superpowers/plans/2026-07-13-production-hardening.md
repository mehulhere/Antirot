# Antirot Production Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make coach turns, runtime state, alarms, memory, and mobile contracts production-safe under malformed input, concurrency, partial failure, and client retries.

**Architecture:** Preserve the Rust backend as the single authority. Replace prose-string outcomes and scattered writes with typed validation and transactional operations; persist one visible turn; use canonical alarm generations and retryable delivery; isolate derived memory indexing from canonical writes; apply one user-local day policy.

**Tech Stack:** Rust/Axum/Tokio/PostgreSQL, Swift, Java/Android, TypeScript/Next.js, Node regression scripts.

## Global Constraints

- Preserve all pre-existing working-tree changes; never reset or overwrite unrelated edits.
- TypeScript uses 4-space indentation and semicolons.
- No phrase bans, keyword blacklists, or backend guards for subjective LLM quality.
- Fallback logs use `🔴 FALLBACK: [what] - Reason: [why] - Impact: [limitation]`.
- Tests must demonstrate RED before production changes and GREEN afterward.
- Do not commit `.env`, credentials, generated provider output, or debug artifacts.
- Do not create commits during this plan unless the user separately requests them; the starting tree is intentionally dirty.

---

### Task 1: Coach-turn correctness and typed tool outcomes

**Files:**
- Modify: `apps/backend/src/llm.rs`
- Modify: `apps/backend/src/routes.rs`
- Modify: `apps/backend/src/models.rs`
- Modify: `apps/backend/src/prompt.rs`
- Modify: `apps/backend/sql/001_init.sql`
- Modify: `apps/backend/tests/fixtures/prompts/backend.txt`
- Modify: `scripts/test-backend-userflows-llm.mjs`
- Modify: `scripts/test-backend-userflows-no-llm.mjs`

**Interfaces:**
- Produces: strict tool argument decoding; `ToolOutcome` with typed success/failure; exact persisted visible replies; newest-N history; safe context delimiters; explicit loop/provider failure; per-user turn serialization and request bounds.

- [ ] Write failing Rust tests proving malformed/missing tool arguments cannot produce actions, zero-minute `end_session` does not transition, curated visible reply is selected as the persisted reply, newest-N history query ordering is correct, and reasoning-prefixed malformed output is not returned.
- [ ] Run focused tests and record the expected failures caused by current defaults/string outcomes/history order.
- [ ] Add a forward-only migration for a first-class visible turn/reply representation or minimally distinguish internal tool messages from the committed visible assistant message.
- [ ] Implement strict typed inputs with explicit bounds for every tool; invalid JSON or missing required values returns a typed error before acquiring a database client or writing.
- [ ] Replace `Success:` parsing with typed `ToolOutcome`; ensure failure propagates through orchestration.
- [ ] Persist exactly one final visible reply after model/tool orchestration and make both model context and `/chat/history` consume it.
- [ ] Select the newest 20/100 turns using a descending subquery and reorder ascending with deterministic `(created_at,id)` ordering.
- [ ] Add per-user turn serialization/idempotency, non-empty and maximum message validation, and chat rate limits.
- [ ] Delimit memory sections as untrusted evidence; put product/safety boundaries above voice; guarantee runtime/current-task/today-log context budgets.
- [ ] Make provider response-shape errors and five-loop exhaustion explicit logged failures.
- [ ] Run focused Rust and Node tests until GREEN, then run prompt snapshots, backend userflows, `cargo check`, and targeted ESLint.
- [ ] Review changed code for string-based success parsing, silent defaults, duplicated visible-reply paths, and unbounded chat input.

### Task 2: Atomic runtime events and reconcileable alarms

**Files:**
- Modify: `apps/backend/src/llm.rs`
- Modify: `apps/backend/src/routes.rs`
- Modify: `apps/backend/src/models.rs`
- Modify: `apps/backend/sql/001_init.sql`
- Modify: `apps/ios/AntirotAlarm/Sources/Models.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/APIClient.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/AlarmCenter.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/CoachViewModel.swift`
- Modify: `apps/android/app/src/main/java/com/mehulhere/antirot/AlarmJob.java`
- Modify: `apps/android/app/src/main/java/com/mehulhere/antirot/AntirotApiClient.java`
- Modify: `apps/android/app/src/main/java/com/mehulhere/antirot/AlarmScheduler.java`
- Modify: `apps/android/app/src/main/java/com/mehulhere/antirot/MainActivity.java`
- Create or modify: focused Rust/Node contract tests under `apps/backend/src/` and `scripts/`

**Interfaces:**
- Consumes: typed `ToolOutcome` from Task 1.
- Produces: canonical alarm-kind contract; transactional `apply_runtime_event`; alarm `seriesId`/generation; retryable lease/confirmation/cancellation reconciliation; APNs outbox/wake after commit.

- [ ] Write failing tests for canonical kind serialization, iOS acceptance of state-generated kinds, transition rollback, failure propagation, lease retry, series acknowledgement cleanup, and Android production-state endpoint usage.
- [ ] Run tests and confirm current enum drift, non-atomic behavior, and test-endpoint usage fail them.
- [ ] Add schema fields/indexes for alarm series/generation, delivery lease, and outbox state using idempotent migrations.
- [ ] Implement one transactional runtime event operation that writes ledger, replaces alarm generation, and upserts runtime state; propagate typed failure.
- [ ] Route explicit alarm creation and state alarm series through one persistence path; enqueue APNs wake only after commit.
- [ ] Change pending fetch from terminal delivery to leases; add bulk/local scheduling confirmation and cancellation tombstones or desired-generation reconciliation.
- [ ] Make acknowledgement cancel the full generation; persist client scheduled IDs and cancel obsolete local alarms.
- [ ] Align Swift/Java alarm kinds while tolerating unknown future values at the transport boundary.
- [ ] Reconcile alarms immediately after state-changing chat responses; preserve OS-specific AlarmKit/notification/AlarmManager adapters.
- [ ] Move Android runtime refresh to `/v1/state`, parse chat runtime state, add endpoint-specific chat timeout, and surface/retry alarm-action failures.
- [ ] Run focused tests GREEN, then Rust tests/check, Node boundary tests, available Android build, and source-level iOS contract tests.
- [ ] Review for direct alarm inserts, raw state upserts outside initialization/tests, at-most-once delivery, and uncancelled generation siblings.

### Task 3: Canonical memory, safe restore/indexing, and user-local days

**Files:**
- Modify: `apps/backend/src/memory.rs`
- Modify: `apps/backend/src/llm.rs`
- Modify: `apps/backend/src/prompt.rs`
- Modify: `apps/backend/src/routes.rs`
- Modify: `apps/backend/src/models.rs`
- Modify: `apps/backend/sql/001_init.sql`
- Modify: `apps/ios/AntirotAlarm/Sources/HomeView.swift`
- Modify: `apps/frontend/app/page.tsx`
- Create or modify: focused Rust/Node tests

**Interfaces:**
- Consumes: transactional runtime reconciliation from Task 2.
- Produces: one memory descriptor registry; canonical-write/derived-index separation; transactional snapshot/distillation operations; correct embedding provenance; persisted IANA timezone and `UserDay` calculation; typed onboarding profile capture.

- [ ] Write failing tests for multi-chunk index freshness, canonical write success when embeddings fail, stale chunk removal after restore, transactional restore semantics, actual fallback provider provenance, completed-day selection around midnight, circular sleep-time mean, and onboarding name/timezone persistence.
- [ ] Run focused tests and confirm failures against current source.
- [ ] Consolidate memory key/file/default/prompt/search/snapshot metadata into one descriptor registry.
- [ ] Make canonical memory commit independent from indexing; version index generations and build/swap safely with bounded provider timeouts and lexical fallback.
- [ ] Restore memories and runtime atomically at the canonical layer; remove stale derived generations and reconcile runtime alarms through Task 2.
- [ ] Lock and transactionally commit distillation summary, durable append, and marker; isolate per-user worker failures.
- [ ] Persist validated IANA timezone through a typed onboarding/profile API; replace the hidden prose marker and duplicate first-message ownership.
- [ ] Use one user-local day helper for logs, prompt context, stats, sleep, and completed-day distillation.
- [ ] Fix completed-sleep sample counting and circular sleep-start averaging.
- [ ] Run focused tests GREEN, then full Rust tests/check, prompt snapshots, backend userflows, frontend type/lint checks.
- [ ] Review for synchronous provider dependency in canonical writes, raw UTC today keys, stale chunk visibility, and duplicate memory registries.

### Task 4: Production contract cleanup and full-system verification

**Files:**
- Modify: `apps/backend/src/routes.rs`
- Modify: native API clients as required
- Modify: `website/tester.html` or document its retirement
- Modify: `env.example.txt`
- Modify: `apps/backend/README.md`
- Modify: `Done.md`
- Modify or create: security, contract, and integration scripts under `scripts/`

**Interfaces:**
- Consumes: canonical turn, runtime/alarm, memory, and timezone contracts from Tasks 1-3.
- Produces: `/v1`-only production clients, measured legacy aliases, documented migrations/configuration, complete verification evidence and manual-device checklist.

- [ ] Write failing boundary tests for mixed unversioned production paths, production clients referencing test endpoints/admin tokens, silent fallbacks, missing limits, and stale legacy tester fields.
- [ ] Run tests and confirm current violations.
- [ ] Migrate all production clients to `/v1`; instrument legacy aliases and retain them only as a documented compatibility window.
- [ ] Retire the static tester or make it a thin canonical-contract client without hardcoded credentials or markdown state inference.
- [ ] Ensure every provider/client fallback is logged and exposed through diagnostics without leaking secrets.
- [ ] Update environment example and backend documentation for new schema/workers/limits without real credentials.
- [ ] Add one crisp manual verification line to `Done.md` covering physical iOS/Android alarm reconciliation and push behavior.
- [ ] Run `npx eslint <changed-js-ts-files>`, `npx tsc --noEmit`, `cargo fmt --check`, `cargo clippy --manifest-path apps/backend/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path apps/backend/Cargo.toml`, deterministic Node suites, frontend build, and available Android build.
- [ ] Run VPS-backed integration/scenario tests when credentials/connectivity are available; record any physical-device-only checks honestly.
- [ ] Conduct final architecture, security, and code-quality reviews; fix all Critical/Important findings and rerun affected/full verification.
