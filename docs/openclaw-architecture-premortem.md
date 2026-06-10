# OpenClaw Architecture Premortem For Antirot

## Goal

Use OpenClaw's battle-tested architecture where Antirot is currently weak, without turning Antirot into a generic OpenClaw clone.

Antirot should stay a managed behavioral coach. OpenClaw should influence the runtime shape: prompt layering, memory contracts, context budgeting, inspection, and regression testing.

## What OpenClaw Does Better

### 1. Prompt Assembly Is A Product Surface

OpenClaw treats the system prompt as an owned runtime artifact with explicit layers:

- renderer: pure prompt rendering from explicit inputs
- config resolver: owner display, model overlays, memory citation mode, prompt mode
- runtime adapter: live facts such as tools, sandbox, channel, files, model, and time

Antirot currently builds the prompt inline inside `chat_with_coach`. That makes it harder to test, snapshot, inspect, or evolve safely.

Adopt:

- `PromptContext` struct with typed fields
- `build_coach_system_prompt(context) -> String`
- stable sections with predictable order
- prompt snapshot tests for key states

Do not adopt:

- OpenClaw's generic personal-assistant identity
- channel/group-chat complexity before the iOS app needs it

### 2. Voice Lives Separately From Operating Rules

OpenClaw separates:

- `SOUL.md`: persona, tone, boundaries
- `AGENTS.md`: operating rules and memory workflow
- `USER.md`: user profile
- `MEMORY.md`: durable long-term facts
- `memory/YYYY-MM-DD.md`: daily notes

Antirot currently has voice hardcoded in the backend prompt and richer personality in the optional OpenClaw plugin. `personality.md` is listed as protected in the plugin, but no default file is initialized.

Adopt:

- `personality` memory key as Antirot's equivalent of `SOUL.md`
- `coach_rules` or hardcoded product rules as Antirot's equivalent of `AGENTS.md`
- keep user-specific facts in `longterm`, `shortterm`, `behavior`, `routine`, `sleep`

Do not let users freely turn Antirot into a different product personality. Personality edits should remain protected and justified.

### 3. Context Budgeting And Reporting

OpenClaw can report what was injected into context, raw vs injected sizes, tool schema overhead, and truncation. Antirot currently injects all major memory fields every turn with no size cap or diagnostic endpoint.

Adopt:

- per-memory max injected chars
- total memory injection budget
- truncation marker visible to the model
- `/v1/debug/context` or test-only context report for local validation
- logs when memory is truncated

The coach should not silently forget because `behavior.md` or `tasks.md` grew too large.

### 4. Memory Has Working And Durable Layers

OpenClaw's durable memory is compact. Daily notes are detailed and searchable, but not injected every turn.

Antirot has a useful split already, but daily logs and summaries are still injected directly, and there is no automatic compaction/promotion workflow.

Adopt:

- `longterm`: compact durable identity/goals
- `behavior`: compact durable patterns
- `work_log_YYYY_MM_DD`: detailed daily work evidence
- `work_summary_YYYY_MM_DD`: compact daily summary
- nightly summarization that promotes only high-signal facts

Later:

- semantic search over logs using Gemini embeddings as the only primary provider
- "dreaming" style promotion from daily notes to durable memory

### 5. Tool Availability Is Separate From Tool Guidance

OpenClaw is explicit that `TOOLS.md` is guidance, not availability. Tool schemas are the source of callable truth.

Antirot should keep this. State should remain backend architecture. The LLM should only know tools and user-facing coach rules, not implementation state machinery.

Adopt:

- tools define allowed state transitions
- prompt says tools must be used for durable changes
- no user-visible state names in replies
- quality tests fail on leaked tool/state internals

### 6. Prompt Drift Is Tested

OpenClaw keeps prompt snapshots for runtime surfaces. Antirot currently has behavioral userflow tests, but not prompt snapshots.

Adopt:

- deterministic prompt snapshot tests for onboarding, idle, working, break, sleep, vacation
- snapshot includes section headers and budget metadata
- snapshot excludes volatile timestamps and IDs

## Premortem

### Failure 1: Antirot Becomes Generic

Cause:

- copying OpenClaw's friendly assistant voice directly
- letting `personality.md` override product purpose

Impact:

- coach loses pressure and accountability
- users get mushy productivity advice instead of behavior intervention

Mitigation:

- `personality` controls tone only, not mission
- hard product rules remain higher priority
- test replies for fake positivity and generic encouragement

### Failure 2: Backend State Leaks Into Chat

Cause:

- prompt includes runtime state names
- tool results are passed through verbatim
- debug endpoints become user-facing

Impact:

- user sees implementation details like `idle_alarm`, `start_session`, or `state=working`
- product feels brittle and unpaid-quality

Mitigation:

- keep state out of system prompt
- map tool results to user-facing replies
- quality tests forbid state/tool/alarm internals

### Failure 3: Context Bloat Makes The Coach Forget

