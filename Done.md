# Done

- Manually verify `/override`, `/vacation`, natural chat tool use, protected edit flow, cron reminders, and loud alarm fallback in a linked OpenClaw gateway.
- Manually verify `start_sleep`, good morning auto-wake logging, sleep-debt calculation, normal alarm fallback, and hidden-buffer loud wake escalation in a linked OpenClaw gateway.
- Manually verify `list_active_triggers`, early completion clearing, early wake clearing, stale callback ignoring, and `reschedule_trigger` for "I need more time" in a linked OpenClaw gateway.
- Manually verify behavior context injection, misc queue add/list/pop, nightly rollover cleanup, and nightly summary extraction in a linked OpenClaw gateway.
- Programmatically verify Scenarios A through M using the `test-scenarios` script to validate all 13 behavioral coaching loops.
- Manually verify onboarding asks for goals/projects, divides them into levels 1-4, prompts the user for confirmation, and writes them to memory files via `save_onboarding_answers`.
- Manually verify the simplified humorous coach-style onboarding prompt and confirmation flow in a linked OpenClaw gateway.
- Build and run `apps/ios` on a real iPhone to verify Antirot registration, notification permission, normal/loud test alarms, alarm actions, and Screen Time authorization.
- Run the GitHub Actions `Build iOS IPA` workflow and install the uploaded unsigned IPA through SideStore/AltStore for no-Mac iPhone testing.
- Build and run `apps/android` on a real Android phone to verify Antirot registration, exact alarm permission, normal/loud alarms, alarm actions, and last-30-minute usage access.
- Build the iOS app with an iOS 26 SDK and verify `Request real alarm permission` schedules real AlarmKit alarms instead of notification fallback.
- Add the Antirot Current Task widget on iPhone and verify scheduled alarms/tasks update the widget through shared app-group storage.
- Select a custom alarm sound on iOS and Android, schedule normal/loud test alarms, and verify the chosen sound plays instead of the system default where the platform signing/runtime supports it.
- Press `Show current task in widget` on iPhone, confirm the app reports app-group availability, and verify the Antirot Current Task widget refreshes without reinstalling.
