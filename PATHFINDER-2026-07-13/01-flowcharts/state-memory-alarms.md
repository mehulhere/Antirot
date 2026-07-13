# Runtime state, memory, and alarms

```mermaid
flowchart TD
    A["LLM state tool<br/>apps/backend/src/llm.rs:1347"] --> B["Append work/sleep memory<br/>apps/backend/src/llm.rs:1522"]
    B --> C["Upsert memory and rebuild index<br/>apps/backend/src/memory.rs:97"]
    C --> D["Delete pending alarms<br/>apps/backend/src/llm.rs:1264"]
    D --> E["Insert five-hour alarm series<br/>apps/backend/src/llm.rs:1291"]
    E --> F["Upsert runtime state<br/>apps/backend/src/llm.rs:1237"]
    F --> G["Client fetch marks delivered<br/>apps/backend/src/routes.rs:867"]
    G --> H["iOS strict alarm decode<br/>apps/ios/AntirotAlarm/Sources/Models.swift:8"]
    H --> I["Schedule AlarmKit/notification<br/>apps/ios/AntirotAlarm/Sources/AlarmCenter.swift:102"]
    I --> J["Acknowledge or snooze<br/>apps/backend/src/routes.rs:921"]
    K["Five-minute distillation worker<br/>apps/backend/src/main.rs:99"] --> L["Summary, durable append, marker<br/>apps/backend/src/memory.rs:725"]
```

The core write path is non-atomic across memory, alarms, and state. Pending delivery is at-most-once, and state-created series bypass the APNs path used at `apps/backend/src/routes.rs:706-865`.
