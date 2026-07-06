import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import net from "node:net";
import fs from "node:fs";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";

export const repoRoot = path.resolve(import.meta.dirname, "..");
export const defaultAdminToken = "test-admin-token";
const postgresContainerName = "antirot-postgres";

export function readDotEnv() {
    const envPath = path.join(repoRoot, ".env");
    if (!fs.existsSync(envPath)) {
        return {};
    }

    const result = {};
    const content = fs.readFileSync(envPath, "utf8");
    const regex = /^\s*([A-Za-z0-9_]+)\s*=\s*(?:'([^']*)'|"([^"]*)"|([^#\r\n]*))/gmu;
    let match;
    while ((match = regex.exec(content)) !== null) {
        const key = match[1];
        const value = match[2] ?? match[3] ?? match[4].trim();
        result[key] = value;
    }
    return result;
}

export function resolveTailoredLlmConfig(extraEnv = {}) {
    const dotEnv = readDotEnv();
    const hasVertexCredentials = Boolean(
        extraEnv.GOOGLE_CLOUD_CREDENTIALS
        || dotEnv.GOOGLE_CLOUD_CREDENTIALS
        || process.env.GOOGLE_CLOUD_CREDENTIALS
    );
    const tailoredProvider = hasVertexCredentials
        ? "vertex"
        : extraEnv.ANTIROT_TAILORED_LLM_PROVIDER
        || dotEnv.ANTIROT_TAILORED_LLM_PROVIDER
        || process.env.ANTIROT_TAILORED_LLM_PROVIDER
        || "gemini";
    const tailoredModel = tailoredProvider === "vertex"
        ? "google/gemini-3.5-flash"
        : extraEnv.ANTIROT_TAILORED_LLM_MODEL
        || dotEnv.ANTIROT_TAILORED_LLM_MODEL
        || process.env.ANTIROT_TAILORED_LLM_MODEL
        || "gemini-3.5-flash";

    return {
        dotEnv,
        hasVertexCredentials,
        tailoredProvider,
        tailoredModel,
        tailoredKey: extraEnv.ANTIROT_TAILORED_LLM_KEY
            || dotEnv.ANTIROT_TAILORED_LLM_KEY
            || dotEnv.GEMINI_API_KEY
            || process.env.ANTIROT_TAILORED_LLM_KEY
            || process.env.GEMINI_API_KEY
            || ""
    };
}

export async function startBackend(extraEnv = {}) {
    const {
        dotEnv,
        hasVertexCredentials,
        tailoredProvider,
        tailoredModel,
        tailoredKey
    } = resolveTailoredLlmConfig(extraEnv);

    const externalBaseUrl = extraEnv.ANTIROT_BACKEND_URL
        || process.env.ANTIROT_BACKEND_URL
        || dotEnv.ANTIROT_BACKEND_URL
        || "";
    if (externalBaseUrl) {
        const baseUrl = externalBaseUrl.replace(/\/+$/u, "");
        console.log(`Using external backend userflow target: ${baseUrl}`);
        await waitForHealth(baseUrl);
        return {
            baseUrl,
            output: () => "",
            stop: async () => {}
        };
    }

    const env = {
        ...dotEnv,
        ...process.env,
        ...extraEnv,
        ANTIROT_BACKEND_BIND: "127.0.0.1:0",
        ANTIROT_ADMIN_TOKEN: defaultAdminToken,
        ANTIROT_DEVICE_TOKEN: "test-device-token",
        ANTIROT_ENABLE_TEST_ENDPOINTS: "1",
        ANTIROT_MEMORY_GEMINI_API_KEY: "",
        ANTIROT_MEMORY_VOYAGE_API_KEY: "",
        ANTIROT_TAILORED_LLM_PROVIDER: tailoredProvider,
        ANTIROT_TAILORED_LLM_MODEL: tailoredModel,
        ANTIROT_TAILORED_LLM_KEY: tailoredKey
    };

    await ensureLocalPostgres(env.DATABASE_URL || "");
    console.log(`Starting backend with tailored LLM provider=${tailoredProvider} model=${tailoredModel} vertexCredentials=${hasVertexCredentials ? "present" : "absent"}`);

    const child = spawn("cargo", ["run", "--manifest-path", "apps/backend/Cargo.toml", "--bin", "antirot-backend"], {
        cwd: repoRoot,
        env,
        stdio: ["ignore", "pipe", "pipe"]
    });

    let output = "";
    let baseUrl;
    const startup = new Promise((resolve, reject) => {
        const timeout = setTimeout(() => {
            reject(new Error(`backend did not start in time\n${output}`));
        }, 60_000);

        function onData(chunk) {
            const text = chunk.toString();
            output += text;
            const match = output.match(/"bind":"(127\.0\.0\.1:\d+)"/u);
            if (match) {
                clearTimeout(timeout);
                baseUrl = `http://${match[1]}`;
                resolve(baseUrl);
            }
        }

        child.stdout.on("data", onData);
        child.stderr.on("data", onData);
        child.on("exit", (code) => {
            if (!baseUrl) {
                clearTimeout(timeout);
                reject(new Error(`backend exited before binding with code ${code}\n${output}`));
            }
        });
    });

    baseUrl = await startup;
    await waitForHealth(baseUrl);

    return {
        baseUrl,
        output: () => output,
        stop: async () => {
            if (child.exitCode !== null) {
                return;
            }
            child.kill("SIGTERM");
            await new Promise((resolve) => child.once("exit", resolve));
        }
    };
}

