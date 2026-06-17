# Done

- Manually verify `/override`, `/vacation`, natural chat tool use, protected edit flow, cron reminders, and loud alarm fallback in a backend app flow.
- Manually verify `start_sleep`, good morning auto-wake logging, sleep-debt calculation, normal alarm fallback, and hidden-buffer loud wake escalation in a backend app flow.
- Manually verify `list_active_triggers`, early completion clearing, early wake clearing, stale callback ignoring, and `reschedule_trigger` for "I need more time" in a backend app flow.
- Manually verify behavior context injection, misc queue add/list/pop, nightly rollover cleanup, and nightly summary extraction in a backend app flow.
- Programmatically verify Scenarios A through M using the `test-scenarios` script to validate all 13 behavioral coaching loops.
- Manually verify onboarding asks for goals/projects, divides them into levels 1-4, prompts the user for confirmation, and writes them to memory files via `save_onboarding_answers`.
- Manually verify the simplified humorous coach-style onboarding prompt and confirmation flow in a backend app flow.
- Build and run `apps/ios` on a real iPhone to verify Antirot registration, notification permission, normal/loud test alarms, alarm actions, and Screen Time authorization.
- Run the GitHub Actions `Build iOS IPA` workflow and install the uploaded unsigned IPA through SideStore/AltStore for no-Mac iPhone testing.
- Build and run `apps/android` on a real Android phone to verify Antirot registration, exact alarm permission, normal/loud alarms, alarm actions, and last-30-minute usage access.
- Build the iOS app with an iOS 26 SDK and verify `Request real alarm permission` schedules real AlarmKit alarms instead of notification fallback.
- Add the Antirot Current Task widget on iPhone and verify scheduled alarms/tasks update the widget through shared app-group storage.
- Select a custom alarm sound on iOS and Android, schedule normal/loud test alarms, and verify the chosen sound plays instead of the system default where the platform signing/runtime supports it.
- Press `Show current task in widget` on iPhone, confirm the app reports app-group availability, and verify the Antirot Current Task widget refreshes without reinstalling.
- Install the iOS IPA, confirm the `Antirot Current Task` widget appears in the iOS widget picker, add it to Home Screen, then press `Show current task in widget`.
- Schedule normal and loud test alarms on iOS and Android and verify normal uses `antirot-normal` while loud/urgent uses `antirot-loud` unless a custom sound is selected.
- In the iOS and Android alarm sound sections, select Auto, bundled normal, bundled loud, and custom sound modes, then verify the scheduled test alarms use the chosen mode.
- Deploy `apps/backend` on a VPS behind `api.antirot.org`, register a phone with `ANTIROT_DEVICE_TOKEN`, create an alarm with `ANTIROT_ADMIN_TOKEN`, poll pending alarms from the app, and verify ack/snooze events are stored.
- Install iOS and Android builds, confirm `https://api.antirot.org` is the default backend, and verify the URL/token fields are hidden under Developer Settings.
- Upgrade from an older iOS/Android install with a blank saved backend URL and verify registration falls back to `https://api.antirot.org` instead of showing a missing VPS URL error.
- Trigger an iOS backend registration failure and verify the app shows the HTTP status/body instead of the generic invalid response message.
- Trigger an iOS backend registration failure and verify `Show full error` reveals the complete stored error detail while the main status remains concise.
- In iOS and Android Developer Settings, press `Reset backend session` and verify it clears the token, rotates the device ID, resets registration, and lets a fresh token register again.
- Press `Reset local login` on iOS and Android and verify it clears stale backend auth, rotates the device ID, and opens `https://antirot.org`.
- Press `Reset local login` on iOS and Android and verify it clears local auth without opening a website.
- Reload the homepage and verify the Rotters Challenged badge starts at 0, counts up quickly, and eases into the exact fetched visitor count.
- Install the iOS IPA, tap `Continue with Google`, verify the native Google sheet appears without opening `login.html`, and confirm the backend returns/stores an Antirot device token.
- Open the iOS app sign-in section and verify the Antirot favicon appears above the Google login button.
- Verify the iOS app shows `Continue with Google` only on the logged-out signup screen, hides device/server/permission controls under bottom settings, and shows `Logout` at the bottom only after sign-in.
- Run `antirot-backend pair --workspace main --timeout 60`, enter the 6-digit code in the signed-in iOS app, and verify the command prints the paired device while Postgres stores the device/workspace mapping.
- Configure APNs env vars on the VPS, sign into the iOS app so it registers an APNs token, create a backend alarm for the device, and verify the app wakes/fetches/schedules the pending alarm.
- Build the iOS app after `xcodegen generate`, verify the login screen shows the animated Antirot branding with ambient gradient and Google sign-in button, then sign in and verify the 3-tab layout (Home/Alarms/Settings) with dark theme, glassmorphism cards, severity-colored alarm cards, permission status dots, developer tools toggle, and dark-themed widget all render correctly.

