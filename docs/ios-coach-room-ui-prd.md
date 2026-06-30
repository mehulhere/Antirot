# Antirot iOS Coach Room UI/UX PRD

## Product Direction

Antirot iOS should feel like a living coach room, not a productivity dashboard. The core experience is a stylized coach character watching, judging, encouraging, and responding to the user's current state.

The app should make the user feel: the coach is here, there is one honest next action, and if the user wants to drift they need to explain themselves.

## Primary Screen

The home screen is full-screen and cinematic.

Core elements:

- Full-screen stylized coach animation or procedural coach stage.
- One large circular primary action button.
- Optional small secondary actions only for specific states.
- Bottom draggable glass chat sheet.
- Hidden complexity: stats, settings, tasks, profile, alarms, and diagnostics.

The first screen must not look like a dashboard.

## State Actions

| Runtime State | Main Button | Small Buttons |
| --- | --- | --- |
| `idle` | Start | none |
| `working` | Done | Extend, Break |
| `break` | Resume | none |
| `sleeping` | Awake | none |
| `onboarding` | Talk | none |
| `unknown` / `offline` | Reconnect | none |

Button rules:

- Main button is circular, large, and thumb-friendly.
- Main button should feel physical and important.
- Small buttons are visually quieter.
- In `working`, Done is dominant; Extend and Break are available but subordinate.

## Coach Character

Character style:

- Stylized human coach.
- Expressive, but not realistic.
- Slightly intimidating, sharp, emotionally readable.
- Avoid cute mascot energy.
- Avoid uncanny realism.

Coach animation states:

- `watching`
- `checking_clock`
- `thinking`
- `focused`
- `strict`
- `impatient`
- `approving`
- `disappointed`
- `celebrating`
- `silent_waiting`

Behavior:

1. Idle loop: coach watches or checks the clock.
2. User sends message or taps an action: coach switches to thinking/checking-clock.
3. LLM response returns with an optional emotion.
4. App transitions to the selected coach emotion.
5. If voice is enabled, app speaks the response or preface.

Transitions should crossfade or smoothly animate, not hard-cut.

## Chat Sheet

Bottom chat is hidden by default.

Collapsed state:

- Small drag handle.
- Latest coach one-liner or status.
- Mic button remains available.

Expanded state:

- Glassy translucent chat panel.
- Background coach animation remains visible through blur.
- Recent messages visible.
- Voice-first composer at bottom.

Snap points:

- Collapsed
- Half-height
- Full-height

Glass style:

- Dark translucent material.
- Strong blur.
- Thin border.
- Subtle sheen.
- Text must stay readable.

## Voice UX

Antirot should feel voice-first.

Composer:

- Mic button is primary.
- Text input is available but secondary.
- Send button is minimal and appears only when text exists.
- Voice playback should feel like the coach speaking, not a generic TTS utility.

After user action:

- App may immediately play a pre-recorded quote.
- LLM response continues once ready.

## Start Work Feedback

On successful Start:

- Show quick charged-particle burst.
- Duration target: 700-1200ms.
- It should feel sharp and energetic, not childish.
- Coach transitions to focused or approving.

## Hidden Complexity

Do not show by default:

- Completed tasks
- Pending tasks
- Stats
- User profile
- Settings
- Diagnostics
- Alarm configuration

Access:

- Settings and secondary screens live behind a small top-right menu.
- Stats and tasks can appear through chat or secondary sheet.
- The main screen should not invite fiddling.

## LLM Emotion Contract

Each coach response may optionally include:

```json
{
  "coach_emotion": "strict",
  "voice_preface": "Good. Now protect this momentum."
}
```

The app maps `coach_emotion` to a coach stage state.

If absent:

- Use `watching` after the response.
- Use `thinking` while the response is in flight.

## Acceptance Criteria

- Opening the app shows the coach scene first, not dashboard content.
- Only one dominant circular button appears per state.
- `working` shows Done plus small Extend and Break.
- Chat can drag up from the bottom.
- Expanded chat uses glass blur and preserves background visibility.
- Coach emotion changes after LLM response when provided.
- Start action triggers a short charged-particle burst.
- Tasks, stats, settings, and diagnostics are not visible in the primary experience.
- UI feels closer to a cinematic coach interface than a productivity tracker.

## Designer Deliverables

- Main home screen with collapsed chat.
- Main home screen with half-expanded chat.
- Full chat sheet state.
- Working state with Done plus Extend and Break.
- Idle state with Start.
- Coach emotion GIF direction sheet.
- Glass material specs.
- Button component specs.
- Particle/confetti reference.
