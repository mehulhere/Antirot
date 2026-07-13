# Implementation Handoff Prompts

## 1. Alarm and runtime-state integrity

```text
/make-plan Fix Antirot runtime-state and alarm integrity around one RuntimeStateService::apply_event entry point. Rewrite apps/backend/src/llm.rs:1131-1345 and state-tool call sites at apps/backend/src/llm.rs:1522-1702; route explicit alarm creation at apps/backend/src/routes.rs:706-865 through one AlarmService; replace pending-delivery semantics at apps/backend/src/routes.rs:867-1037 with leases/confirmation/series cancellation; align iOS kinds at apps/ios/AntirotAlarm/Sources/Models.swift:8-16 and Android scheduling. Use PATHFINDER-2026-07-13/01-flowcharts/state-memory-alarms.md. Require transactions, typed errors, canonical enums, series IDs, an outbox, and mobile reconciliation. Do not keep the old direct-insert path behind a flag and do not add string parsing for success.
```

## 2. Coach turn consistency

```text
/make-plan Create one CoachTurnService::run_turn that serializes/idempotently executes a user turn and persists one exact visible_reply. Rewrite apps/backend/src/llm.rs:259-516, typed tool validation at apps/backend/src/llm.rs:1347-1919, and history at apps/backend/src/routes.rs:1500-1530. Use PATHFINDER-2026-07-13/01-flowcharts/prompt-llm.md. Fix newest-N history, curated-reply persistence, bare-Done invariants, malformed tool defaults, loop exhaustion, prompt memory trust boundaries, and per-user concurrency. Do not add phrase blacklists or backend guards for subjective coach quality.
```

## 3. Memory consistency and user-day policy

```text
/make-plan Separate canonical MemoryStore writes from versioned asynchronous MemoryIndexer work. Rewrite apps/backend/src/memory.rs:97-117,200-275,566-815,896-1007; consolidate memory metadata repeated at apps/backend/src/prompt.rs:75-103 and apps/backend/src/llm.rs:1360-1422,1941-1961; add one UserDay timezone policy for apps/backend/src/llm.rs:778-801,1522-1612 and distillation. Use PATHFINDER-2026-07-13/01-flowcharts/state-memory-alarms.md. Require transactional snapshot restore/distillation, stale-index cleanup, accurate embedding provenance, provider timeouts, and completed behavioral-day calculation. Do not make canonical writes fail because derived embeddings fail.
```

## 4. Client/API contract cleanup

```text
/make-plan Make /v1 the canonical Antirot client contract and generate/share runtime-state and alarm-kind models. Fix Android production state at apps/android/app/src/main/java/com/mehulhere/antirot/AntirotApiClient.java:71-77, iOS alarm decoding at apps/ios/AntirotAlarm/Sources/Models.swift:8-16, mixed paths in both native clients, and the legacy tester contract drift. Use PATHFINDER-2026-07-13/01-flowcharts/client-api.md. Keep OS-specific scheduling adapters, but unify IDs, desired-set reconciliation, timeouts, retry semantics, and DTOs. Do not move production clients onto test/admin endpoints.
```

## 5. Prompt reduction after correctness fixes

```text
/make-plan Reduce the Antirot system prompt at apps/backend/src/prompt.rs:157-301 after runtime/turn correctness is fixed. Keep a short invariant core ordered as safety/product boundaries, state invariants, decision policy, then voice; select small runtime-mode fragments for onboarding, work, break, sleep, health, and vacation. Replace first-come memory budgeting at apps/backend/src/prompt.rs:304-340 with priority guarantees based on apps/backend/src/llm.rs:825-922. Preserve broad behavioral guidance and tests without phrase bans, keyword blacklists, or example-specific backend guards. Use PATHFINDER-2026-07-13/01-flowcharts/prompt-llm.md and 03-unified-proposal.md.
```
