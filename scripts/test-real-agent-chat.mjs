/* global console, process, URL */

import assert from "node:assert/strict";
import { createServer } from "node:http";
import { execFile } from "node:child_process";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

const runEnabled = process.env.ANTIROT_RUN_REAL_AGENT_TESTS === "1";
const hasModelKey = [
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "GOOGLE_API_KEY",
    "GEMINI_API_KEY",
    "OPENROUTER_API_KEY"
].some((key) => Boolean(process.env[key]?.trim()));

if (!runEnabled) {
    console.log("Skipping real-agent chat test. Set ANTIROT_RUN_REAL_AGENT_TESTS=1 to run it.");
    process.exit(0);
}

if (!hasModelKey) {
    throw new Error("Real-agent chat test needs a provider key in the shell, such as OPENAI_API_KEY, ANTHROPIC_API_KEY, GOOGLE_API_KEY, GEMINI_API_KEY, or OPENROUTER_API_KEY.");
}

const repoRoot = path.resolve(import.meta.dirname, "..");
const workspaceDir = await fs.mkdtemp(path.join(os.tmpdir(), "antirot-real-agent-"));
const sessionId = `antirot-real-agent-${Date.now()}`;
const bridgeAdminToken = "real-agent-test-admin";
const bridgeDeviceId = "real-agent-test-device";
const bridgeRequests = [];

function readJsonBody(request) {
    return new Promise((resolve, reject) => {
        let body = "";
        request.setEncoding("utf8");
        request.on("data", (chunk) => {
            body += chunk;
        });
        request.on("end", () => {
            try {
                resolve(body ? JSON.parse(body) : {});
            } catch (error) {
                reject(error);
            }
        });
        request.on("error", reject);
    });
}

const bridge = createServer(async (request, response) => {
    try {
        const url = new URL(request.url ?? "/", "http://127.0.0.1");
        const authorization = request.headers.authorization ?? "";
        if (authorization !== `Bearer ${bridgeAdminToken}`) {
            response.writeHead(401, { "Content-Type": "application/json" });
            response.end(JSON.stringify({ error: "unauthorized" }));
            return;
        }

        if (request.method === "GET" && url.pathname === "/v1/workspaces/main/devices") {
            bridgeRequests.push({ method: request.method, path: url.pathname });
            response.writeHead(200, { "Content-Type": "application/json" });
            response.end(JSON.stringify({
                ok: true,
                workspaceId: "main",
                devices: [{ deviceId: bridgeDeviceId, platform: "ios", notificationCapability: "remote_notification" }]
            }));
            return;
        }

        if (request.method === "POST" && url.pathname === "/v1/alarms") {
            const body = await readJsonBody(request);
            bridgeRequests.push({ method: request.method, path: url.pathname, body });
            response.writeHead(200, { "Content-Type": "application/json" });
            response.end(JSON.stringify({ ok: true, alarm: body, delivery: { mode: "test", status: "queued" } }));
            return;
        }

        response.writeHead(404, { "Content-Type": "application/json" });
        response.end(JSON.stringify({ error: "not found" }));
    } catch (error) {
        response.writeHead(500, { "Content-Type": "application/json" });
        response.end(JSON.stringify({ error: error instanceof Error ? error.message : String(error) }));
    }
});

await new Promise((resolve) => bridge.listen(0, "127.0.0.1", resolve));
const address = bridge.address();
assert.ok(address && typeof address === "object");
const bridgeUrl = `http://127.0.0.1:${address.port}`;

async function runAgentTurn(message) {
    const env = {
        ...process.env,
        ANTIROT_WORKSPACE_DIR: workspaceDir,
        ANTIROT_BRIDGE_URL: bridgeUrl,
        ANTIROT_ADMIN_TOKEN: bridgeAdminToken,
        ANTIROT_BRIDGE_DEVICE_ID: bridgeDeviceId
    };
    const { stdout, stderr } = await execFileAsync(
        "npx",
        [
            "openclaw",
            "agent",
            "--local",
            "--session-id",
            sessionId,
            "--message",
            message,
            "--json",
            "--timeout",
            "240"
        ],
        {
            cwd: repoRoot,
            env,
            maxBuffer: 10 * 1024 * 1024,
            timeout: 260_000
        }
    );
    if (stderr.trim()) {
        console.error(stderr);
    }
    return stdout.trim();
}

function activeTriggersFromWorkspace() {
    return fs.readFile(path.join(workspaceDir, ".antirot", "triggers.json"), "utf8")
        .then((text) => JSON.parse(text).triggers.filter((trigger) => trigger.status === "active"));
}

try {
    console.log(`Running real-agent chat test with session ${sessionId}`);

    await runAgentTurn([
        "Automated Antirot integration test, turn 1.",
        "Use the Antirot tool startAlarm now with reason 'real agent chat test non-response'.",
        "After the tool call, reply in one short sentence containing REAL_AGENT_TURN_1_DONE."
    ].join("\n"));

    const alarmPost = bridgeRequests.find((request) => request.method === "POST" && request.path === "/v1/alarms");
    assert.ok(alarmPost, "agent should have caused startAlarm to queue a bridge alarm");
    assert.equal(alarmPost.body.deviceId, bridgeDeviceId);
    assert.equal(alarmPost.body.severity, "normal");
    assert.equal(alarmPost.body.hiddenBufferApplied, true);

    let activeTriggers = await activeTriggersFromWorkspace();
    assert.ok(activeTriggers.some((trigger) => trigger.kind === "alarm_escalation"), "startAlarm should arm an alarm escalation trigger");

    await runAgentTurn([
        "Automated Antirot integration test, turn 2 in the same session.",
        "Use list_active_triggers. Do not clear anything yet.",
        "Reply with REAL_AGENT_TURN_2_DONE and mention whether an alarm_escalation trigger exists."
    ].join("\n"));

    activeTriggers = await activeTriggersFromWorkspace();
    const escalation = activeTriggers.find((trigger) => trigger.kind === "alarm_escalation");
    assert.ok(escalation, "same chat should still have the alarm escalation active until the LLM clears it");

    await runAgentTurn([
        "Automated Antirot integration test, turn 3 in the same session.",
        "The test is resolved. Use clear_active_trigger on the active alarm_escalation trigger.",
        "Reply with REAL_AGENT_TURN_3_DONE after clearing it."
    ].join("\n"));

    activeTriggers = await activeTriggersFromWorkspace();
    assert.equal(activeTriggers.filter((trigger) => trigger.kind === "alarm_escalation").length, 0);

    console.log("real-agent chat test passed");
} finally {
    await new Promise((resolve) => bridge.close(resolve));
    if (process.env.ANTIROT_KEEP_REAL_AGENT_WORKSPACE !== "1") {
        await fs.rm(workspaceDir, { recursive: true, force: true });
    } else {
        console.log(`Kept real-agent workspace at ${workspaceDir}`);
    }
}
