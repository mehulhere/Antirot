/* global URL, Response, console */

import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { scheduleBridgeAlarm } from "../dist/runtime.js";
import {
    createAntirotTrigger,
    clearMatchingTriggers,
    listActiveTriggers
} from "../dist/triggers.js";
import { ensureWorkspace, readStats } from "../dist/storage.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const workspaceDir = path.resolve(__dirname, "../test-workspace-recent");

async function cleanWorkspace() {
    await fs.rm(workspaceDir, { recursive: true, force: true });
    await fs.mkdir(workspaceDir, { recursive: true });
    await ensureWorkspace(workspaceDir);
}

function withMockFetch(handler) {
    const originalFetch = globalThis.fetch;
    const calls = [];
    globalThis.fetch = async (input, init = {}) => {
        const url = input instanceof URL ? input.toString() : String(input);
        calls.push({ url, init });
        return await handler({ url, init, calls });
    };
    return async () => {
        globalThis.fetch = originalFetch;
        return calls;
    };
}

function jsonResponse(value, status = 200) {
    return new Response(JSON.stringify(value), {
        status,
        headers: { "Content-Type": "application/json" }
    });
}

function textResponse(value, status = 200) {
    return new Response(value, { status });
}

function parseJsonBody(call) {
    assert.equal(typeof call.init.body, "string");
    return JSON.parse(call.init.body);
}

async function testMissingBridgeConfigFallsBack() {
    const result = await scheduleBridgeAlarm({
        config: { enableCron: false, bridgeUrl: "https://api.antirot.org" },
        severity: "normal",
        title: "Antirot",
        message: "Come back",
        fireDelayMins: 1
    });

    assert.equal(result.ok, false);
    assert.match(result.message, /bridgeUrl\/bridgeAdminToken is not configured/u);
}

async function testConfiguredDeviceQueuesNormalAlarm() {
    const restore = withMockFetch(async ({ url, init }) => {
        assert.equal(url, "https://api.antirot.org/v1/alarms");
        assert.equal(init.method, "POST");
        assert.equal(init.headers.Authorization, "Bearer admin-token");
        return jsonResponse({ ok: true });
    });

    const before = Date.now();
    const result = await scheduleBridgeAlarm({
        config: {
            bridgeUrl: "https://api.antirot.org/",
            bridgeAdminToken: "admin-token",
            bridgeDeviceId: "iphone-123"
        },
        severity: "normal",
        title: "Antirot",
        message: "Come back",
        fireDelayMins: 1
    });
    const calls = await restore();

    assert.equal(result.ok, true);
    assert.equal(result.deviceId, "iphone-123");
    assert.equal(calls.length, 1);

    const body = parseJsonBody(calls[0]);
    assert.equal(body.deviceId, "iphone-123");
    assert.equal(body.severity, "normal");
    assert.equal(body.kind, "routine_overdue");
    assert.equal(body.hiddenBufferApplied, true);
    assert.equal(body.requiresAcknowledgement, true);

    const fireAt = Date.parse(body.fireAt);
    assert.ok(fireAt >= before + 55_000, "fireAt should be about one minute out");
    assert.ok(fireAt <= before + 70_000, "fireAt should not drift far past one minute");
}

async function testWorkspaceLookupThenQueuesLoudAlarm() {
    const restore = withMockFetch(async ({ url, init }) => {
        if (url === "https://api.antirot.org/v1/workspaces/main/devices") {
            assert.equal(init.headers.Authorization, "Bearer admin-token");
            return jsonResponse({
                ok: true,
                workspaceId: "main",
                devices: [
                    {
                        deviceId: "paired-iphone",
                        platform: "ios",
                        notificationCapability: "remote_notification"
                    }
                ]
            });
        }
        if (url === "https://api.antirot.org/v1/alarms") {
            return jsonResponse({ ok: true });
        }
        throw new Error(`Unexpected URL: ${url}`);
    });

    const result = await scheduleBridgeAlarm({
        config: {
            bridgeUrl: "https://api.antirot.org",
            bridgeAdminToken: "admin-token",
            bridgeWorkspaceId: "main"
        },
        severity: "loud",
        title: "Antirot loud alarm",
        message: "Enough disappearing",
        fireDelayMins: 1
    });
    const calls = await restore();

    assert.equal(result.ok, true);
    assert.equal(result.deviceId, "paired-iphone");
    assert.equal(calls.length, 2);

    const body = parseJsonBody(calls[1]);
    assert.equal(body.deviceId, "paired-iphone");
    assert.equal(body.severity, "loud");
    assert.equal(body.kind, "non_response");
}