function isLocalHost(hostname) {
    return hostname === "localhost" || hostname === "127.0.0.1" || hostname === "::1";
}

function commandExists(command) {
    const result = spawnSync(command, ["--version"], {
        cwd: repoRoot,
        encoding: "utf8",
        stdio: ["ignore", "ignore", "ignore"]
    });
    return result.status === 0;
}

function findContainerRuntime() {
    if (commandExists("podman")) {
        return "podman";
    }
    if (commandExists("docker")) {
        return "docker";
    }
    return "";
}

function canConnectTcp(host, targetPort) {
    return new Promise((resolve) => {
        const socket = net.createConnection({ host, port: targetPort });
        const done = (ok) => {
            socket.removeAllListeners();
            socket.destroy();
            resolve(ok);
        };
        socket.setTimeout(1000);
        socket.once("connect", () => done(true));
        socket.once("error", () => done(false));
        socket.once("timeout", () => done(false));
    });
}

async function waitForPostgres(host, targetPort) {
    for (let attempt = 0; attempt < 30; attempt += 1) {
        if (await canConnectTcp(host, targetPort)) {
            return true;
        }
        await delay(1000);
    }
    return false;
}

function containerExists(runtime) {
    const result = spawnSync(runtime, ["inspect", postgresContainerName], {
        cwd: repoRoot,
        encoding: "utf8",
        stdio: ["ignore", "ignore", "ignore"]
    });
    return result.status === 0;
}

function runContainerCommand(runtime, args, description) {
    const result = spawnSync(runtime, args, {
        cwd: repoRoot,
        encoding: "utf8",
        stdio: "inherit"
    });
    if (result.status !== 0) {
        throw new Error(`${description} failed with exit code ${result.status ?? "unknown"}`);
    }
}

