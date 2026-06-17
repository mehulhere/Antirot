#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const defaultExamplePath = path.join(repoRoot, "env.example.txt");
const defaultEnvPath = path.join(repoRoot, ".env");

const args = process.argv.slice(2);
const envPath = path.resolve(args[0] ?? defaultEnvPath);
const examplePath = path.resolve(args[1] ?? defaultExamplePath);

const requiredBackendKeys = new Set([
    "ANTIROT_BACKEND_BIND",
    "DATABASE_URL",
    "ANTIROT_ADMIN_TOKEN",
    "ANTIROT_DEVICE_TOKEN",
    "GOOGLE_IOS_CLIENT_ID",
    "ANTIROT_WORKSPACE_ID",
    "GOOGLE_CLOUD_CREDENTIALS",
    "RUST_LOG",
    "FIREWORKS_BASE_URL",
    "FIREWORKS_AUDIO_BASE_URL",
    "FIREWORKS_API_KEY",
    "FIREWORKS_STT_MODEL",
    "ASYNC_BASE_URL",
    "ASYNC_API_KEY",
    "ASYNC_TTS_MODEL",
    "ANTIROT_MEMORY_EMBEDDING_MODEL",
    "ANTIROT_MEMORY_EMBEDDING_FALLBACK_MODEL",
    "ANTIROT_MEMORY_GEMINI_API_KEY",
]);

const optionalButUsefulKeys = new Set([
    "ANTIROT_APNS_ENV",
    "ANTIROT_APNS_TEAM_ID",
    "ANTIROT_APNS_KEY_ID",
    "ANTIROT_APNS_PRIVATE_KEY_PATH",
    "ANTIROT_APNS_TOPIC",
    "ASYNC_TTS_API_KEY",
    "ASYNC_TTS_VOICE_ID",
    "ANTIROT_MEMORY_VOYAGE_API_KEY",
]);

const optionalKeys = new Set([
    "CROF_BASE_URL",
    "CROF_API_KEY",
    "ANTIROT_JUDGE_MODEL",
    "ANTIROT_JUDGE_EFFORT_LEVEL",
    "ANTIROT_JUDGE_MIN_OVERALL",
    "ANTIROT_JUDGE_MIN_DIMENSION",
    "ANTIROT_WORKSPACE_DIR",
    "ANTIROT_BACKEND_URL",
    "ANTIROT_BACKEND_DEVICE_ID",
]);

function readEnvFile(filePath) {
    if (!fs.existsSync(filePath)) {
        throw new Error(`Env file not found: ${filePath}`);
    }

    const entries = new Map();
    const duplicateKeys = [];
    const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/);

    for (const [index, rawLine] of lines.entries()) {
        const line = rawLine.trim();
        if (!line || line.startsWith("#")) {
            continue;
        }

        const equalsIndex = line.indexOf("=");
        if (equalsIndex === -1) {
            continue;
        }

        const key = line.slice(0, equalsIndex).trim();
        const value = stripQuotes(line.slice(equalsIndex + 1).trim());
        if (entries.has(key)) {
            duplicateKeys.push({ key, line: index + 1 });
        }
        entries.set(key, { value, line: index + 1 });
    }

    return { entries, duplicateKeys };
}

function stripQuotes(value) {
    if (
        (value.startsWith("\"") && value.endsWith("\"")) ||
        (value.startsWith("'") && value.endsWith("'"))
    ) {
        return value.slice(1, -1);
    }
    return value;
}

function isPlaceholder(value) {
    const normalized = value.trim();
    return (
        normalized === "" ||
        normalized.includes("CHANGE_") ||
        normalized.includes("YOUR_") ||
        normalized.includes("PASTE_") ||
        normalized.includes("change-me") ||
        normalized.includes("XXXXXXXXXX") ||
        normalized === "api.yourdomain.com"
    );
}

function displayValue(value) {
    if (!value) {
        return "<empty>";
    }
    if (/TOKEN|KEY|PASSWORD|SECRET|CREDENTIAL/i.test(value)) {
        return "<redacted>";
    }
    if (value.length > 80) {
        return `${value.slice(0, 77)}...`;
    }
    return value;
}

function printGroup(title, items) {
    if (items.length === 0) {
        return;
    }
    console.log(`\n${title}`);
    for (const item of items) {
        console.log(`- ${item}`);
    }
}

let example;
let target;
try {
    example = readEnvFile(examplePath);
    target = readEnvFile(envPath);
} catch (error) {
    console.error(`ERROR: ${error.message}`);
    process.exit(2);
}

const exampleKeys = [...example.entries.keys()];
const missing = [];
const emptyRequired = [];
const placeholders = [];
const optionalPlaceholders = [];
const optionalEmpty = [];
const unknown = [];
const duplicates = [];

for (const key of exampleKeys) {
    const targetEntry = target.entries.get(key);
    if (!targetEntry) {
        if (requiredBackendKeys.has(key)) {
            missing.push(key);
        }
        continue;
    }

    if (requiredBackendKeys.has(key) && targetEntry.value.trim() === "") {
        emptyRequired.push(`${key} at line ${targetEntry.line}`);
        continue;
    }

    if (requiredBackendKeys.has(key) && isPlaceholder(targetEntry.value)) {
        placeholders.push(`${key} at line ${targetEntry.line} = ${displayValue(targetEntry.value)}`);
        continue;
    }

    if (optionalButUsefulKeys.has(key) || optionalKeys.has(key)) {
        if (targetEntry.value.trim() === "") {
            optionalEmpty.push(`${key} at line ${targetEntry.line}`);
        } else if (isPlaceholder(targetEntry.value)) {
            optionalPlaceholders.push(`${key} at line ${targetEntry.line} = ${displayValue(targetEntry.value)}`);
        }
    }
}

for (const [key, entry] of target.entries) {
    if (!example.entries.has(key)) {
        unknown.push(`${key} at line ${entry.line}`);
    }
}

for (const duplicate of target.duplicateKeys) {
    duplicates.push(`${duplicate.key} repeated at line ${duplicate.line}`);
}

console.log(`Checked env file: ${envPath}`);
console.log(`Against example: ${examplePath}`);

printGroup("Missing required backend keys", missing);
printGroup("Required keys with empty values", emptyRequired);
printGroup("Required keys still using placeholders", placeholders);
printGroup("Optional/useful keys left empty", optionalEmpty);
printGroup("Optional/useful keys still using placeholders", optionalPlaceholders);
printGroup("Unknown keys not present in env.example.txt", unknown);
printGroup("Duplicate keys", duplicates);

const failures = missing.length + emptyRequired.length + placeholders.length + duplicates.length;
if (failures > 0) {
    console.log(`\nResult: FAIL (${failures} blocking issue${failures === 1 ? "" : "s"})`);
    process.exit(1);
}

console.log("\nResult: PASS");
process.exit(0);