async function testWorkspaceLookupNoDeviceFailsCleanly() {
    const restore = withMockFetch(async () => jsonResponse({ ok: true, devices: [] }));
    const result = await scheduleBridgeAlarm({
        config: {
            bridgeUrl: "https://api.antirot.org",
            bridgeAdminToken: "admin-token",
            bridgeWorkspaceId: "empty"
        },
        severity: "normal",
        title: "Antirot",
        message: "Come back",
        fireDelayMins: 1
    });
    const calls = await restore();

    assert.equal(result.ok, false);
    assert.equal(calls.length, 1);
    assert.match(result.message, /no paired devices found/u);
}

async function testBridgeAlarmErrorDoesNotPretendSuccess() {
    const restore = withMockFetch(async ({ url }) => {
        if (url.endsWith("/v1/workspaces/main/devices")) {
            return jsonResponse({ ok: true, devices: [{ deviceId: "paired-iphone" }] });
        }
        return textResponse("bad alarm request", 400);
    });

    const result = await scheduleBridgeAlarm({
        config: {
            bridgeUrl: "https://api.antirot.org",
            bridgeAdminToken: "admin-token",
            bridgeWorkspaceId: "main"
        },
        severity: "normal",
        title: "Antirot",
        message: "Come back",
        fireDelayMins: 1
    });
    const calls = await restore();

    assert.equal(result.ok, false);
    assert.equal(result.deviceId, "paired-iphone");
    assert.equal(calls.length, 2);
    assert.match(result.message, /HTTP 400/u);
}

async function testAlarmEscalationTriggerLifecycle() {
    await cleanWorkspace();
    const config = { enableCron: false };

    const first = await createAntirotTrigger({
        workspaceDir,
        config,
        kind: "alarm_escalation",
        scope: "daily",
        label: "normalCount=1",
        reason: "user did not return",
        delayMins: 10,
        cronName: "antirot-phone-alarm-escalation",
        systemEvent: "callback should let the LLM decide whether to clear or ring"
    });

    assert.equal(first.trigger.status, "active");
    assert.equal(first.trigger.requestedDelayMins, 10);
    assert.equal(first.cron.ok, false);
    assert.match(first.cron.message, /cron disabled/u);

    let active = await listActiveTriggers(workspaceDir);
    assert.equal(active.length, 1);
    assert.equal(active[0].kind, "alarm_escalation");

    await clearMatchingTriggers({
        workspaceDir,
        config,
        kinds: ["alarm_escalation"],
        reason: "LLM decided the user genuinely returned"
    });

    active = await listActiveTriggers(workspaceDir);
    assert.equal(active.length, 0);
}

async function testStatsShapeIncludesAlarmCounters() {
    await cleanWorkspace();
    const stats = await readStats(workspaceDir);
    assert.deepEqual(stats.normalAlarmsTriggered, {});
    assert.deepEqual(stats.loudAlarmsTriggered, {});
}

async function run() {
    await testMissingBridgeConfigFallsBack();
    await testConfiguredDeviceQueuesNormalAlarm();
    await testWorkspaceLookupThenQueuesLoudAlarm();
    await testWorkspaceLookupNoDeviceFailsCleanly();
    await testBridgeAlarmErrorDoesNotPretendSuccess();
    await testAlarmEscalationTriggerLifecycle();
    await testStatsShapeIncludesAlarmCounters();
    console.log("recent alarm flow tests passed");
}

await run();
