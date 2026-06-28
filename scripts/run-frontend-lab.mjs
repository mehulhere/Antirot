#!/usr/bin/env node

import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import net from "node:net";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const mode = process.argv[2] ?? "dev";
const frontendDir = path.join(repoRoot, "apps", "frontend");
const backendUrl = process.env.NEXT_PUBLIC_ANTIROT_BACKEND_URL || "https://api.antirot.org";
const port = process.env.PORT || "3000";
const postgresContainerName = "antirot-postgres";

function readDotEnv(filePath) {
    if (!fs.existsSync(filePath)) {
        return {};
    }
    const env = {};
    for (const rawLine of fs.readFileSync(filePath, "utf8").split(/\r?\n/)) {
        const line = rawLine.trim();
        if (!line || line.startsWith("#")) {
            continue;
        }
        const index = line.indexOf("=");
        if (index === -1) {
            continue;
        }
        const key = line.slice(0, index).trim();
        let value = line.slice(index + 1).trim();
        if (
            (value.startsWith("\"") && value.endsWith("\"")) ||
            (value.startsWith("'") && value.endsWith("'"))
        ) {
            value = value.slice(1, -1);
        }
        env[key] = value;
    }
    return env;
}

function readVpsEnv() {
    if (process.env.ANTIROT_FRONTEND_USE_VPS_ENV === "0") {
        return {};
    }
    const result = spawnSync(
        "ssh",
        [
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=8",
            "antirot@antirot.org",
            "python3 - <<'PY'\nfrom pathlib import Path\nkeys = {'ANTIROT_ADMIN_TOKEN', 'ANTIROT_DEVICE_TOKEN'}\ntry:\n    text = Path('/etc/antirot/backend.env').read_text()\nexcept Exception:\n    raise SystemExit(0)\nfor line in text.splitlines():\n    if '=' not in line or line.lstrip().startswith('#'):\n        continue\n    key, value = line.split('=', 1)\n    if key in keys:\n        print(f'{key}={value}')\nPY"
        ],
        {
            cwd: repoRoot,
            encoding: "utf8",
            stdio: ["ignore", "pipe", "ignore"]
        }
    );
    if (result.status !== 0 || !result.stdout.trim()) {
        return {};
    }
    const env = {};
    for (const line of result.stdout.split(/\r?\n/)) {
        const index = line.indexOf("=");
        if (index > 0) {
            env[line.slice(0, index)] = line.slice(index + 1);
        }
    }
    return env;
}

function isLocalHost(hostname) {
    return hostname === "localhost" || hostname === "127.0.0.1" || hostname === "::1";
}

function shouldEnsureLocalPostgres() {
    if (process.env.ANTIROT_FRONTEND_FORCE_DB_SETUP === "1") {
        return true;
    }
    try {
        return isLocalHost(new URL(backendUrl).hostname);
    } catch {
        return false;
    }
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
    if (
        process.env.ANTIROT_FRONTEND_SKIP_DB_SETUP === "1" ||
        mode !== "dev" ||
        !databaseUrl ||
        !shouldEnsureLocalPostgres()
    ) {
        return;
    }

    let parsed;
    try {
        parsed = new URL(databaseUrl);
    } catch {
        console.warn("Skipping local Postgres setup because DATABASE_URL is not a valid URL.");
        return;
    }

    if (!parsed.protocol.startsWith("postgres") || !isLocalHost(parsed.hostname)) {
        return;
    }

    const dbHost = parsed.hostname;
    const dbPort = Number(parsed.port || "5432");
    if (await canConnectTcp(dbHost, dbPort)) {
        console.log(`Local Postgres is already listening on ${dbHost}:${dbPort}.`);
        return;
    }

    const runtime = findContainerRuntime();
    if (!runtime) {
        throw new Error(
            [
                `Local Postgres is not listening on ${dbHost}:${dbPort}.`,
                "Install Docker or Podman, or start Postgres manually.",
                "Set ANTIROT_FRONTEND_SKIP_DB_SETUP=1 to skip this frontend launcher check."
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
    console.log(`Local Postgres is ready on ${dbHost}:${dbPort}.`);
}

const localEnv = readDotEnv(path.join(repoRoot, ".env"));
const backendEnv = readDotEnv(path.join(repoRoot, "apps", "backend", ".env"));
await ensureLocalPostgres(process.env.DATABASE_URL || localEnv.DATABASE_URL || backendEnv.DATABASE_URL || "");
const vpsEnv = readVpsEnv();
const adminToken =
    process.env.NEXT_PUBLIC_ANTIROT_ADMIN_TOKEN ||
    vpsEnv.ANTIROT_ADMIN_TOKEN ||
    process.env.ANTIROT_ADMIN_TOKEN ||
    localEnv.ANTIROT_ADMIN_TOKEN ||
    backendEnv.ANTIROT_ADMIN_TOKEN ||
    "";
const deviceToken =
    process.env.NEXT_PUBLIC_ANTIROT_DEVICE_TOKEN ||
    vpsEnv.ANTIROT_DEVICE_TOKEN ||
    process.env.ANTIROT_DEVICE_TOKEN ||
    localEnv.ANTIROT_DEVICE_TOKEN ||
    backendEnv.ANTIROT_DEVICE_TOKEN ||
    "";
const googleWebClientId =
    process.env.NEXT_PUBLIC_GOOGLE_WEB_CLIENT_ID ||
    process.env.GOOGLE_WEB_CLIENT_ID ||
    localEnv.NEXT_PUBLIC_GOOGLE_WEB_CLIENT_ID ||
    localEnv.GOOGLE_WEB_CLIENT_ID ||
    backendEnv.NEXT_PUBLIC_GOOGLE_WEB_CLIENT_ID ||
    backendEnv.GOOGLE_WEB_CLIENT_ID ||
    "";

const nextEnv = {
    ...process.env,
    NEXT_PUBLIC_ANTIROT_BACKEND_URL: backendUrl,
    NEXT_PUBLIC_ANTIROT_ADMIN_TOKEN: adminToken,
    NEXT_PUBLIC_ANTIROT_DEVICE_TOKEN: deviceToken,
    NEXT_PUBLIC_GOOGLE_WEB_CLIENT_ID: googleWebClientId
};

const tokenSource = vpsEnv.ANTIROT_ADMIN_TOKEN ? "VPS env" : adminToken ? "local env" : "missing";
console.log(`Antirot Lab backend: ${backendUrl}`);
console.log(`Antirot Lab auth source: ${tokenSource}`);

const nextArgs = mode === "build" ? ["next", "build"] : ["next", "dev", "-p", port];
const child = spawn("npx", nextArgs, {
    cwd: frontendDir,
    env: nextEnv,
    stdio: "inherit"
});

child.on("exit", (code, signal) => {
    if (signal) {
        process.kill(process.pid, signal);
    }
    process.exit(code ?? 1);
});
