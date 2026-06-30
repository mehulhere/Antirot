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

        await putMemory(
            backend.baseUrl,
            fixture.deviceToken,
            "tasks",
            "# Task Pipeline\n- [ ] Write backend userflow tests\n"
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
        const durableAfterSleep = await getMemory(backend.baseUrl, fixture.deviceToken, "durable");
        assert.match(durableAfterSleep.content, /Distilled from daily logs via good_night/u);
        pass("UF-06 start_sleep enters sleeping and distills memory");

        result = await runTool(backend.baseUrl, fixture.userId, "log_wake", {
            sleep_quality: 4
        });
        assert.equal(result.ok, true, result.result);
        assertState(result.snapshot, "idle");
        assertAlarmFamily(result.snapshot, "idle_alarm");
        pass("UF-07 log_wake enters noisy idle");

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
        assert.equal(result.ok, true, result.result);
        assertAlarmFamily(result.snapshot, "idle_alarm");

        const pending = await api(
            backend.baseUrl,
            `/v1/alarms/pending?device_id=${encodeURIComponent(fixture.deviceId)}`,
            { headers: authHeaders(fixture.deviceToken) }
        );
        assert.ok(Array.isArray(pending));
        assert.ok(pending.length > 0, "expected pending idle alarms before ack");
        const first = pending[0];
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
        assert.equal(alarmCount(state, "idle_alarm"), 0);
        pass("UF-11 grouped alarm ack clears alarm family");

        const routine = await getMemory(backend.baseUrl, fixture.deviceToken, "routine");
        assert.match(routine.content, /Gym: 60 mins/u);
        assert.match(routine.content, /girlfriend: 45 mins/u);
        result = await runTool(backend.baseUrl, fixture.userId, "patch_file", {
            file_path: "routine.md",
            patch: "<<<<<<< SEARCH\n\n=======\n- Reading: 30 mins\n>>>>>>> REPLACE"
        });
        assert.equal(result.ok, true, result.result);
        const updatedRoutine = await getMemory(backend.baseUrl, fixture.deviceToken, "routine");
        assert.match(updatedRoutine.content, /Reading: 30 mins/u);
        state = await snapshot(backend.baseUrl, fixture.userId, fixture.deviceId);
        assertState(state, "idle");
        assert.equal(alarmCount(state, "idle_alarm"), 0, "routine patch should not create alarms");
        pass("UF-12 routine.md patches without state churn");

        const personality = await getMemory(backend.baseUrl, fixture.deviceToken, "personality");
        assert.match(personality.content, /Strict but intelligent sports coach/u);
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
        pass("UF-14 context report budgets oversized memory");

        result = await runTool(backend.baseUrl, fixture.userId, "memory_search", {
            query: "drift loop evidence",
            limit: 3
        });
        assert.equal(result.ok, true, result.result);
        assert.match(result.result, /Relevant memory found/u);
        assert.match(result.result, /behavior/u);
        pass("UF-15 semantic memory search degrades to lexical without embedding keys");

        const adminReport = await adminContextReport(backend.baseUrl, fixture.userId);
        assert.equal(adminReport.ok, true);
        assert.equal(adminReport.userId, fixture.userId);
        assert.equal(adminReport.sleepMetrics.sleepSampleCount >= 1, true);
        pass("UF-16 production admin context report returns diagnostics");

        console.log("backend no-LLM userflow tests passed");
    } finally {
        await backend.stop();
    }
}

main().catch((error) => {
    console.error(error);
    process.exit(1);
});
