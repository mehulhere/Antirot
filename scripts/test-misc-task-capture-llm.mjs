/* global AbortSignal */

import assert from "node:assert/strict";
import crypto from "node:crypto";

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

const runEnabled = process.env.ANTIROT_RUN_LLM_USERFLOW_TESTS === "1";
const retryDelaysMs = [2_000, 5_000, 10_000];

async function chat(baseUrl, token, message) {
    const requestId = crypto.randomUUID();
    let body;
    for (let attempt = 0; attempt <= retryDelaysMs.length; attempt += 1) {
        try {
            body = await api(baseUrl, "/v1/chat", {
                method: "POST",
                signal: AbortSignal.timeout(90_000),
                headers: authHeaders(token),
                body: JSON.stringify({ requestId, message })
            });
            break;
        } catch (error) {
            const text = error instanceof Error ? error.message : String(error);
            const canRetry = /502 Bad Gateway|503 Service Unavailable|upstream service temporarily unavailable|429 Too Many Requests|UNAVAILABLE|TimedOut|timeout|RESOURCE_EXHAUSTED|quota exceeded|LLM API request failed|Connection reset|Token request failed/iu.test(text);
            if (!canRetry || attempt >= retryDelaysMs.length) {
                throw error;
            }
            const delayMs = retryDelaysMs[attempt];
            console.log(`LLM unavailable; retrying misc chat after ${Math.round(delayMs / 1000)}s: ${text}`);
            await new Promise((resolve) => setTimeout(resolve, delayMs));
        }
    }
    assert.ok(body, "LLM chat retry loop completed without a response");
    assert.equal(body.ok, true);
    assert.equal(typeof body.reply, "string");
    assertProductionQuality(body.reply);
    return body.reply;
}

function assertPlannedTaskWithEstimate(content, taskPattern, durationPattern, detail) {
    assert.match(
        content,
        taskPattern,
        `Expected planned task in tasks.md for ${detail}.\nTasks content:\n${content}`
    );
    assert.match(
        content,
        durationPattern,
        `Expected estimated duration in tasks.md for ${detail}.\nTasks content:\n${content}`
    );
}

async function main() {
    if (!runEnabled) {
        console.log("Skipping misc task capture LLM tests. Set ANTIROT_RUN_LLM_USERFLOW_TESTS=1 to run them.");
        return;
    }

    const backend = await startBackend();
    try {
        const miscFixture = await resetFixture(backend.baseUrl, "misc-task-capture-llm");

        await putMemory(
            backend.baseUrl,
            miscFixture.deviceToken,
            "tasks",
            "# Task Pipeline\n- [ ] Write backend userflow tests\n"
        );
        await putMemory(
            backend.baseUrl,
            miscFixture.deviceToken,
            "miscellaneous_todo",
            "# Miscellaneous Todo\n"
        );

        const startResult = await runTool(backend.baseUrl, miscFixture.userId, "start_session", {
            task_id: "Write backend userflow tests",
            estimated_minutes: 25
        });
        assert.equal(startResult.ok, true, startResult.result);
        assertState(startResult.snapshot, "working");
        assertAlarmFamily(startResult.snapshot, "session_alarm");

        const reply = await chat(
            backend.baseUrl,
            miscFixture.deviceToken,
            "I just remembered: later I need to order printer paper. Add that to my task list for later, but keep me on the current work session."
        );

        const state = await snapshot(backend.baseUrl, miscFixture.userId, miscFixture.deviceId);
        assertState(state, "working");
        assertAlarmFamily(state, "session_alarm");

        const tasks = await getMemory(backend.baseUrl, miscFixture.deviceToken, "tasks");
        const misc = await getMemory(backend.baseUrl, miscFixture.deviceToken, "miscellaneous_todo");

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

        const midSessionFixture = await resetFixture(backend.baseUrl, "planned-task-during-session-llm");
        await putMemory(
            backend.baseUrl,
            midSessionFixture.deviceToken,
            "tasks",
            "# Task Pipeline\n- [ ] Write backend userflow tests\n"
        );
        const midStartResult = await runTool(backend.baseUrl, midSessionFixture.userId, "start_session", {
            task_id: "Write backend userflow tests",
            estimated_minutes: 25
        });
        assert.equal(midStartResult.ok, true, midStartResult.result);

        const midReply = await chat(
            backend.baseUrl,
            midSessionFixture.deviceToken,
            "Add a planned task for after this session: draft the onboarding QA notes for 2 hours. Keep this current work session running."
        );

        const midState = await snapshot(backend.baseUrl, midSessionFixture.userId, midSessionFixture.deviceId);
        assertState(midState, "working");
        assertAlarmFamily(midState, "session_alarm");

        const midTasks = await getMemory(backend.baseUrl, midSessionFixture.deviceToken, "tasks");
        const midMisc = await getMemory(backend.baseUrl, midSessionFixture.deviceToken, "miscellaneous_todo");
        assertPlannedTaskWithEstimate(
            midTasks.content,
            /draft (?:the )?onboarding qa notes/iu,
            /\b(?:2\s*h(?:ours?)?|120\s*min(?:ute)?s?)\b/iu,
            `mid-session task add\nReply: ${midReply}`
        );
        assert.doesNotMatch(
            midMisc.content,
            /draft (?:the )?onboarding qa notes/iu,
            `Expected planned task with estimate to avoid miscellaneous_todo.md.\nReply: ${midReply}\nMisc content:\n${midMisc.content}`
        );
        pass("planned task with estimate can be added during a session", midReply.replace(/\s+/gu, " ").slice(0, 220));

        const afterSessionFixture = await resetFixture(backend.baseUrl, "planned-task-after-session-llm");
        await putMemory(
            backend.baseUrl,
            afterSessionFixture.deviceToken,
            "tasks",
            "# Task Pipeline\n- [ ] Write backend userflow tests\n"
        );
        const afterStartResult = await runTool(backend.baseUrl, afterSessionFixture.userId, "start_session", {
            task_id: "Write backend userflow tests",
            estimated_minutes: 25
        });
        assert.equal(afterStartResult.ok, true, afterStartResult.result);
        const afterEndResult = await runTool(backend.baseUrl, afterSessionFixture.userId, "end_session", {
            actual_minutes: 25,
            productive_level: 80
        });
        assert.equal(afterEndResult.ok, true, afterEndResult.result);

        const afterReply = await chat(
            backend.baseUrl,
            afterSessionFixture.deviceToken,
            "Add the next planned task: review the launch checklist for 1.5 hours."
        );

        const afterTasks = await getMemory(backend.baseUrl, afterSessionFixture.deviceToken, "tasks");
        const afterMisc = await getMemory(backend.baseUrl, afterSessionFixture.deviceToken, "miscellaneous_todo");
        assertPlannedTaskWithEstimate(
            afterTasks.content,
            /review the launch checklist/iu,
            /\b(?:1\.5\s*h(?:ours?)?|90\s*min(?:ute)?s?)\b/iu,
            `post-session task add\nReply: ${afterReply}`
        );
        assert.doesNotMatch(
            afterMisc.content,
            /review the launch checklist/iu,
            `Expected post-session planned task with estimate to avoid miscellaneous_todo.md.\nReply: ${afterReply}\nMisc content:\n${afterMisc.content}`
        );
        pass("planned task with estimate can be added after a session", afterReply.replace(/\s+/gu, " ").slice(0, 220));
        console.log("misc task capture LLM test passed");
    } finally {
        await backend.stop();
    }
}

main().catch((error) => {
    console.error(error);
    process.exit(1);
});
