# Client and API delivery

```mermaid
flowchart TD
    A["iOS queued chat<br/>apps/ios/AntirotAlarm/Sources/CoachViewModel.swift:177"] --> C["POST /v1/chat<br/>apps/backend/src/routes.rs:1484"]
    B["Android queued chat<br/>apps/android/app/src/main/java/com/mehulhere/antirot/MainActivity.java:224"] --> C
    C --> D["Reply plus runtime state<br/>apps/backend/src/routes.rs:1490"]
    D --> E["iOS GET /v1/state<br/>apps/ios/AntirotAlarm/Sources/APIClient.swift:146"]
    D --> F["Android GET /v1/test/state<br/>apps/android/app/src/main/java/com/mehulhere/antirot/AntirotApiClient.java:71"]
    F --> G["Test endpoint and admin gate<br/>apps/backend/src/routes.rs:1967"]
    H["State tool inserts alarm series<br/>apps/backend/src/llm.rs:1291"] --> I["Pending-alarm fetch<br/>apps/backend/src/routes.rs:867"]
    I --> J["iOS/Android local scheduling<br/>apps/ios/AntirotAlarm/Sources/AlarmCenter.swift:79"]
    K["Explicit alarm route<br/>apps/backend/src/routes.rs:706"] --> L["APNs wake<br/>apps/backend/src/apns.rs:18"]
```

The clients mix versioned and unversioned routes. Android depends on a production-disabled test endpoint, while the legacy tester infers state from markdown logs instead of the authoritative runtime-state API.
