# Antirot

Antirot is an OpenClaw plugin for strict personal accountability. It behaves like a tough, moody sports coach: high standards, rare praise, sharp reminders, and deterministic state tools so cheap LLMs do not manually rewrite memory or timers.

## Development

```bash
npm install
npm run build
npx openclaw plugins install --link .
npx openclaw plugins enable antirot
npx openclaw gateway restart
npx openclaw plugins inspect antirot --runtime --json
```

## Install From GitHub

```bash
npx openclaw plugins install https://github.com/mehulhere/Antirot.git
npx openclaw plugins enable antirot
npx openclaw gateway restart
npx openclaw plugins inspect antirot --runtime --json
```

Use `--link .` while developing locally. Use the GitHub URL when installing the published plugin on another machine.

## OpenClaw Config

Configure the workspace explicitly when possible:

```json
{
    "plugins": {
        "entries": {
            "antirot": {
                "enabled": true,
                "config": {
                    "workspaceDir": "/absolute/path/to/openclaw/workspace",
                    "openclawCommand": "openclaw",
                    "enableCron": true,
                    "normalAlarmCommand": "paplay /path/to/normal-alarm.wav",
                    "alarmCommand": "paplay /path/to/alarm.wav"
                }
            }
        }
    }
}
```

Sleep is tracked separately in `sleep.md` and `.antirot/sleep_stats.json`. Saying "I am going to sleep" should start sleep mode and schedule a normal wake alarm after required sleep plus a hidden buffer, followed by loud escalation after another hidden buffer if no good morning variant is received. User-facing replies should not pre-tell exact alarm or reminder times.

Daily runtime triggers are tracked in `.antirot/triggers.json`. The agent should inspect, clear, and reschedule triggers only through Antirot tools (`list_active_triggers`, `clear_active_trigger`, `reschedule_trigger`) instead of calling OpenClaw cron directly.

Behavior memory is kept in `behavior.md` and injected into the compact prompt context. Use `log_behavior_note` for stable drift patterns or accountability tactics, and use the misc queue tools (`add_to_misc_queue`, `list_misc_queue`, `pop_misc_task`) to park intrusive thoughts without derailing focus.

Onboarding should happen in chat. The agent should call `get_onboarding_status`, ask simple questions in user language, and save answers through `save_onboarding_answers` instead of telling the user to manually edit `longterm.md`, `shortterm.md`, or `behavior.md`. The user should not have to classify answers as long-term, short-term, or behavior; the agent does that split. The same flow is used later for periodic goal reviews or when priorities change.

Night cleanup should use `run_nightly_rollover` and `write_nightly_summary` so completed tasks are cleared, unfinished tasks carry forward, and summaries land in `work.md`/`behavior.md` without manual file rewrites.

The plugin blocks ordinary file-tool edits to protected Antirot files unless the agent first records a justified protected edit intent. Shell access can still bypass ordinary file-tool hooks, so use OpenClaw tool policy to deny `exec` or `group:runtime` when you want stronger protection.

## License

Antirot is dual licensed:

- AGPL-3.0-or-later for open source use.
- A separate commercial license for users who do not want to comply with AGPL terms. Commercial use is negotiated case by case and, unless otherwise agreed in writing, is subject to a royalty capped at 10% of revenue attributable to Antirot use.

See `LICENSE`, `LICENSE-AGPL-3.0`, and `COMMERCIAL-LICENSE.md`.
