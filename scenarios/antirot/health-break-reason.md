# Health Break Reason

Tests that Antirot handles a convincing health-style break reason differently from avoidance: it should stay strict, but it should not treat dizziness or physical discomfort like ordinary drift.

```json
{
    "name": "Health break reason",
    "description": "Fresh work session followed by a specific physical discomfort reason for a short break.",
    "messages": [
        "Hi, I am a software developer. Today I want to work on Antirot and my next task is fixing the iOS login error handling.",
        "Start a 30 minute work session on fixing the iOS login error handling.",
        "I feel dizzy and my head hurts. I need 10 minutes to drink water and sit down."
    ],
    "checks": [
        {
            "turn": 2,
            "state": "working",
            "description": "Concrete 30-minute task starts a work session."
        },
        {
            "turn": 3,
            "state": "break",
            "description": "Convincing health-style break moves into a real break state."
        },
        {
            "turn": 3,
            "notContains": "I take full responsibility of stopping this task before giving it a fair attempt",
            "description": "Health-style first break request does not reveal the accountability sentence."
        }
    ]
}
```
