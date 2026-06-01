import { execFile, exec as execWithShell } from "node:child_process";
import { promisify } from "node:util";
const execFileAsync = promisify(execFile);
const execWithShellAsync = promisify(execWithShell);
function randomJitterPercent() {
    return 0.05 + Math.random() * 0.05;
}
function applyHiddenTimeBuffer(delayMins) {
    const requestedDelayMins = Math.max(1, Math.round(delayMins));
    const jitterPercent = randomJitterPercent();
    return {
        requestedDelayMins,
        scheduledDelayMins: Math.max(1, Math.round(requestedDelayMins * (1 + jitterPercent))),
        jitterPercent
    };
}
export async function scheduleCronReminder(params) {
    const buffered = applyHiddenTimeBuffer(params.delayMins);
    if (params.config.enableCron === false) {
        return {
            ok: false,
            message: "🔴 FALLBACK: cron disabled - Reason: plugins.entries.antirot.config.enableCron=false - Impact: reminder recorded in state only",
            ...buffered
        };
    }
    const command = params.config.openclawCommand?.trim() || "openclaw";
    try {
        const { stdout } = await execFileAsync(command, [
            "cron",
            "add",
            "--name",
            params.name,
            "--at",
            `${buffered.scheduledDelayMins}m`,
            "--session",
            "main",
            "--system-event",
            params.systemEvent,
            "--wake",
            "now",
            "--delete-after-run",
            "--json"
        ]);
        return {
            ok: true,
            message: "Scheduled with a hidden buffer.",
            cronJobId: extractCronJobId(stdout),
            ...buffered
        };
    }
    catch (error) {
        return {
            ok: false,
            message: `🔴 FALLBACK: cron scheduling failed - Reason: ${error instanceof Error ? error.message : String(error)} - Impact: reminder recorded in state only`,
            ...buffered
        };
    }
}
export async function cancelCronReminder(params) {
    if (!params.cronJobId) {
        return {
            ok: false,
            message: "🔴 FALLBACK: cron removal skipped - Reason: no cron job id was recorded - Impact: Antirot registry cleared, but stale cron callback may still arrive and must be ignored"
        };
    }
    const command = params.config.openclawCommand?.trim() || "openclaw";
    try {
        await execFileAsync(command, ["cron", "rm", params.cronJobId, "--json"]);
        return {
            ok: true,
            message: "Cron reminder removed.",
            cronJobId: params.cronJobId
        };
    }
    catch (error) {
        return {
            ok: false,
            message: `🔴 FALLBACK: cron removal failed - Reason: ${error instanceof Error ? error.message : String(error)} - Impact: Antirot registry cleared, but stale cron callback may still arrive and must be ignored`,
            cronJobId: params.cronJobId
        };
    }
}
function extractCronJobId(stdout) {
    const text = stdout.toString().trim();
    if (!text) {
        return undefined;
    }
    try {
        const parsed = JSON.parse(text);
        return findStringValue(parsed, ["id", "jobId"]) ?? findNestedCronJobId(parsed);
    }
    catch {
        return undefined;
    }
}
function findStringValue(value, keys) {
    if (!value || typeof value !== "object") {
        return undefined;
    }
    const record = value;
    for (const key of keys) {
        const candidate = record[key];
        if (typeof candidate === "string" && candidate.trim()) {
            return candidate.trim();
        }
    }
    return undefined;
}
function findNestedCronJobId(value) {
    if (!value || typeof value !== "object") {
        return undefined;
    }
    const record = value;
    return findStringValue(record.job, ["id", "jobId"])
        ?? findStringValue(record.result, ["id", "jobId"])
        ?? findStringValue(record.cronJob, ["id", "jobId"]);
}
export async function triggerAlarmCommand(config) {
    const command = config.alarmCommand?.trim();
    if (!command) {
        return {
            ok: false,
            message: "🔴 FALLBACK: loud alarm command missing - Reason: plugins.entries.antirot.config.alarmCommand is not set - Impact: only urgent text reminder can be sent"
        };
    }
    try {
        await execWithShellAsync(command, { timeout: 30_000 });
        return { ok: true, message: "Loud alarm command executed." };
    }
    catch (error) {
        return {
            ok: false,
            message: `🔴 FALLBACK: loud alarm command failed - Reason: ${error instanceof Error ? error.message : String(error)} - Impact: only urgent text reminder can be sent`
        };
    }
}
export async function triggerNormalAlarmCommand(config) {
    const command = config.normalAlarmCommand?.trim();
    if (!command) {
        return {
            ok: false,
            message: "🔴 FALLBACK: normal alarm command missing - Reason: plugins.entries.antirot.config.normalAlarmCommand is not set - Impact: only wake-up text reminder can be sent"
        };
    }
    try {
        await execWithShellAsync(command, { timeout: 30_000 });
        return { ok: true, message: "Normal alarm command executed." };
    }
    catch (error) {
        return {
            ok: false,
            message: `🔴 FALLBACK: normal alarm command failed - Reason: ${error instanceof Error ? error.message : String(error)} - Impact: only wake-up text reminder can be sent`
        };
    }
}
export async function scheduleBridgeAlarm(params) {
    const bridgeUrl = normalizeBridgeUrl(params.config.bridgeUrl);
    const adminToken = params.config.bridgeAdminToken?.trim() || process.env.ANTIROT_ADMIN_TOKEN?.trim();
    if (!bridgeUrl || !adminToken) {
        return {
            ok: false,
            message: "🔴 FALLBACK: phone alarm skipped - Reason: bridgeUrl/bridgeAdminToken is not configured - Impact: only chat/cron escalation can run"
        };
    }
    const deviceResult = await resolveBridgeDeviceId({
        bridgeUrl,
        adminToken,
        config: params.config
    });
    if (!deviceResult.ok || !deviceResult.deviceId) {
        return deviceResult;
    }
    const alarmId = `antirot-${params.severity}-${Date.now()}`;
    const fireAt = new Date(Date.now() + Math.max(1, Math.round(params.fireDelayMins)) * 60_000);
    const expiresAt = new Date(fireAt.getTime() + 30 * 60_000);
    try {
        const response = await fetch(`${bridgeUrl}/v1/alarms`, {
            method: "POST",
            headers: {
                "Authorization": `Bearer ${adminToken}`,
                "Content-Type": "application/json"
            },
            body: JSON.stringify({
                id: alarmId,
                deviceId: deviceResult.deviceId,
                kind: params.severity === "loud" ? "non_response" : "routine_overdue",
                severity: params.severity,
                title: params.title,
                message: params.message,
                fireAt: fireAt.toISOString(),
                hiddenBufferApplied: true,
                requiresAcknowledgement: true,
                expiresAt: expiresAt.toISOString()
            })
        });
        const body = await response.text();
        if (!response.ok) {
            return {
                ok: false,
                message: `🔴 FALLBACK: phone alarm queue failed - Reason: bridge returned HTTP ${response.status}: ${body.slice(0, 300)} - Impact: only chat/cron escalation can run`,
                deviceId: deviceResult.deviceId,
                alarmId
            };
        }
        return {
            ok: true,
            message: `Phone ${params.severity} alarm queued through bridge.`,
            deviceId: deviceResult.deviceId,
            alarmId
        };
    }
    catch (error) {
        return {
            ok: false,
            message: `🔴 FALLBACK: phone alarm queue failed - Reason: ${error instanceof Error ? error.message : String(error)} - Impact: only chat/cron escalation can run`,
            deviceId: deviceResult.deviceId,
            alarmId
        };
    }
}
async function resolveBridgeDeviceId(params) {
    const configuredDevice = params.config.bridgeDeviceId?.trim() || process.env.ANTIROT_BRIDGE_DEVICE_ID?.trim();
    if (configuredDevice) {
        return {
            ok: true,
            message: "Using configured bridge device id.",
            deviceId: configuredDevice
        };
    }
    const workspaceId = params.config.bridgeWorkspaceId?.trim() || process.env.ANTIROT_WORKSPACE_ID?.trim() || "main";
    try {
        const response = await fetch(`${params.bridgeUrl}/v1/workspaces/${encodeURIComponent(workspaceId)}/devices`, {
            headers: {
                "Authorization": `Bearer ${params.adminToken}`
            }
        });
        const body = await response.text();
        if (!response.ok) {
            return {
                ok: false,
                message: `🔴 FALLBACK: phone device lookup failed - Reason: bridge returned HTTP ${response.status}: ${body.slice(0, 300)} - Impact: configure bridgeDeviceId or pair the app`
            };
        }
        const parsed = JSON.parse(body);
        const deviceId = parsed.devices?.[0]?.deviceId;
        if (!deviceId) {
            return {
                ok: false,
                message: `🔴 FALLBACK: phone device lookup failed - Reason: no paired devices found for workspace ${workspaceId} - Impact: run bridge pairing or configure bridgeDeviceId`
            };
        }
        return {
            ok: true,
            message: `Resolved paired device for workspace ${workspaceId}.`,
            deviceId
        };
    }
    catch (error) {
        return {
            ok: false,
            message: `🔴 FALLBACK: phone device lookup failed - Reason: ${error instanceof Error ? error.message : String(error)} - Impact: configure bridgeDeviceId or pair the app`
        };
    }
}
function normalizeBridgeUrl(value) {
    const trimmed = value?.trim() || process.env.ANTIROT_BRIDGE_URL?.trim() || "https://api.antirot.org";
    return trimmed.replace(/\/+$/, "");
}
