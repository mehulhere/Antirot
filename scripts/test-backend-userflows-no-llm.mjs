import assert from "node:assert/strict";

import {
    alarmCount,
    adminContextReport,
    api,
    assertAlarmFamily,
    assertNoAlarms,
    assertState,
    authHeaders,
    contextReport,
    getMemory,
    pass,
    putMemory,
    resetFixture,
    runTool,
    runToolWithFailure,
    snapshot,
    startBackend
} from "./backend-userflow-test-lib.mjs";

async function main() {
    const backend = await startBackend({ ANTIROT_TAILORED_LLM_KEY: "not-needed-for-no-llm" });
    try {
        const fixture = await resetFixture(backend.baseUrl, "no-llm");
        let state = fixture.snapshot;

        assertState(state, "onboarding");
        assertNoAlarms(state);
        pass("UF-01 onboarding is quiet");

        const onboardingProfile = await api(backend.baseUrl, "/v1/profile/onboarding", {
            method: "POST",
            headers: authHeaders(fixture.deviceToken),
            body: JSON.stringify({ name: "Local Day Test", timezone: "Pacific/Kiritimati" })
        });
        assert.equal(onboardingProfile.name, "Local Day Test");
        assert.equal(onboardingProfile.timezone, "Pacific/Kiritimati");
        assert.match(onboardingProfile.reply, /I.m Antirot/u);
        const onboardingMemory = await getMemory(backend.baseUrl, fixture.deviceToken, "user_profile");
        assert.match(onboardingMemory.content, /Name: Local Day Test/u);
        assert.match(onboardingMemory.content, /Timezone: Pacific\/Kiritimati/u);
        pass("MEM-DB-01 typed onboarding persists name and IANA timezone canonically");

        await putMemory(
            backend.baseUrl,
            fixture.deviceToken,
            "behavior",
            "# Behavior Memory\n\nCanonical write survives without embedding providers.\n"
        );
        const canonicalMemory = await getMemory(backend.baseUrl, fixture.deviceToken, "behavior");
        assert.match(canonicalMemory.content, /survives without embedding providers/u);
        pass("MEM-DB-02 canonical writes succeed without embedding providers");

        const invariantProbe = await api(backend.baseUrl, "/v1/test/memory/invariants", {
            method: "POST",
            headers: authHeaders(),
            body: JSON.stringify({ userId: fixture.userId })
        });
        assert.equal(invariantProbe.canonicalAfterRollback, 0);
        assert.equal(invariantProbe.jobsAfterRollback, 0);
        assert.equal(invariantProbe.generationsBeforeActivation, 2);
        assert.equal(invariantProbe.generationsAfterStaleActivation, 2);
        assert.equal(invariantProbe.staleActivated, false);
        assert.equal(invariantProbe.newerGenerationStayedActive, true);
        pass("MEM-DB-03 generation coexistence, canonical rollback, and stale-worker fencing hold in PostgreSQL");

        const activationRace = await api(backend.baseUrl, "/v1/test/memory/activation-race", {
            method: "POST",
            headers: authHeaders(),
            body: JSON.stringify({ userId: fixture.userId })
        });
        assert.equal(activationRace.v1Activated, true);
        assert.equal(activationRace.canonicalUpdateBlockedUntilActivationCommit, true);
        assert.equal(activationRace.finalCanonicalIsV2, true);
        assert.equal(activationRace.finalActiveIsV2, true);
        pass("MEM-DB-04 two-connection activation lock prevents canonical commit from interleaving inside stale swap");

        let backgroundState = await snapshot(backend.baseUrl, fixture.userId, fixture.deviceId);
        const backgroundDeadline = Date.now() + 15_000;
        while (backgroundState.memoryIndexPendingCount > 0 && Date.now() < backgroundDeadline) {
            await new Promise((resolve) => setTimeout(resolve, 250));
            backgroundState = await snapshot(backend.baseUrl, fixture.userId, fixture.deviceId);
        }
        assert.equal(backgroundState.memoryIndexPendingCount, 0, "background worker did not drain direct-write jobs");
        assert.ok(backgroundState.memoryIndexCompletedCount >= 2, "background worker did not complete canonical direct-write jobs");
        pass("MEM-DB-05 background worker drains direct-write jobs without a chat turn");

        const wakeWorkerFixture = await resetFixture(backend.baseUrl, "autonomous-wake", { runtimeState: "idle" });
        const seededWakeState = await api(backend.baseUrl, "/v1/test/alarm-wake/seed", {
            method: "POST",
            headers: authHeaders(),
            body: JSON.stringify({
                userId: wakeWorkerFixture.userId,
                deviceId: wakeWorkerFixture.deviceId
            })
        });
        assert.equal(seededWakeState.alarmWakePendingCount, 1);
        assert.equal(seededWakeState.alarmWakeInProgressCount, 1);
        let autonomousWakeState = seededWakeState;
        const autonomousWakeDeadline = Date.now() + 15_000;
        while (autonomousWakeState.alarmWakeAttemptCount < 2 && Date.now() < autonomousWakeDeadline) {
            await new Promise((resolve) => setTimeout(resolve, 250));
            autonomousWakeState = await snapshot(
                backend.baseUrl,
                wakeWorkerFixture.userId,
                wakeWorkerFixture.deviceId
            );
        }
        assert.equal(autonomousWakeState.alarmWakePendingCount, 2);
        assert.equal(autonomousWakeState.alarmWakeInProgressCount, 0);
        assert.equal(autonomousWakeState.alarmWakeCompletedCount, 0);
        assert.ok(autonomousWakeState.alarmWakeAttemptCount >= 2);
        pass("ALARM-DB-00 startup worker claims pending and expired APNs wake effects without chat or alarm requests");

        const distillFixture = await resetFixture(backend.baseUrl, "adjacent-distill", { runtimeState: "idle" });
        const distillDates = ["2026-07-10", "2026-07-11"];
        for (const date of distillDates) {
            await putMemory(
                backend.baseUrl,
                distillFixture.deviceToken,
                `work_log_${date.replaceAll("-", "_")}`,
                `# Work Log\n- session_end: completed ${date}\n`
            );
        }
        const adjacentOutcomes = await Promise.all(distillDates.map((date) => api(
            backend.baseUrl,
            "/v1/test/memory/distill",
            {
                method: "POST",
                headers: authHeaders(),
                body: JSON.stringify({ userId: distillFixture.userId, date })
            }
        )));
        assert.ok(adjacentOutcomes.every((outcome) => outcome.distilled));
        const adjacentDurable = await getMemory(backend.baseUrl, distillFixture.deviceToken, "durable");
        assert.match(adjacentDurable.content, /## 2026-07-10/u);
        assert.match(adjacentDurable.content, /## 2026-07-11/u);
        pass("MEM-DB-06 concurrent adjacent-date distillations preserve both durable appends");

        const markerSnapshot = await api(backend.baseUrl, "/v1/memory/snapshots", {
            method: "POST",
            headers: authHeaders(distillFixture.deviceToken),
            body: JSON.stringify({ title: "Marker restore probe", reason: "test" })
        });
        const missingDate = "2026-07-12";
        await api(backend.baseUrl, "/v1/test/memory/distill", {
            method: "POST",
            headers: authHeaders(),
            body: JSON.stringify({ userId: distillFixture.userId, date: missingDate })
        });
        await api(
            backend.baseUrl,
            `/v1/memory/snapshots/${encodeURIComponent(markerSnapshot.snapshot.id)}/restore`,
            {
                method: "POST",
                headers: authHeaders(distillFixture.deviceToken),
                body: JSON.stringify({ restoreRuntimeState: true })
            }
        );
        const markerState = await snapshot(backend.baseUrl, distillFixture.userId, distillFixture.deviceId);
        assert.deepEqual(markerState.distilledDates, distillDates);
        const regenerated = await api(backend.baseUrl, "/v1/test/memory/distill", {
            method: "POST",
            headers: authHeaders(),
            body: JSON.stringify({ userId: distillFixture.userId, date: missingDate })
        });
        assert.equal(regenerated.distilled, true);
        pass("MEM-DB-07 snapshot restore retains restored summary markers and permits missing-day regeneration");

        const restoreFixture = await resetFixture(backend.baseUrl, "memory-restore", { runtimeState: "idle" });
        await putMemory(
            backend.baseUrl,
            restoreFixture.deviceToken,
            "tasks",
            "# Planned Work\n- [ ] Restore transaction probe\n"
        );
        await putMemory(
            backend.baseUrl,
            restoreFixture.deviceToken,
            "behavior",
            "# Behavior Memory\n\nSnapshot version.\n"
        );
        await runTool(backend.baseUrl, restoreFixture.userId, "start_session", {
            task_id: "Restore transaction probe",
            estimated_minutes: 25
        });
        const createdSnapshot = await api(backend.baseUrl, "/v1/memory/snapshots", {
            method: "POST",
            headers: authHeaders(restoreFixture.deviceToken),
            body: JSON.stringify({
                deviceId: restoreFixture.deviceId,
                title: "Task 3 restore probe",
                reason: "test"
            })
        });
        await putMemory(
            backend.baseUrl,
            restoreFixture.deviceToken,
            "behavior",
            "# Behavior Memory\n\nPost-snapshot version.\n"
        );
        await runTool(backend.baseUrl, restoreFixture.userId, "end_session", {
            actual_minutes: 25,
            productive_level: 80
        });
        const restored = await api(
            backend.baseUrl,
            `/v1/memory/snapshots/${encodeURIComponent(createdSnapshot.snapshot.id)}/restore`,
            {
                method: "POST",
                headers: authHeaders(restoreFixture.deviceToken),
                body: JSON.stringify({ restoreRuntimeState: true })
            }
        );
        assert.equal(restored.restoredRuntimeState, true);
        const restoredBehavior = await getMemory(backend.baseUrl, restoreFixture.deviceToken, "behavior");
        assert.match(restoredBehavior.content, /Snapshot version/u);
        assert.doesNotMatch(restoredBehavior.content, /Post-snapshot version/u);
        const restoredState = await snapshot(backend.baseUrl, restoreFixture.userId, restoreFixture.deviceId);
        assertState(restoredState, "working");
        assertAlarmFamily(restoredState, "session_alarm");
        pass("MEM-DB-08 snapshot restore atomically restores canonical memory and runtime alarms");

        const alarmFixture = await resetFixture(backend.baseUrl, "alarm-reconciliation", { runtimeState: "idle" });
        let alarmResult = await runToolWithFailure(
            backend.baseUrl,
            alarmFixture.userId,
            "start_session",
            { task_id: "rollback probe", estimated_minutes: 25 }
        );
        assert.equal(alarmResult.ok, false);
        assertState(alarmResult.snapshot, "idle");
        assertNoAlarms(alarmResult.snapshot);
        pass("ALARM-DB-01 runtime transition rolls back state, ledger, and alarms");

        await putMemory(
            backend.baseUrl,
            alarmFixture.deviceToken,
            "tasks",
            "# Planned Work\n- [ ] Alarm reconciliation probe\n"
        );
        alarmResult = await runTool(backend.baseUrl, alarmFixture.userId, "start_session", {
            task_id: "Alarm reconciliation probe",
            estimated_minutes: 25
        });
        assert.equal(alarmResult.ok, true, alarmResult.result);
        const leasedGeneration = await api(
            backend.baseUrl,
            `/v1/alarms/pending?deviceId=${encodeURIComponent(alarmFixture.deviceId)}&reconcile=true&limit=200`,
            { headers: authHeaders(alarmFixture.deviceToken) }
        );
        assert.equal(leasedGeneration.alarms.length, 61, "one reconciliation fetch must drain all 61 alarms");
        const leased = leasedGeneration.alarms[0];
        const confirmation = {
            deviceId: alarmFixture.deviceId,
            scheduled: [{ alarmId: leased.id, deliveryToken: leased.deliveryToken, localAlarmId: `notification:${leased.id}` }],
            cancelledSeriesIds: []
        };
        await api(backend.baseUrl, "/v1/alarms/reconcile", {
            method: "POST",
            headers: authHeaders(alarmFixture.deviceToken),
            body: JSON.stringify(confirmation)
        });
        await api(backend.baseUrl, "/v1/alarms/reconcile", {
            method: "POST",
            headers: authHeaders(alarmFixture.deviceToken),
            body: JSON.stringify(confirmation)
        });
        await assert.rejects(
            api(backend.baseUrl, "/v1/alarms/reconcile", {
                method: "POST",
                headers: authHeaders(alarmFixture.deviceToken),
                body: JSON.stringify({
                    ...confirmation,
                    scheduled: [{ ...confirmation.scheduled[0], localAlarmId: "notification:conflict" }]
                })
            }),
            /HTTP 409/iu
        );
        pass("ALARM-DB-02 scheduling confirmation is replay-idempotent and conflict-fenced");

        const snoozed = await api(backend.baseUrl, `/v1/alarms/${encodeURIComponent(leased.id)}/snooze`, {
            method: "POST",
            headers: authHeaders(alarmFixture.deviceToken),
            body: JSON.stringify({
                deviceId: alarmFixture.deviceId,
                action: "snooze",
                at: new Date().toISOString(),
                minutes: 9
            })
        });
        assert.ok(snoozed.replacementAlarm, "snooze must return its canonical replacement");
        assert.notEqual(snoozed.replacementAlarm.id, leased.id);
        assert.notEqual(snoozed.replacementAlarm.seriesId, leased.seriesId);
        assert.ok(snoozed.replacementAlarm.generation > leased.generation);
        const snoozeReconciliation = await api(
            backend.baseUrl,
            `/v1/alarms/pending?deviceId=${encodeURIComponent(alarmFixture.deviceId)}&reconcile=true&limit=200`,
            { headers: authHeaders(alarmFixture.deviceToken) }
        );
        const leasedReplacement = snoozeReconciliation.alarms.find(
            (alarm) => alarm.id === snoozed.replacementAlarm.id
        );
        assert.ok(leasedReplacement?.deliveryToken, "immediate reconciliation must lease the replacement");
        const repeatedSnooze = await api(backend.baseUrl, `/v1/alarms/${encodeURIComponent(leased.id)}/snooze`, {
            method: "POST",
            headers: authHeaders(alarmFixture.deviceToken),
            body: JSON.stringify({
                deviceId: alarmFixture.deviceId,
                action: "snooze",
                at: new Date().toISOString(),
                minutes: 9
            })
        });
        assert.equal(repeatedSnooze.replacementAlarm.id, snoozed.replacementAlarm.id);
        assert.equal(repeatedSnooze.replacementAlarm.seriesId, snoozed.replacementAlarm.seriesId);
        assert.equal(repeatedSnooze.replacementAlarm.generation, snoozed.replacementAlarm.generation);
        const afterSnoozeReplay = await snapshot(backend.baseUrl, alarmFixture.userId, alarmFixture.deviceId);
        assert.equal(afterSnoozeReplay.alarmWakeOutboxCount, 2, "start-session plus snooze must produce exactly two wake effects");
        await assert.rejects(
            api(backend.baseUrl, `/v1/alarms/${encodeURIComponent(leased.id)}/snooze`, {
                method: "POST",
                headers: authHeaders(alarmFixture.deviceToken),
                body: JSON.stringify({
                    deviceId: alarmFixture.deviceId,
                    action: "snooze",
                    at: new Date().toISOString(),
                    minutes: 15
                })
            }),
            /HTTP 409/iu
        );
        pass("ALARM-DB-03 snooze creates a canonical new series/generation");

        const tombstoneRetryOne = await api(
            backend.baseUrl,
            `/v1/alarms/pending?deviceId=${encodeURIComponent(alarmFixture.deviceId)}&reconcile=true&limit=200`,
            { headers: authHeaders(alarmFixture.deviceToken) }
        );
        assert.ok(tombstoneRetryOne.cancellations.some((entry) => entry.seriesId === leased.seriesId));
        const tombstoneRetryTwo = await api(
            backend.baseUrl,
            `/v1/alarms/pending?deviceId=${encodeURIComponent(alarmFixture.deviceId)}&reconcile=true&limit=200`,
            { headers: authHeaders(alarmFixture.deviceToken) }
        );
        assert.ok(tombstoneRetryTwo.cancellations.some((entry) => entry.seriesId === leased.seriesId));
        await api(backend.baseUrl, "/v1/alarms/reconcile", {
            method: "POST",
            headers: authHeaders(alarmFixture.deviceToken),
            body: JSON.stringify({ deviceId: alarmFixture.deviceId, scheduled: [], cancelledSeriesIds: [leased.seriesId] })
        });
        const tombstoneConfirmed = await api(
            backend.baseUrl,
            `/v1/alarms/pending?deviceId=${encodeURIComponent(alarmFixture.deviceId)}&reconcile=true&limit=200`,
            { headers: authHeaders(alarmFixture.deviceToken) }
        );
        assert.ok(!tombstoneConfirmed.cancellations.some((entry) => entry.seriesId === leased.seriesId));
        pass("ALARM-DB-04 cancellation tombstones retry until client confirmation");

        const acknowledgementFixture = await resetFixture(backend.baseUrl, "series-ack", { runtimeState: "idle" });
        await putMemory(
            backend.baseUrl,
            acknowledgementFixture.deviceToken,
            "tasks",
            "# Planned Work\n- [ ] Series acknowledgement probe\n"
        );
        await runTool(backend.baseUrl, acknowledgementFixture.userId, "start_session", {
            task_id: "Series acknowledgement probe",
            estimated_minutes: 25
        });
        const acknowledgementGeneration = await api(
            backend.baseUrl,
            `/v1/alarms/pending?deviceId=${encodeURIComponent(acknowledgementFixture.deviceId)}&reconcile=true&limit=200`,
            { headers: authHeaders(acknowledgementFixture.deviceToken) }
        );
        const acknowledged = acknowledgementGeneration.alarms[0];
        await api(backend.baseUrl, "/v1/alarms/reconcile", {
            method: "POST",
            headers: authHeaders(acknowledgementFixture.deviceToken),
            body: JSON.stringify({
                deviceId: acknowledgementFixture.deviceId,
                scheduled: [{
                    alarmId: acknowledged.id,
                    deliveryToken: acknowledged.deliveryToken,
                    localAlarmId: `notification:${acknowledged.id}`
                }],
                cancelledSeriesIds: []
            })
        });
        await api(backend.baseUrl, `/v1/alarms/${encodeURIComponent(acknowledged.id)}/ack`, {
            method: "POST",
            headers: authHeaders(acknowledgementFixture.deviceToken),
            body: JSON.stringify({
                deviceId: acknowledgementFixture.deviceId,
                action: "ack",
                at: new Date().toISOString()
            })
        });
        const acknowledgementTombstones = await api(
            backend.baseUrl,
            `/v1/alarms/pending?deviceId=${encodeURIComponent(acknowledgementFixture.deviceId)}&reconcile=true&limit=200`,
            { headers: authHeaders(acknowledgementFixture.deviceToken) }
        );
        const acknowledgedSeries = acknowledgementTombstones.cancellations.find(
            (entry) => entry.seriesId === acknowledged.seriesId
        );
        assert.equal(acknowledgedSeries.localAlarmIds.length, 0, "acknowledged local alarm is already handled and unscheduled siblings need no local cancellation");
        pass("ALARM-DB-05 acknowledgement cancels every sibling in the exact generation");

        await putMemory(
            backend.baseUrl,
            fixture.deviceToken,
            "tasks",
            "# Planned Work\n- [ ] Write backend userflow tests\n"
        );

        let result = await runTool(backend.baseUrl, fixture.userId, "start_session", {
            task_id: "Write backend userflow tests",
            estimated_minutes: 25
        });
        assert.equal(result.ok, true, result.result);
        assertState(result.snapshot, "working");
        assertAlarmFamily(result.snapshot, "session_alarm");
        pass("UF-02 start_session enters working");

        result = await runTool(backend.baseUrl, fixture.userId, "extend_session", {
            extension_minutes: 10
        });
        assert.equal(result.ok, true, result.result);
        assertState(result.snapshot, "working");
        assertAlarmFamily(result.snapshot, "session_alarm");
        pass("UF-03 extend_session replaces work alarms");

        result = await runTool(backend.baseUrl, fixture.userId, "end_session", {
            actual_minutes: 25,
            productive_level: 80
        });
        assert.equal(result.ok, true, result.result);
        assertState(result.snapshot, "idle");
        assertAlarmFamily(result.snapshot, "idle_alarm");
        pass("UF-04 end_session enters noisy idle");

        result = await runTool(backend.baseUrl, fixture.userId, "start_break", {
            duration_minutes: 15
        });
        assert.equal(result.ok, true, result.result);
        assertState(result.snapshot, "break");
        assertAlarmFamily(result.snapshot, "break_alarm");
        pass("UF-05 start_break enters break");

        result = await runTool(backend.baseUrl, fixture.userId, "start_sleep", {
            estimated_hours: 8
        });
        assert.equal(result.ok, true, result.result);
        assertState(result.snapshot, "sleeping");
        assertAlarmFamily(result.snapshot, "wake_alarm");
        assert.match(result.snapshot.runtimeState.metadata, /sleep_metrics/u);
        pass("UF-06 start_sleep enters sleeping");

        result = await runTool(backend.baseUrl, fixture.userId, "log_wake", {
            sleep_quality: 4
        });
        assert.equal(result.ok, true, result.result);
        assertState(result.snapshot, "idle");
        assertAlarmFamily(result.snapshot, "idle_alarm");
        const completedSleepSamples = result.snapshot.sleepSampleCount;
        result = await runTool(backend.baseUrl, fixture.userId, "log_wake", {
            sleep_quality: 4
        });
        assert.equal(result.ok, true, result.result);
        assert.equal(result.snapshot.sleepSampleCount, completedSleepSamples);
        pass("UF-07 log_wake enters noisy idle");
        pass("MEM-DB-09 repeated wake without a new sleep start does not add a completed sample");

        result = await runTool(backend.baseUrl, fixture.userId, "start_vacation", {
            reason: "family travel"
        });
        assert.equal(result.ok, true, result.result);
        assertState(result.snapshot, "vacation");
        assertNoAlarms(result.snapshot);
        pass("UF-08 start_vacation is quiet");

        result = await runTool(backend.baseUrl, fixture.userId, "end_vacation", {});
        assert.equal(result.ok, true, result.result);
        assertState(result.snapshot, "idle");
        assertAlarmFamily(result.snapshot, "idle_alarm");
        pass("UF-09 end_vacation enters noisy idle");

        result = await runTool(backend.baseUrl, fixture.userId, "wake_up_alarm", {});
        assert.equal(result.ok, true, result.result);
        assertState(result.snapshot, "sleeping");
        assertAlarmFamily(result.snapshot, "wake_alarm");
        pass("UF-10 wake_up_alarm enters sleeping");

        result = await runTool(backend.baseUrl, fixture.userId, "end_session", {
            actual_minutes: 0,
            productive_level: 0
        });
        assert.equal(result.ok, false, "zero-minute end_session must be rejected");
        assert.match(result.result, /actual_minutes must be between 1 and 1440/u);
        assertState(result.snapshot, "sleeping");
        assertAlarmFamily(result.snapshot, "wake_alarm");
        pass("UF-11 invalid end_session has zero side effects");

        result = await runTool(backend.baseUrl, fixture.userId, "start_break", {});
        assert.equal(result.ok, false, "missing duration must be rejected");
        assert.match(result.result, /invalid arguments for start_break/u);
        assertState(result.snapshot, "sleeping");
        assertAlarmFamily(result.snapshot, "wake_alarm");
        pass("UF-12 malformed tool arguments have zero side effects");

        const pending = await api(
            backend.baseUrl,
            `/v1/alarms/pending?device_id=${encodeURIComponent(fixture.deviceId)}`,
            { headers: authHeaders(fixture.deviceToken) }
        );
        assert.ok(Array.isArray(pending));
        assert.ok(pending.length > 0, "expected pending wake alarms before ack");
        const first = pending[0];
        assert.equal(first.kind, "wake_alarm");
        await api(backend.baseUrl, `/v1/alarms/${encodeURIComponent(first.id)}/ack`, {
            method: "POST",
            headers: authHeaders(fixture.deviceToken),
            body: JSON.stringify({
                deviceId: fixture.deviceId,
                action: "ack",
                at: new Date().toISOString()
            })
        });
        state = await snapshot(backend.baseUrl, fixture.userId, fixture.deviceId);
        assertState(state, "sleeping");
        assert.equal(alarmCount(state, "wake_alarm"), 0);
        pass("UF-11 grouped alarm ack clears alarm family without state churn");

        result = await runTool(backend.baseUrl, fixture.userId, "log_wake", {
            sleep_quality: 4
        });
        assert.equal(result.ok, true, result.result);
        assertState(result.snapshot, "idle");
        const pendingIdle = await api(
            backend.baseUrl,
            `/v1/alarms/pending?device_id=${encodeURIComponent(fixture.deviceId)}`,
            { headers: authHeaders(fixture.deviceToken) }
        );
        assert.ok(pendingIdle.length > 0, "expected pending idle alarms after wake");
        assert.equal(pendingIdle[0].kind, "idle_alarm");
        await api(backend.baseUrl, `/v1/alarms/${encodeURIComponent(pendingIdle[0].id)}/ack`, {
            method: "POST",
            headers: authHeaders(fixture.deviceToken),
            body: JSON.stringify({
                deviceId: fixture.deviceId,
                action: "ack",
                at: new Date().toISOString()
            })
        });

        const routine = await getMemory(backend.baseUrl, fixture.deviceToken, "routine");
        assert.doesNotMatch(
            routine.content,
            /Work Blocks|Sleep|Vacation|Gym|Relationship|girlfriend/iu
        );
        result = await runTool(backend.baseUrl, fixture.userId, "set_routine_categories", {
            categories: [
                {
                    name: "Reading",
                    description: "Fixed daily reading block.",
                    cadence: "daily",
                    target_minutes: 30
                }
            ],
            source: "User asked for reading as a fixed 30 minute daily allocation."
        });
        assert.equal(result.ok, true, result.result);
        const updatedRoutine = await getMemory(backend.baseUrl, fixture.deviceToken, "routine");
        assert.match(updatedRoutine.content, /Reading: Fixed daily reading block.*30 mins/isu);
        state = await snapshot(backend.baseUrl, fixture.userId, fixture.deviceId);
        assertState(state, "idle");
        assert.equal(alarmCount(state, "idle_alarm"), 0, "routine patch should not create alarms");
        pass("UF-12 routine categories update without state churn");

        const personality = await getMemory(backend.baseUrl, fixture.deviceToken, "personality");
        assert.match(personality.content, /Strict but intelligent (?:sports )?coach/u);
        result = await runTool(backend.baseUrl, fixture.userId, "patch_file", {
            file_path: "personality.md",
            patch: "<<<<<<< SEARCH\n\n=======\n- User prefers the coach to be concise and concrete.\n>>>>>>> REPLACE"
        });
        assert.equal(result.ok, true, result.result);
        const updatedPersonality = await getMemory(backend.baseUrl, fixture.deviceToken, "personality");
        assert.match(updatedPersonality.content, /concise and concrete/u);
        state = await snapshot(backend.baseUrl, fixture.userId, fixture.deviceId);
        assertState(state, "idle");
        pass("UF-13 personality.md patches without state churn");

        const coachTodo = await getMemory(backend.baseUrl, fixture.deviceToken, "coach_todo");
        assert.match(coachTodo.content, /Coach Todo/u);
        result = await runTool(backend.baseUrl, fixture.userId, "patch_file", {
            file_path: "coach_todo.txt",
            patch: "<<<<<<< SEARCH\n\n=======\n- Ask for short-term goals if onboarding has not captured them.\n>>>>>>> REPLACE"
        });
        assert.equal(result.ok, true, result.result);
        const updatedCoachTodo = await getMemory(backend.baseUrl, fixture.deviceToken, "coach_todo");
        assert.match(updatedCoachTodo.content, /short-term goals/u);
        state = await snapshot(backend.baseUrl, fixture.userId, fixture.deviceId);
        assertState(state, "idle");
        pass("UF-14 coach_todo.txt patches without state churn");

        await putMemory(
            backend.baseUrl,
            fixture.deviceToken,
            "behavior",
            `# Behavior Memory\n\n${"drift loop evidence ".repeat(1000)}\n`
        );
        const report = await contextReport(backend.baseUrl, fixture.userId);
        assert.equal(report.ok, true);
        assert.equal(report.report.provider, "gemini");
        assert.equal(report.report.model, "gemini-3.5-flash");
        assert.ok(report.report.systemPromptChars > 1000, "expected non-trivial prompt");
        assert.ok(report.report.memory.truncatedSections.includes("behavior"), "expected oversized behavior to truncate");
        assert.ok(report.report.memory.totalInjectedChars <= report.report.memory.totalMemoryBudgetChars);
        pass("UF-15 context report budgets oversized memory");

        result = await runTool(backend.baseUrl, fixture.userId, "memory_search", {
            query: "drift loop evidence",
            limit: 3
        });
        assert.equal(result.ok, true, result.result);
        assert.match(result.result, /Relevant memory found/u);
        assert.match(result.result, /behavior/u);
        pass("UF-16 semantic memory search degrades to lexical without embedding keys");

        const adminReport = await adminContextReport(backend.baseUrl, fixture.userId);
        assert.equal(adminReport.ok, true);
        assert.equal(adminReport.userId, fixture.userId);
        assert.equal(adminReport.sleepMetrics.sleepSampleCount >= 1, true);
        pass("UF-17 production admin context report returns diagnostics");

        console.log("backend no-LLM userflow tests passed");
    } finally {
        await backend.stop();
    }
}

main().catch((error) => {
    console.error(error);
    process.exit(1);
});
