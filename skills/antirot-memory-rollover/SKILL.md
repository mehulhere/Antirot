---
name: antirot-memory-rollover
description: Maintain Antirot behavior memory, miscellaneous side-quest queue, nightly summaries, and task rollover through plugin tools.
user-invocable: false
---

# Antirot Memory And Rollover

Use this skill when the user mentions intrusive thoughts, side quests, small admin tasks, behavioral patterns, drift loops, nightly summaries, task rollover, or midnight planning cleanup.

## Behavior Memory

Use `log_behavior_note` for stable patterns, not one-off noise.

Good behavior notes:

- repeated drift triggers
- accountability tactics that worked
- accountability tactics that backfired
- emotional states that predict avoidance
- recovery patterns after distraction

Do not dump raw conversation into `behavior.md`.

## Misc Queue

Use `add_to_misc_queue` when the user has a side thought that should not interrupt the current task.

Use `list_misc_queue` or `pop_misc_task` when the user needs a short useful break diversion.

Do not let misc tasks replace Level 1 work unless the user can justify it or uses `/override`.

## Nightly Rollover

Use `run_nightly_rollover` at night or during midnight planning cleanup.

It should:

- remove completed checkbox tasks
- carry unfinished checkbox tasks forward
- append new tasks if provided
- write a compact rollover note to `work.md`

Use `write_nightly_summary` when the day has enough evidence to summarize wins, failures, and behavior notes.

Do not manually rewrite `tasks.md`, `work.md`, or `behavior.md` for these flows.
