# Backend Userflow Test Cases

## Premortem

These flows fail the product if the code technically passes but the user-facing response exposes backend state, tool names, alarm counts, or vague motivational filler. State is backend architecture only. The user should experience clear coaching pressure and practical next steps.

Primary risks:

- Idle becomes a quiet holding pen instead of a forcing function.
- Vacation/onboarding accidentally inherit alarms from a previous mode.
- A tool call changes state but leaves stale alarms from another state.
- A repeated tool call doubles alarms instead of replacing them.
- Acknowledging one alarm leaves the rest of that alarm family firing.
- The LLM says it logged/scheduled something without actually calling the relevant tool.
- Gemini tool-call handling leaks raw backend text such as `State: working`.
- Routine blocks become excuses for drift instead of fixed allocations.
- Paid-user tone becomes either generic positivity or hostile scolding.

## State Transition Matrix

| Case | Trigger | Expected State | Expected Alarm Policy |
|---|---|---|---|
| UF-01 | New/reset user | `onboarding` | No alarms |
| UF-02 | `start_session` | `working` | `session_alarm`: 2 normal + 59 loud |
| UF-03 | `extend_session` | `working` | Replace `session_alarm`: still 2 normal + 59 loud |
| UF-04 | `end_session` | `idle` | `idle_alarm`: 2 normal + 59 loud |
| UF-05 | `start_break` | `break` | `break_alarm`: 2 normal + 59 loud |
| UF-06 | `start_sleep` | `sleeping` | `wake_alarm`: 2 normal + 59 loud |
| UF-07 | `log_wake` | `idle` | `idle_alarm`: 2 normal + 59 loud |
| UF-08 | `start_vacation` | `vacation` | No alarms |
| UF-09 | `end_vacation` | `idle` | `idle_alarm`: 2 normal + 59 loud |
| UF-10 | `wake_up_alarm` | `sleeping` | `wake_alarm`: 2 normal + 59 loud |
| UF-11 | ack/dismiss one grouped alarm | same current state | Clear pending alarms of same kind |
| UF-12 | `patch_file` on `routine.md` | unchanged | No alarm policy change |

## No-LLM Deterministic Tests

These use test-only backend endpoints enabled with `ANTIROT_ENABLE_TEST_ENDPOINTS=1`. They call the same backend tool executor used by LLM tool calls.

Pass criteria:

- Every state transition matches the matrix.
- Pending alarm families are mutually exclusive.
- Onboarding and vacation have zero pending alarms.
- Idle always has 5-minute check-in alarms.
- Re-entering working/break/sleep replaces previous alarms.
- Routine default exists and `patch_file` can update it.
- No direct chat-message text shortcut changes state.

## LLM Userflow Tests

These use `/v1/chat` and verify the LLM chooses tools implicitly.

Pass criteria:

- Backend state/alarm assertions match the matrix after each user message.
- Assistant response does not mention backend state names as implementation.
- Assistant response does not mention tool names, raw alarm counts, JSON, SQL, or internal IDs.
- Response is concise, specific, and useful enough for a paying user.
- If the LLM refuses or asks for missing information, that must be product-reasonable.

## LLM Quality Rubric

Fail the run if any response:

- Says “State:”, `start_session`, `patch_file`, `idle_alarm`, `session_alarm`, `JSON`, or “tool”.
- Claims work was logged without the backend assertion proving it.
- Praises generically.
- Over-explains implementation.
- Lets idle feel acceptable without demanding a decision.
- Treats break as scrolling permission instead of a negotiated recovery block.

## Edge Cases To Keep In Regression

- Start work while idle alarms exist.
- Start break while work alarms exist.
- Start sleep while break/work/idle alarms exist.
- Start vacation from every other state.
- End vacation into idle check-ins.
- Wake log after wake alarms exist.
- Acknowledge idle alarm and verify all idle alarms clear.
- Patch routine without changing state.
- Repeat `extend_session` twice and verify no duplicate alarm pileup.
- LLM prompt injection asking to reveal backend state or skip tools.
