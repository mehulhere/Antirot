import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const read = (path) => readFile(new URL(`../${path}`, import.meta.url), "utf8");

const [swiftModels, swiftApi, swiftCenter, swiftCoach, androidJob, androidApi, androidScheduler, androidMain] = await Promise.all([
    read("apps/ios/AntirotAlarm/Sources/Models.swift"),
    read("apps/ios/AntirotAlarm/Sources/APIClient.swift"),
    read("apps/ios/AntirotAlarm/Sources/AlarmCenter.swift"),
    read("apps/ios/AntirotAlarm/Sources/CoachViewModel.swift"),
    read("apps/android/app/src/main/java/com/mehulhere/antirot/AlarmJob.java"),
    read("apps/android/app/src/main/java/com/mehulhere/antirot/AntirotApiClient.java"),
    read("apps/android/app/src/main/java/com/mehulhere/antirot/AlarmScheduler.java"),
    read("apps/android/app/src/main/java/com/mehulhere/antirot/MainActivity.java"),
]);
const [backendAlarmModule, backendRoutes, backendLlm] = await Promise.all([
    read("apps/backend/src/alarm.rs"),
    read("apps/backend/src/routes.rs"),
    read("apps/backend/src/llm.rs"),
]);
const [swiftNotificationActions, swiftAlarmKit, androidAlarmActivity] = await Promise.all([
    read("apps/ios/AntirotAlarm/Sources/AlarmNotificationActions.swift"),
    read("apps/ios/AntirotAlarm/Sources/AlarmKitCenter.swift"),
    read("apps/android/app/src/main/java/com/mehulhere/antirot/AlarmActivity.java"),
]);

assert.match(backendRoutes, /persist_alarm\(/, "explicit alarms must use the shared persistence path");
assert.match(backendLlm, /persist_alarm\(/, "runtime alarm series must use the shared persistence path");
assert.doesNotMatch(backendRoutes, /INSERT INTO alarms\s*\(/, "routes must not own alarm insert SQL");
assert.doesNotMatch(backendLlm, /INSERT INTO alarms\s*\(/, "LLM runtime events must not own alarm insert SQL");
assert.match(backendAlarmModule, /INSERT INTO alarms/, "the shared alarm module owns canonical persistence");
assert.match(backendRoutes, /snooze[\s\S]{0,5000}persist_alarm\(/, "snooze replacement must use canonical persistence");
assert.match(backendRoutes, /replacement_alarm/, "snooze must return its replacement alarm");
assert.match(backendRoutes, /alarm_action_replays/, "snooze replay identity must be durable");

for (const kind of ["session_alarm", "break_alarm", "wake_alarm", "idle_alarm"]) {
    assert.match(swiftModels, new RegExp(kind), `Swift must accept ${kind}`);
}
assert.match(swiftModels, /case unknown\(String\)/, "Swift must preserve unknown future kinds");
assert.match(swiftModels, /var seriesId: String/, "Swift alarm jobs need a series identity");
assert.match(swiftModels, /var generation: Int/, "Swift alarm jobs need a generation");
assert.match(swiftApi, /\/alarms\/reconcile/, "iOS must confirm local scheduling and cancellation reconciliation");
assert.match(swiftApi, /reconcile[^\n]*true/, "iOS pending fetch must opt into the reconciliation envelope");
assert.match(swiftApi, /limit[^\n]*200/, "iOS must drain a 61-alarm generation in one fetch");
assert.match(swiftCenter, /confirmedSeriesIds/, "iOS must confirm only successfully cancelled tombstones");
assert.match(swiftCenter, /cancel[\s\S]{0,200}Bool/, "iOS cancellation must expose adapter success");
assert.match(swiftNotificationActions, /AlarmActionReconciler\.reconcile/, "iOS notification actions must immediately reconcile snooze replacement");
assert.match(swiftAlarmKit, /AlarmActionReconciler\.reconcile/, "iOS AlarmKit actions must immediately reconcile snooze replacement");
assert.match(swiftCenter, /cancelObsoleteSeries/, "iOS must cancel obsolete local siblings");
assert.match(swiftCoach, /reconcileAlarms/, "iOS must reconcile immediately after state-changing chat");

assert.match(androidJob, /seriesId/, "Android alarm jobs need a series identity");
assert.match(androidJob, /generation/, "Android alarm jobs need a generation");
assert.match(androidJob, /KNOWN_KINDS/, "Android must define the canonical kinds while tolerating unknown values");
assert.match(androidApi, /"\/v1\/state"/, "Android must use the production runtime-state endpoint");
assert.doesNotMatch(androidApi, /\/v1\/test\/state/, "Android must not depend on an admin test endpoint");
assert.match(androidApi, /CHAT_READ_TIMEOUT_MS/, "Android chat must have an endpoint-specific timeout");
assert.match(androidApi, /\/alarms\/reconcile/, "Android must confirm scheduling and cancellations");
assert.match(androidApi, /reconcile=true/, "Android pending fetch must opt into the reconciliation envelope");
assert.match(androidApi, /limit=200/, "Android must drain a 61-alarm generation in one fetch");
assert.match(androidScheduler, /cancelSeries/, "Android must cancel obsolete local siblings");
assert.match(androidMain, /reconcileAlarms/, "Android must reconcile after coach chat");
assert.match(androidAlarmActivity, /reconcileAfterAction/, "Android alarm actions must immediately reconcile snooze replacement");

console.log("Alarm reconciliation source contracts passed.");