---

**Architecture Pivot (2026-06-08):** Antirot is a native app + managed backend product. Primary development focus is the iOS app (`apps/ios/`), Android app (`apps/android/`), backend (`apps/backend/`), and frontend tester (`website/tester.html`).

- Deploy the updated `apps/backend`, configure subscription tier (BYOK or Tailored) at `/v1/subscription`, send a test message to `/v1/chat`, and verify memory files are initialized and tools update `user_memories` correctly.
- Verify that calling `start_session` fails if the `task_id` does not match active tasks in `tasks.md`, `log_wake` accepts `sleep_quality` (1-5) instead of tiredness, weekly override records are appended to weekly override files, and legacy alarm triggers are removed from the coach tool definitions.
- Verify that calling `start_session` auto-sets a sequence of session alarms (silent for the first two, loud for subsequent ones up to 5 hours) and that `end_session` auto-deletes them; verify the app schedules morning alarms from sleep state automatically and that acknowledging an alarm or calling `log_wake` cancels the remaining pending alarms.
- Verify that sending a chat message containing "log work", "need a break", or "start working" immediately cancels pending session alarms, and that `extend_session` and `start_break` tools successfully schedule the 5-hour alarm sequence.
- Verify that configuring `GOOGLE_CLOUD_CREDENTIALS` in the environment/dot-env automatically routes tailored requests to Google Vertex AI using RS256 JWT assertion OAuth token exchange.
- Open `http://localhost:8000/tester.html`, verify it auto-connects with no backend settings panel, URL input, password input, or connect button, and confirm chat, pending alarms, and memory tabs use the local backend.
- Verify backend runtime states transition through onboarding, working, sleeping, break, vacation, and idle; confirm idle schedules check-in alarms every 5 minutes while onboarding/vacation schedule none.
- Verify `routine.md` appears in the tester memory tabs, initializes with fixed daily allocations, and can be updated by the coach through `patch_file`.
- Run backend userflow tests with `npm run test:backend-userflows` and `npm run test:backend-userflows-llm`; review the LLM transcript for paid-product quality, rerunning the LLM suite after Gemini `gemini-3.5-flash` quota recovers if it returns `429 RESOURCE_EXHAUSTED`.
- Verify the battle-tested prompt architecture keeps replies free of internal state language, enforces context budgets, and passes the expanded Gemini transcript quality suite.
- Verify prompt snapshot fixtures fail on accidental prompt drift while backend userflows still pass.
- Verify nightly memory distillation updates `durable.md` when sleep starts, sleep metrics update after wake logs, semantic memory search returns relevant historical logs once memory is large, and `/v1/admin/context` works with admin auth outside test mode.
- Verify GitHub Backend CI blocks prompt snapshot drift and runs backend no-LLM userflows against Postgres on pull requests.
- Run `npm run test:backend-userflows-llm` followed by `CROF_API_KEY=... npm run test:llm-judge-quality`, then review `.antirot/llm-judge-quality-report.json` for Qwen judge scores and issues before treating live LLM output as paid-product ready.
- Build the iOS app on a real iPhone, verify Coach/Plan/Alarms/Settings render correctly, record a voice check-in through Fireworks STT, send Done/Start/Break buttons through chat, and confirm Async TTS playback after `ASYNC_TTS_VOICE_ID` is configured.
- Launch the backend-only VPS setup from `docs/backend-vps-new-user.md` with fresh `antirot` and `antirot-backend` Linux users, then verify `/health`, `/v1/chat`, and `/v1/speech/transcribe` through the public HTTPS domain.
