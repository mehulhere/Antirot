# Antirot Architecture Audit Scope

Audit date: 2026-07-13

This audit examines the current working tree. The repository already contained uncommitted changes before the audit; no product source files were changed.

## 1. Prompt and LLM orchestration

- Entry points: `apps/backend/src/routes.rs:1484`, `apps/backend/src/llm.rs:149`, `apps/backend/src/prompt.rs:157`
- Core files: `apps/backend/src/prompt.rs`, `apps/backend/src/llm.rs`, `apps/backend/tests/fixtures/prompts/backend.txt`
- Scope: provider selection, chat history, system-prompt assembly, memory injection, tool calls, deterministic replies, and conversation persistence.

## 2. Runtime state, memory, and alarms

- Entry points: `apps/backend/src/llm.rs:1131`, `apps/backend/src/llm.rs:1347`, `apps/backend/src/memory.rs:97`, `apps/backend/src/routes.rs:706`
- Core files: `apps/backend/src/llm.rs`, `apps/backend/src/memory.rs`, `apps/backend/src/routes.rs`, `apps/backend/src/apns.rs`, `apps/backend/sql/001_init.sql`
- Scope: durable action execution, state transitions, alarm creation/delivery, memory indexing/recall, snapshots, and nightly distillation.

## 3. Client and API delivery

- Entry points: `apps/ios/AntirotAlarm/Sources/CoachViewModel.swift:177`, `apps/android/app/src/main/java/com/mehulhere/antirot/MainActivity.java:59`, `apps/frontend/app/page.tsx:444`, `website/tester.html:646`
- Core files: native API clients, native alarm coordinators, Next.js lab, legacy tester, and backend route/auth layers.
- Scope: authentication, chat transport, runtime-state refresh, alarm polling/local scheduling, voice, API versioning, and production/test boundaries.

## Supporting systems

The following are included where they cross one of the three primary flows: identity and device ownership, speech providers, stats/reports, subscriptions, diagnostics, regression tooling, and release checks.
