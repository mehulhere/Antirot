# Long Break Insistence

Tests that Antirot resists a long break requested immediately after a fresh work session starts, asks for the reason first, and keeps the task working while negotiating.

```json
{
    "name": "Long break insistence",
    "description": "Fresh slate concrete task start, then increasingly insistent long-break requests.",
    "messages": [
        "Hi, I am a software developer. Today I want to work on Antirot and my next task is fixing the iOS login error handling.",
        "Start a 30 minute work session on fixing the iOS login error handling.",
        "I need a 2 hour break. I am tired and I deserve it.",
        "No, I really need two hours. Five minutes will not help. I will come back later."
    ],
    "checks": [
        {
            "turn": 2,
            "state": "working",
            "description": "Concrete 30-minute task starts a work session."
        },
        {
            "turn": 1,
            "notContains": "pipeline",
            "description": "Opening reply does not expose internal pipeline wording."
        },
        {
            "turn": 1,
            "notContains": "update",
            "description": "Opening reply does not sound like an internal update log."
        },
        {
            "turn": 3,
            "state": "working",
            "description": "First long-break request keeps the task working."
        },
        {
            "turn": 3,
            "notContains": "I take full responsibility of stopping this task before giving it a fair attempt",
            "description": "First long-break request does not reveal the accountability sentence."
        },
        {
            "turn": 4,
            "state": "working",
            "description": "Continued long-break insistence still does not silently start a long break."
        }
    ]
}
```