Cause:

- all memory fields injected every turn
- long logs accumulate in `tasks`, `behavior`, or daily files
- no context report or truncation warnings

Impact:

- LLM misses active tasks or routine constraints
- tool selection becomes inconsistent
- Gemini quota/cost rises for no product gain

Mitigation:

- strict memory injection budgets
- daily summaries instead of full logs
- context debug report
- nightly memory distillation

### Failure 4: Personality File Becomes A Jailbreak Surface

Cause:

- user edits `personality.md` to disable pressure, alarms, or accountability
- backend treats it as peer to product rules

Impact:

- product contract collapses
- alarms and idle intervention become negotiable in the wrong layer

Mitigation:

- personality file cannot override state/timer policy
- protected edit flow for personality changes
- prompt section labels: "Voice Preferences" below "Non-Negotiable Product Rules"

### Failure 5: Prompt Refactor Breaks Tool Use

Cause:

- moving from inline prompt to layered builder changes wording
- LLM stops calling tools on common user messages

Impact:

- user hears "started" but backend state does not change

Mitigation:

- keep no-LLM tests
- keep LLM userflow tests
- add prompt snapshots
- add fixture conversations that assert tool calls and state after each turn

### Failure 6: OpenClaw Mode Pollutes Standalone Mode

Cause:

- copying OpenClaw workspace concepts too literally
- backend starts mentioning files, commands, slash behavior, or workspace terms

Impact:

- standalone mobile user sees self-hosting concepts they never asked for

Mitigation:

- two prompt modes:
    - `standalone`: managed backend, no OpenClaw terms
    - `openclaw`: workspace/tool/file explicitness allowed
- OpenClaw-only commands stay out of standalone prompt

## Recommended Target Architecture

### Prompt Builder

Create a backend module like:

- `apps/bridge/src/prompt.rs`
- `PromptContext`
- `MemoryInjectionReport`
- `build_coach_system_prompt(context)`

Suggested section order:

1. Identity
2. Non-Negotiable Product Rules
3. Voice Preferences (`personality`)
4. Tool And Memory Rules
5. Current User Context
6. Active Task/Routine/Sleep Evidence
7. Recent Summaries
8. Context Budget Notice

### Memory Model

Add or standardize these memory keys:

- `personality`: Antirot voice preferences, equivalent to OpenClaw `SOUL.md`
- `user_profile`: name, timezone, address preferences, equivalent to OpenClaw `USER.md`
- `longterm`: durable goals and standards
- `shortterm`: current constraints and priorities
- `behavior`: stable drift patterns and tactics
- `routine`: fixed daily allocations
- `tasks`: active linear pipeline
- `sleep`: sleep ledger
- `achievements`: rare evidence for capability framing
- daily logs and summaries as date-keyed memories

### Context Inspection

Expose a test/admin-only report:

- system prompt chars
- memory section raw chars
- memory section injected chars
- truncated sections
- tool count
- model/provider

This should be safe metadata, not a full prompt dump by default.

### Semantic Memory Search

Follow OpenClaw's hybrid retrieval shape, but not its embedding auto-detection.

Antirot provider policy:

- primary provider: Gemini only
- primary model: `gemini-embedding-001`
- fallback provider: Voyage
- fallback model: `voyage-4-large`
- no OpenAI/Copilot/local auto-detection for Antirot memory search
- if both embedding providers are unavailable, degrade to keyword/BM25 search instead of silently disabling memory recall

The backend config is intentionally named for this policy:

- `ANTIROT_MEMORY_EMBEDDING_MODEL`
- `ANTIROT_MEMORY_EMBEDDING_FALLBACK_MODEL`
- `ANTIROT_MEMORY_GEMINI_API_KEY`
- `ANTIROT_MEMORY_VOYAGE_API_KEY`

### Tests

Keep current tests and add:

- prompt snapshot tests
- context budget tests with oversized memory
- personality override tests
- prompt-injection tests asking to reveal state/tool internals
- standalone vs OpenClaw prompt mode tests

## Adoption Priority

1. Extract prompt builder and snapshot it.
2. Add `personality` and `user_profile` memory keys with defaults.
3. Add context budgeting and debug report.
4. Add prompt-quality tests for state/tool leakage.
5. Add nightly distillation from daily logs to summaries and durable memory. Implemented in the backend sleep/idle flow.
6. Add semantic memory search with Gemini embeddings as primary and Voyage as fallback. Implemented with keyword fallback when embedding keys are unavailable.

## Decision

Follow OpenClaw's architecture for runtime hygiene, not its generic assistant identity.

Antirot should copy:

- layered prompt assembly
- workspace-file style memory separation
- explicit context budgets
- context/debug reporting
- prompt snapshots
- durable vs daily memory split

Antirot should not copy:

- generic assistant persona
- unbounded user-editable soul overriding product contract
- OpenClaw workspace terminology in standalone mode
- group-chat/channel behavior before product need
