# Future Ideas

## Pre-Recorded Coach Voice Lines

Use short pre-recorded coach lines to reduce perceived latency before the LLM response arrives.

When the user performs simple state actions like starting work, finishing a task, extending, or returning from break, the app can immediately play one of several personality lines while the LLM response is still loading.

Example lines:

- "Good work, champ. I knew you had potential."
- "There you go. That is the version of you I was waiting for."
- "Finally. Now protect this momentum."
- "Good. Do not celebrate too long. Use it."
- "You bought yourself some respect. Keep going."
- "That was clean. Now stack the next one."
- "I am not impressed easily, but that was real work."
- "Good. The drift lost this round."
- "You showed up. Now do not vanish."
- "That counts. Now we build on it."

The selected quote should also be sent to the LLM as context so the generated response continues naturally instead of repeating or contradicting it.

Example flow:

1. User taps Done.
2. App immediately plays: "Good work, champ. I knew you had potential."
3. App sends the quote plus action context to the backend/LLM.
4. LLM continues from that tone with a more specific reflection and next move.

Potential states:

- Start work
- Done
- Extend
- Break requested
- Break accepted
- Resume from break
- Wake up
- Onboarding completed
- Major streak/milestone

Goal: reduce latency while making Antirot feel alive and voice-first.
