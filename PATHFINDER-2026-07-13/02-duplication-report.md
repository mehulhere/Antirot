# Duplication Report

## Harmful duplication

1. **Runtime-state vocabulary is redefined across layers.** PostgreSQL defines states at `apps/backend/sql/001_init.sql:188-194`; backend transitions repeat them at `apps/backend/src/llm.rs:1142-1233`; frontend, iOS, and Android repeat partial maps at `apps/frontend/app/page.tsx:240-288`, `apps/ios/AntirotAlarm/Sources/StateActions.swift:24-245`, and `apps/android/app/src/main/java/com/mehulhere/antirot/CoachQuickAction.java:21-48`. Platform presentation is legitimate; the enum and transition rules are not.

2. **Alarm kinds have incompatible vocabularies.** Backend series use `session_alarm`, `break_alarm`, `wake_alarm`, and `idle_alarm` at `apps/backend/src/llm.rs:1149-1225`; iOS accepts a different closed enum at `apps/ios/AntirotAlarm/Sources/Models.swift:8-16`. Cleanup repeats literals at `apps/backend/src/routes.rs:1006-1019` and `apps/backend/src/llm.rs:1264-1275`.

3. **State writes bypass one another.** Normal transitions are at `apps/backend/src/llm.rs:1131-1261`, while auth initialization, test reset, and snapshot restore write state separately at `apps/backend/src/routes.rs:414-424`, `apps/backend/src/routes.rs:2151-2164`, and `apps/backend/src/memory.rs:239-267`.

4. **Alarm creation has two persistence paths.** The explicit route persists and wakes APNs at `apps/backend/src/routes.rs:706-865`; coach state transitions insert directly without APNs at `apps/backend/src/llm.rs:1291-1345`.

5. **Alarm lifecycle policy is recreated on server and clients.** Backend delivery/action logic lives at `apps/backend/src/routes.rs:867-1037`; iOS and Android schedule independently at `apps/ios/AntirotAlarm/Sources/AlarmCenter.swift:79-134` and `apps/android/app/src/main/java/com/mehulhere/antirot/AlarmScheduler.java:19-41`, without a shared series identity or reconciliation contract.

6. **Memory metadata is hand-maintained in six places.** Defaults/allowlist: `apps/backend/src/prompt.rs:75-103`; prompt sections: `apps/backend/src/llm.rs:825-922`; filename mapping/schema: `apps/backend/src/llm.rs:1360-1422,1941-1961`; snapshots/search: `apps/backend/src/memory.rs:21-35,875-909`.

7. **Runtime tools repeat log-write then state-transition code.** Start/end/extend/break/sleep/wake repeat the same non-atomic sequence at `apps/backend/src/llm.rs:1522-1702`.

8. **Tool success and visible replies have competing representations.** Execution returns prose strings at `apps/backend/src/llm.rs:1347-1919`; success is reparsed at `apps/backend/src/llm.rs:556-620`; curated copy can replace persisted model text at `apps/backend/src/llm.rs:471-516`.

9. **Onboarding copy and its hidden control protocol have multiple owners.** Backend copy is at `apps/backend/src/llm.rs:147`; iOS copy is at `apps/ios/AntirotAlarm/Sources/CoachViewModel.swift:7-14`; hidden messages are built at `apps/ios/AntirotAlarm/Sources/HomeView.swift:287-296` and `apps/frontend/app/page.tsx:329-338`.

10. **“Today” is reconstructed using UTC across features.** Prompt/logging use it at `apps/backend/src/llm.rs:778-801,1522-1612`; distillation uses it at `apps/backend/src/memory.rs:566-579`; frontend repeats it at `apps/frontend/app/page.tsx:268-274`, while prompt time is hardcoded to IST at `apps/backend/src/llm.rs:1057-1066`.

## Duplication to preserve

- AlarmKit/UserNotifications on iOS and AlarmManager on Android are legitimate OS adapters. Unify only wire contracts, IDs, and reconciliation.
- Platform-specific action layout and visual labels are legitimate after the state enum is canonical.
- Admin/test envelopes may contain extra diagnostics, but should embed the production runtime-state DTO.
- Temporary unversioned API aliases are acceptable only with usage telemetry and an explicit removal window.

## Consolidation order

1. Canonical runtime/alarm contracts.
2. Transactional runtime transition plus one alarm service/outbox.
3. Desired-alarm reconciliation for both mobile clients.
4. Typed tool outcomes and one persisted visible reply.
5. Memory descriptor registry and async derived indexing.
6. Typed onboarding and one user-local day policy.
