# Antirot Production Hardening Design

## Goal

Make the current backend-owned Antirot architecture reliable under real mobile, LLM-provider, database, and concurrency failure modes without replacing its product model or adding speculative infrastructure.

## Invariants

- A user-visible success is emitted only after all canonical side effects commit.
- The exact visible coach reply is the reply persisted and later restored.
- A malformed tool call has zero side effects.
- User-editable memory is context data, never trusted instructions.
- Runtime state and alarm generations change atomically.
- Alarm delivery remains retryable until a client confirms local scheduling.
- One alarm acknowledgement cancels the entire obsolete escalation generation locally and remotely.
- Canonical memory success never depends on an embedding provider.
- User-local day boundaries drive logs, stats, sleep, and distillation.
- Production clients never depend on admin/test endpoints.

## Architecture

Keep Axum, PostgreSQL, native clients, and the provider-compatible chat API. Introduce typed boundaries inside the existing modules first: typed tool outcomes and validated arguments; transactional runtime events; canonical alarm kinds/generations; exact visible turn persistence; canonical memory commits with derived indexing failure isolation; and user timezone/day helpers. Split large modules only where a boundary is stable enough to reduce risk.

## Delivery strategy

1. Stabilize coach turns, history, validation, and reply persistence.
2. Make runtime events and alarms transactional and reconcileable across clients.
3. Make memory restore/indexing/distillation safe and user-timezone aware.
4. Migrate production clients to canonical contracts, close operational gaps, and run complete verification.

## Error handling

Use typed errors internally. Provider, embedding, APNs, and client-scheduling failures must be logged with the repository fallback format and produce an honest retryable state. Never encode success/failure in display strings.

## Testing

Every behavior change follows red-green-refactor. Rust unit tests cover pure parsing/contracts; database-backed tests cover transactions and persistence; JavaScript boundary tests verify native/source contracts where platform builds are unavailable; existing prompt/userflow/security tests remain gates. Live VPS and native-device checks are documented separately when local execution cannot prove OS/provider behavior.

## Scope boundary

This hardening does not redesign the iOS visual interface, add multi-agent coaching, or tune subjective coach style beyond removing instruction duplication and enforcing trust boundaries.
