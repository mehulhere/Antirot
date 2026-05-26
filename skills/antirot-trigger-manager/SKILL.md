---
name: antirot-trigger-manager
description: Keep Antirot timers, reminders, alarms, and active daily triggers organized through Antirot tools only.
user-invocable: false
---

# Antirot Trigger Manager

Use this skill whenever a user talks about active timers, reminders, alarms, finishing early, waking early, needing more time, or a callback firing.

## Rule

Never call OpenClaw cron interfaces directly. Use Antirot tools only:

- `list_active_triggers`
- `clear_active_trigger`
- `reschedule_trigger`
- `set_state_timer`
- `start_routine`
- `start_session`
- `start_sleep`
- `log_wake`

## Active Trigger Check

Before acting on any timer, reminder, wake, alarm, or overdue callback, call `list_active_triggers`.

- If the matching trigger is active, continue with the appropriate action.
- If the matching trigger is not active, ignore the callback as stale.
- Midnight planning reminders do not count as daily task triggers.

## Early Completion

If the user finishes early, wakes early, cancels a break, or says the reminder is no longer needed:

1. Call `list_active_triggers`.
2. Identify the matching trigger by kind and label.
3. Call `clear_active_trigger`.
4. Do not mention exact remaining time.

## More Time

If the user says they need more time:

1. Call `list_active_triggers`.
2. Identify the matching trigger.
3. If the extension is discretionary, ask why.
4. Call `reschedule_trigger`.
5. Say a hidden buffer was added. Do not reveal the exact new callback time.

## User Wording

Accepted early-completion signals include:

- "done"
- "finished"
- "I woke up"
- "good morning"
- "I am back"
- "task is over"
- "break over"
- "cancel that"

Accepted extension signals include:

- "I need more time"
- "extend it"
- "not done yet"
- "give me a bit"
- "running late"

Keep this mechanical. The coach can be moody, but trigger state must stay clean.
