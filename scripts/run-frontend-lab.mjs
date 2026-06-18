#!/usr/bin/env node

import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const mode = process.argv[2] ?? "dev";
const frontendDir = path.join(repoRoot, "apps", "frontend");
const backendUrl = process.env.NEXT_PUBLIC_ANTIROT_BACKEND_URL || "https://api.antirot.org";
const port = process.env.PORT || "3000";

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

const localEnv = readDotEnv(path.join(repoRoot, ".env"));
const vpsEnv = readVpsEnv();
const adminToken =
    process.env.NEXT_PUBLIC_ANTIROT_ADMIN_TOKEN ||
    vpsEnv.ANTIROT_ADMIN_TOKEN ||
    process.env.ANTIROT_ADMIN_TOKEN ||
    localEnv.ANTIROT_ADMIN_TOKEN ||
    "";
const deviceToken =
    process.env.NEXT_PUBLIC_ANTIROT_DEVICE_TOKEN ||
    vpsEnv.ANTIROT_DEVICE_TOKEN ||
    process.env.ANTIROT_DEVICE_TOKEN ||
    localEnv.ANTIROT_DEVICE_TOKEN ||
    "";

const nextEnv = {
    ...process.env,
    NEXT_PUBLIC_ANTIROT_BACKEND_URL: backendUrl,
    NEXT_PUBLIC_ANTIROT_ADMIN_TOKEN: adminToken,
    NEXT_PUBLIC_ANTIROT_DEVICE_TOKEN: deviceToken
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
