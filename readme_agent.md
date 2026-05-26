# Agent Orientation

## Product Context

Antirot is an adaptive behavioral operating system for people with ADHD-like attention drift, hyperfocus, inconsistent executive function, and strong response to challenge-based accountability.

The product should feel like a strict but intelligent sports coach: demanding, skeptical of excuses, emotionally restrained, rarely impressed, but capable of sharp praise when the user performs exceptionally well. Its purpose is not generic positivity. It motivates through identity reinforcement, capability framing, pressure, standards, and memory of past high-performance work.

The system should enforce self-justification more than obedience. When the user wants to do something low-value, the product should make them explain how it serves their primary goals. If they can justify it, the schedule adapts. If not, the friction should interrupt unconscious drift.

## Core Files

- `AGENTS.md`: repository workflow, style, validation, response, and safety rules.
- `product_spec.md`: full product specification for the adaptive behavioral OS.
- `readme_agent.md`: this orientation file for future agents.

## Expected Memory Files

These may not exist yet, but the product architecture expects them:

- `longterm.md`: primary goals, standards, identity framing, motivational triggers, non-negotiables.
- `short.md`: current priorities, temporary goals, active constraints, daily state.
- `behavior.md`: recurring focus patterns, drift tendencies, emotional triggers, effective accountability styles.
- `work.md`: day-wise work summaries, achievements, failures, focus trends, and evidence for future motivation.

Prefer structured summaries and event logs over raw chat history. Cheap models should receive compact, explicit context rather than large unstructured transcripts.

## Product Architecture Notes

- Scheduler and timer behavior should be deterministic code, not model memory.
- The model can decide when a timer or reminder is needed, but code should execute the timer.
- State tracking matters: working, idle, sleeping, vacation, travel, break, deep focus, burnout risk, and similar modes should prevent stupid reminders.
- Reminders should escalate with variation: strict, persistent, disappointed, challenge-based, achievement-based, and only later alarm-like.
- Vacation or relationship time should disable accountability pressure unless the user explicitly opts in.
- Night interactions should reduce anxiety and support closure. Morning interactions can be firmer and more activating.

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

No app scaffold exists yet. When code is added, follow the repo baseline:

- `npx eslint <changed-files>`
- `npx tsc --noEmit`
- relevant focused script or manual flow
- `npm run build` only at meaningful checkpoints

## Gotchas

- Do not make the coach infinitely harsh. The system must allow negotiated breaks, recovery, vacation mode, sleep, and honest constraint changes.
- Avoid fake praise. Praise should be rare, specific, and grounded in work history.
- Fallbacks must never be silent. Use the repository's required fallback log format when adding runtime code.
- For non-trivial manual/product verification, add one crisp verification line to `Done.md`.