async function ensureLocalPostgres(databaseUrl) {
    if (process.env.ANTIROT_SKIP_DB_SETUP === "1" || !databaseUrl) {
        return;
    }

    let parsed;
    try {
        parsed = new URL(databaseUrl);
    } catch {
        return;
    }

    if (!parsed.protocol.startsWith("postgres") || !isLocalHost(parsed.hostname)) {
        return;
    }

    const dbHost = parsed.hostname;
    const dbPort = Number(parsed.port || "5432");
    if (await canConnectTcp(dbHost, dbPort)) {
        return;
    }

    const runtime = findContainerRuntime();
    if (!runtime) {
        throw new Error(
            [
                `Local Postgres is not listening on ${dbHost}:${dbPort}.`,
                "Install Docker or Podman, start Postgres manually, or set ANTIROT_SKIP_DB_SETUP=1 to bypass the automatic test DB setup."
            ].join(" ")
        );
    }

    const dbName = decodeURIComponent(parsed.pathname.replace(/^\//, "") || "antirot_backend");
    const dbUser = decodeURIComponent(parsed.username || "antirot_backend");
    const dbPassword = decodeURIComponent(parsed.password || "antirot_backend");

    if (containerExists(runtime)) {
        console.log(`Starting existing ${postgresContainerName} container with ${runtime}.`);
        runContainerCommand(runtime, ["start", postgresContainerName], "Postgres container start");
    } else {
        console.log(`Creating ${postgresContainerName} local Postgres container with ${runtime}.`);
        runContainerCommand(
            runtime,
            [
                "run",
                "-d",
                "--name",
                postgresContainerName,
                "-e",
                `POSTGRES_USER=${dbUser}`,
                "-e",
                `POSTGRES_PASSWORD=${dbPassword}`,
                "-e",
                `POSTGRES_DB=${dbName}`,
                "-p",
                `127.0.0.1:${dbPort}:5432`,
                "postgres:16-alpine"
            ],
            "Postgres container create"
        );
    }

    if (!(await waitForPostgres(dbHost, dbPort))) {
        throw new Error(`Postgres did not start listening on ${dbHost}:${dbPort} within 30s.`);
    }
}

async function waitForHealth(baseUrl) {
    for (let attempt = 0; attempt < 60; attempt += 1) {
        try {
            const response = await fetch(`${baseUrl}/v1/health`);
            if (response.ok) {
                return;
            }
        } catch {
            // Retry until the listener is actually serving.
        }
        await new Promise((resolve) => setTimeout(resolve, 250));
    }
    throw new Error(`backend health did not become ready at ${baseUrl}`);
}

export async function api(baseUrl, pathName, options = {}) {
    const response = await fetch(`${baseUrl}${pathName}`, {
        ...options,
        headers: {
            "Content-Type": "application/json",
            ...(options.headers ?? {})
        }
    });
    const text = await response.text();
    let body;
    try {
        body = text ? JSON.parse(text) : {};
    } catch {
        body = { raw: text };
    }
    if (!response.ok) {
        throw new Error(`${options.method ?? "GET"} ${pathName} failed HTTP ${response.status}: ${text}`);
    }
    return body;
}

export function authHeaders(token = process.env.ANTIROT_ADMIN_TOKEN || readDotEnv().ANTIROT_ADMIN_TOKEN || defaultAdminToken) {
    return { Authorization: `Bearer ${token}` };
}

export async function resetFixture(baseUrl, label, options = {}) {
    const safe = label.replace(/[^a-z0-9-]/giu, "-").toLowerCase();
    const userId = `userflow-${safe}-${Date.now()}`;
    const deviceId = `device-${safe}-${Date.now()}`;
    const deviceToken = `token-${safe}-${Date.now()}`;
    const snapshot = await api(baseUrl, "/v1/test/reset", {
        method: "POST",
        headers: authHeaders(),
        body: JSON.stringify({ userId, deviceId, deviceToken, runtimeState: options.runtimeState })
    });
    return { userId, deviceId, deviceToken, snapshot };
}

export async function runTool(baseUrl, userId, name, args = {}) {
    return await api(baseUrl, "/v1/test/tool", {
        method: "POST",
        headers: authHeaders(),
        body: JSON.stringify({ userId, name, args })
    });
}

export async function snapshot(baseUrl, userId, deviceId) {
    return await api(baseUrl, `/v1/test/state?userId=${encodeURIComponent(userId)}&deviceId=${encodeURIComponent(deviceId)}`, {
        headers: authHeaders()
    });
}

export async function contextReport(baseUrl, userId, provider = "gemini", model = "gemini-3.5-flash") {
    return await api(
        baseUrl,
        `/v1/test/context?userId=${encodeURIComponent(userId)}&provider=${encodeURIComponent(provider)}&model=${encodeURIComponent(model)}`,
        { headers: authHeaders() }
    );
}

export async function adminContextReport(baseUrl, userId, provider = "gemini", model = "gemini-3.5-flash") {
    return await api(
        baseUrl,
        `/v1/admin/context?userId=${encodeURIComponent(userId)}&provider=${encodeURIComponent(provider)}&model=${encodeURIComponent(model)}`,
        { headers: authHeaders() }
    );
}

export async function putMemory(baseUrl, token, key, content) {
    return await api(baseUrl, `/v1/memory/${encodeURIComponent(key)}`, {
        method: "PUT",
        headers: authHeaders(token),
        body: JSON.stringify({ content })
    });
}

export async function getMemory(baseUrl, token, key) {
    return await api(baseUrl, `/v1/memory/${encodeURIComponent(key)}`, {
        headers: authHeaders(token)
    });
}

export function alarmCount(state, kind, severity) {
    return state.alarmCounts
        .filter((entry) => entry.kind === kind && (!severity || entry.severity === severity))
        .reduce((sum, entry) => sum + Number(entry.count), 0);
}

export function assertState(state, expected) {
    assert.equal(state.runtimeState?.state, expected, `expected runtime state ${expected}`);
}

export function assertNoAlarms(state) {
    assert.deepEqual(state.alarmCounts, [], "expected no pending alarms");
}

export function assertAlarmFamily(state, kind) {
    assert.equal(alarmCount(state, kind, "normal"), 2, `expected two normal ${kind} alarms`);
    assert.equal(alarmCount(state, kind, "loud"), 59, `expected fifty-nine loud ${kind} alarms`);
    assert.equal(alarmCount(state, kind), 61, `expected sixty-one total ${kind} alarms`);
    const other = state.alarmCounts.filter((entry) => entry.kind !== kind);
    assert.deepEqual(other, [], `expected no other alarm families, got ${JSON.stringify(other)}`);
}

export function assertNoBackendLeak(reply) {
    const forbidden = [
        /^\s*State:/imu,
        /\bstart_session\b/iu,
        /\bend_session\b/iu,
        /\bstart_break\b/iu,
        /\bstart_sleep\b/iu,
        /\bpatch_file\b/iu,
        /\bmemory_search\b/iu,
        /\bidle_alarm\b/iu,
        /\bsession_alarm\b/iu,
        /\bbreak_alarm\b/iu,
        /\bwake_alarm\b/iu,
        /\buser_runtime/iu,
        /```json|\{[\s\S]{0,200}"(?:args|deviceId|runtimeState|state|tool|userId)"\s*:/u,
        /\bSQL\b/u,
        /\bmust push back\b/iu,
        /\btool call\b/iu,
        /\binternal tool names?\b/iu,
        /\binternal configuration\b/iu,
        /\btechnical parameters?\b/iu,
        /\bthis interface\b/iu,
        /\bsystem payloads?\b/iu,
        /\bbackend configuration\b/iu,
        /\bconfiguration parameters?\b/iu,
        /\btool names?\s*(?:are|:)/iu,
        /\btools? (?:are )?locked\b/iu,
        /\bbackend state\s*(?:is|:|\{)/iu,
        /\braw payloads?\s*(?:are|:|\{)/iu,
        /\b(?:backend|internal|runtime) state machine\b|\bstate machine\s*(?:details|architecture|transition)/iu,
        /\bbaseline parameters\b/iu,
        /\bhidden context\b/iu,
        /\bsaved fields?\b/iu,
        /\bprofile setup\b/iu,
        /\bprofile updates?\b/iu,
        /\bprofile (?:is )?updated\b/iu,
        /\bprofile has been updated\b/iu,
        /\btimezone (?:is )?updated\b/iu,
        /\btimezone (?:is )?locked\b/iu,
        /\b(?:session|task|block|sleep|work)\s+(?:is\s+)?logged\b/iu,
        /\blogged and closed\b/iu,
        /\bsession\s+(?:is\s+)?closed\b/iu,
        /\bmemory (?:is )?updated\b/iu,
        /^\s*#+\s*(?:reasoning summary|analytical assessment|analysis|reasoning)\b/imu,
        /\bpersonality profile (?:is )?updated\b/iu,
        /\bpersonality updated\b/iu,
        /\bpersonality configuration\b/iu,
        /\bconfiguration (?:is )?locked\b/iu,
        /\bmemory files?\b/iu,
        /\bmemory logs?\b/iu,
        /\b[A-Za-z0-9_-]+\.md\b/u
    ];
    for (const pattern of forbidden) {
        assert.doesNotMatch(reply, pattern, `reply leaked backend internals: ${reply}`);
    }
}

export function assertProductionQuality(reply) {
    assert.ok(reply.trim().length >= 12, `reply too short: ${reply}`);
    assert.ok(reply.trim().length <= 1200, `reply too long: ${reply}`);
    assertNoBackendLeak(reply);
    assert.doesNotMatch(reply, /\bgreat job\b|\bproud of you\b|\bamazing\b/iu, "reply used generic praise");
}

export function pass(name, detail = "") {
    console.log(`PASS ${name}${detail ? ` - ${detail}` : ""}`);
}
