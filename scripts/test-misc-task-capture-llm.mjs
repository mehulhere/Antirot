/* global AbortSignal */

import assert from "node:assert/strict";

import {
    api,
    assertAlarmFamily,
    assertProductionQuality,
    assertState,
    authHeaders,
    getMemory,
    pass,
    putMemory,
    resetFixture,
    runTool,
    snapshot,
    startBackend
} from "./backend-userflow-test-lib.mjs";

async function chat(baseUrl, token, message) {
    const body = await api(baseUrl, "/v1/chat", {
        method: "POST",
        signal: AbortSignal.timeout(90_000),
        headers: authHeaders(token),
        body: JSON.stringify({ message })
    });
    assert.equal(body.ok, true);
    assert.equal(typeof body.reply, "string");
    assertProductionQuality(body.reply);
    return body.reply;
}

async function main() {
    const backend = await startBackend();
    try {
        const fixture = await resetFixture(backend.baseUrl, "misc-task-capture-llm");

        await putMemory(
            backend.baseUrl,
            fixture.deviceToken,
            "tasks",
            "# Task Pipeline\n- [ ] Write backend userflow tests\n"
        );
        await putMemory(
            backend.baseUrl,
            fixture.deviceToken,
            "miscellaneous_todo",
            "# Miscellaneous Todo\n"
        );

        const startResult = await runTool(backend.baseUrl, fixture.userId, "start_session", {
            task_id: "Write backend userflow tests",
            estimated_minutes: 25
        });
        assert.equal(startResult.ok, true, startResult.result);
        assertState(startResult.snapshot, "working");
        assertAlarmFamily(startResult.snapshot, "session_alarm");

        const reply = await chat(
            backend.baseUrl,
            fixture.deviceToken,
            "I just remembered: later I need to order printer paper. Add that to my task list for later, but keep me on the current work session."
        );

        const state = await snapshot(backend.baseUrl, fixture.userId, fixture.deviceId);
        assertState(state, "working");
        assertAlarmFamily(state, "session_alarm");

        const tasks = await getMemory(backend.baseUrl, fixture.deviceToken, "tasks");
        const misc = await getMemory(backend.baseUrl, fixture.deviceToken, "miscellaneous_todo");

        assert.match(
            misc.content,
            /order printer paper/iu,
            `Expected remembered side task in miscellaneous_todo.md.\nReply: ${reply}\nMisc content:\n${misc.content}`
        );
        assert.doesNotMatch(
            tasks.content,
            /order printer paper/iu,
            `Expected active task pipeline to stay focused on current executable work.\nReply: ${reply}\nTasks content:\n${tasks.content}`
        );

        pass("misc remembered task is captured in miscellaneous_todo.md", reply.replace(/\s+/gu, " ").slice(0, 220));
        console.log("misc task capture LLM test passed");
    } finally {
        await backend.stop();
    }
}

main().catch((error) => {
    console.error(error);
    process.exit(1);
});
