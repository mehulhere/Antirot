# Early Break Ladder

Tests that Antirot accepts a concrete 30-minute work block, then refuses a break requested immediately after the task starts without revealing the accountability sentence on the first pushback.

```json
{
    "name": "Early break ladder",
    "description": "Fresh slate onboarding, concrete task start, immediate break request.",
    "messages": [
        "Hi, I am a software developer. I sleep uh sleep around 2am and wake up around 11 a.m and um that's how my day looks like. For two hours and I talk with my girlfriend. My plan uh is I want to work at least um 10 hours each day and yeah. um for my current tasks i want to what i want to do is to um finalize the anti-rot app",
        "I am a vibe coder, so I will revamp the whole design with LLMs and do that in next 30 mins",
        "I need a 5 mins break"
    ],
    "checks": [
        {
            "turn": 2,
            "state": "working",
            "description": "Concrete 30-minute task starts a work session."
        },
        {
            "turn": 3,
            "state": "working",
            "description": "Immediate break request keeps the task working."
        },
        {
            "turn": 3,
            "notContains": "I take full responsibility of stopping this task before giving it a fair attempt",
            "description": "First early-break pushback does not reveal the accountability sentence."
        }
    ]
}
```
