#!/usr/bin/env node

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import crypto from "node:crypto";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);

const options = parseArgs(process.argv.slice(2));
if (options.help) {
    printUsage();
    process.exit(0);
}

const env = {
    ...readEnvFile(path.join(repoRoot, ".env"), false),
    ...readEnvFile(options.envFile, false),
    ...process.env
};

const baseUrl = normalizeBaseUrl(
    options.baseUrl
    || env.ANTIROT_BACKEND_URL
    || "http://127.0.0.1:8787"
);
const token = options.token || env.ANTIROT_ADMIN_TOKEN || env.ANTIROT_DEVICE_TOKEN || "";

if (!token) {
    fail("Missing auth token. Pass --token or set ANTIROT_ADMIN_TOKEN in the env file.");
}

const results = [];
let generatedAudio;

try {
    await checkHealth();
    const ttsAudio = await checkTts();
    generatedAudio = ttsAudio;
    await checkStt(ttsAudio);
    await checkEmbeddingsViaMemory();
    await checkLlmChat();
} finally {
    printResults();
}

if (results.some((result) => result.status === "FAIL")) {
    process.exit(1);
}

if (results.some((result) => result.status === "SKIP")) {
    process.exit(2);
}

process.exit(0);

async function checkHealth() {
    const started = Date.now();
    const body = await requestJson("/v1/health", { auth: false });
    assert(body.ok === true, "health response missing ok=true");
    pass("health", started, "backend responded");
}

async function checkTts() {
    const started = Date.now();
    try {
        const body = await requestJson("/v1/speech/synthesize", {
            method: "POST",
            body: {
                text: "Antirot integration smoke test. Voice output is online."
            }
        });
        assert(body.ok === true, "TTS response missing ok=true");
        assert(typeof body.audioBase64 === "string" && body.audioBase64.length > 100, "TTS audioBase64 is too small");
        const bytes = Buffer.from(body.audioBase64, "base64");
        assert(bytes.length > 100, "TTS decoded audio is too small");
        const contentType = body.contentType || "application/octet-stream";
        const extension = extensionForContentType(contentType);
        const filePath = path.join(os.tmpdir(), `antirot-tts-smoke-${Date.now()}${extension}`);
        fs.writeFileSync(filePath, bytes);
        pass("tts", started, `${bytes.length} bytes, ${contentType}`);
        return { filePath, contentType };
    } catch (error) {
        failResult("tts", started, error.message);
        return null;
    }
}

async function checkStt(ttsAudio) {
    const started = Date.now();
    const audioFile = options.audioFile || ttsAudio?.filePath;
    const contentType = options.audioContentType || ttsAudio?.contentType || contentTypeForPath(audioFile);
    if (!audioFile) {
        skip("stt", started, "No audio available. Fix TTS or pass --audio-file path/to/voice.m4a.");
        return;
    }
    if (!fs.existsSync(audioFile)) {
        failResult("stt", started, `Audio file not found: ${audioFile}`);
        return;
    }

    try {
        const form = new FormData();
        const bytes = fs.readFileSync(audioFile);
        form.append(
            "file",
            new Blob([bytes], { type: contentType }),
            path.basename(audioFile)
        );
        const body = await requestJson("/v1/speech/transcribe", {
            method: "POST",
            headers: {},
            rawBody: form
        });
        assert(body.ok === true, "STT response missing ok=true");
        assert(typeof body.text === "string" && body.text.trim().length > 0, "STT text is empty");
        pass("stt", started, `transcribed: ${body.text.trim().slice(0, 80)}`);
    } catch (error) {
        failResult("stt", started, error.message);
    }
}

async function checkEmbeddingsViaMemory() {
    const started = Date.now();
    const marker = `antirot embedding smoke ${Date.now()}`;
    const paragraph = [
        `# Integration Smoke Memory`,
        `Marker: ${marker}`,
        "This memory is intentionally long enough to create several chunks.",
        "The backend should index it through Gemini embeddings, with Voyage as fallback if configured.",
        "The content repeats product words like focus, sleep, routine, accountability, and work session."
    ].join("\n");
    const content = Array.from({ length: 45 }, (_, index) => `${paragraph}\nChunk line ${index + 1}.`).join("\n\n");

    try {
        const body = await requestJson("/v1/memory/longterm", {
            method: "PUT",
            body: { content }
        });
        assert(body.ok === true, "memory update response missing ok=true");
        assert(body.key === "longterm", "memory update response key mismatch");
        pass("embeddings", started, "memory update completed; backend attempted semantic indexing; check backend logs for provider fallback warnings");
    } catch (error) {
        failResult("embeddings", started, error.message);
    }
}

async function checkLlmChat() {
    const started = Date.now();
    try {
        const body = await requestJson("/v1/chat", {
            method: "POST",
            body: {
                requestId: crypto.randomUUID(),
                message: "Integration smoke test. Reply in one short sentence confirming the coach LLM path is online."
            }
        });
        assert(body.ok === true, "chat response missing ok=true");
        assert(typeof body.reply === "string" && body.reply.trim().length > 0, "chat reply is empty");
        pass("llm", started, body.reply.trim().slice(0, 120));
    } catch (error) {
        failResult("llm", started, error.message);
    }
}

async function requestJson(pathName, options = {}) {
    const headers = new Headers(options.headers ?? {});
    if (options.auth !== false) {
        headers.set("Authorization", `Bearer ${token}`);
    }
    let body;
    if (options.rawBody) {
        body = options.rawBody;
    } else if (options.body !== undefined) {
        headers.set("Content-Type", "application/json");
        body = JSON.stringify(options.body);
    }

    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), Number(options.timeoutMs ?? 90_000));
    let response;
    let text;
    try {
        response = await fetch(`${baseUrl}${pathName}`, {
            method: options.method ?? "GET",
            headers,
            body,
            signal: controller.signal
        });
        text = await response.text();
    } catch (error) {
        throw new Error(`${options.method ?? "GET"} ${pathName} request failed: ${error.message}`);
    } finally {
        clearTimeout(timeout);
    }

    let json;
    try {
        json = text ? JSON.parse(text) : {};
    } catch {
        throw new Error(`${options.method ?? "GET"} ${pathName} returned non-JSON HTTP ${response.status}: ${text.slice(0, 500)}`);
    }

    if (!response.ok) {
        throw new Error(`${options.method ?? "GET"} ${pathName} failed HTTP ${response.status}: ${JSON.stringify(json).slice(0, 800)}`);
    }
    return json;
}

function parseArgs(args) {
    const parsed = {
        envFile: path.join(repoRoot, ".env"),
        baseUrl: "",
        token: "",
        audioFile: "",
        audioContentType: "",
        help: false
    };

    for (let index = 0; index < args.length; index += 1) {
        const arg = args[index];
        if (arg === "--help" || arg === "-h") {
            parsed.help = true;
        } else if (arg === "--env-file") {
            parsed.envFile = path.resolve(args[++index]);
        } else if (arg === "--base-url") {
            parsed.baseUrl = args[++index];
        } else if (arg === "--token") {
            parsed.token = args[++index];
        } else if (arg === "--audio-file") {
            parsed.audioFile = path.resolve(args[++index]);
        } else if (arg === "--audio-content-type") {
            parsed.audioContentType = args[++index];
        } else {
            fail(`Unknown argument: ${arg}`);
        }
    }

    return parsed;
}

function readEnvFile(filePath, required = true) {
    if (!filePath || !fs.existsSync(filePath)) {
        if (required) {
            fail(`Env file not found: ${filePath}`);
        }
        return {};
    }

    const result = {};
    const regex = /^\s*([A-Za-z0-9_]+)\s*=\s*(?:'([^']*)'|"([^"]*)"|([^#\r\n]*))/gmu;
    const content = fs.readFileSync(filePath, "utf8");
    let match;
    while ((match = regex.exec(content)) !== null) {
        const key = match[1];
        const value = match[2] ?? match[3] ?? match[4].trim();
        result[key] = value;
    }
    return result;
}

function normalizeBaseUrl(value) {
    return value.replace(/\/+$/u, "");
}

function extensionForContentType(contentType) {
    if (contentType.includes("mpeg") || contentType.includes("mp3")) {
        return ".mp3";
    }
    if (contentType.includes("wav")) {
        return ".wav";
    }
    if (contentType.includes("mp4") || contentType.includes("m4a")) {
        return ".m4a";
    }
    return ".bin";
}

function contentTypeForPath(filePath) {
    if (!filePath) {
        return "application/octet-stream";
    }
    const ext = path.extname(filePath).toLowerCase();
    if (ext === ".mp3") {
        return "audio/mpeg";
    }
    if (ext === ".wav") {
        return "audio/wav";
    }
    if (ext === ".m4a" || ext === ".mp4") {
        return "audio/mp4";
    }
    return "application/octet-stream";
}

function pass(name, started, detail) {
    results.push({ name, status: "PASS", ms: Date.now() - started, detail });
}

function skip(name, started, detail) {
    results.push({ name, status: "SKIP", ms: Date.now() - started, detail });
}

function failResult(name, started, detail) {
    results.push({ name, status: "FAIL", ms: Date.now() - started, detail });
}

function assert(condition, message) {
    if (!condition) {
        throw new Error(message);
    }
}

function fail(message) {
    console.error(`ERROR: ${message}`);
    process.exit(2);
}

function printResults() {
    console.log(`Backend integration smoke test: ${baseUrl}`);
    for (const result of results) {
        console.log(`[${result.status}] ${result.name} ${result.ms}ms - ${result.detail}`);
    }
    const passCount = results.filter((result) => result.status === "PASS").length;
    const failCount = results.filter((result) => result.status === "FAIL").length;
    const skipCount = results.filter((result) => result.status === "SKIP").length;
    console.log(`Result: ${passCount} passed, ${failCount} failed, ${skipCount} skipped`);
    if (generatedAudio?.filePath) {
        console.log(`Generated TTS audio: ${generatedAudio.filePath}`);
    }
}

function printUsage() {
    console.log(`Usage:
  node scripts/test-backend-integrations.mjs --env-file /etc/antirot/backend.env --base-url https://api.example.com

Options:
  --env-file PATH            Env file with ANTIROT_ADMIN_TOKEN and provider keys. Defaults to .env.
  --base-url URL             Backend URL. Defaults to ANTIROT_BACKEND_URL or http://127.0.0.1:8787.
  --token TOKEN              Auth token. Defaults to ANTIROT_ADMIN_TOKEN, then ANTIROT_DEVICE_TOKEN.
  --audio-file PATH          Optional real speech file for STT if TTS is unavailable.
  --audio-content-type TYPE  MIME type for --audio-file. Guessed from extension if omitted.
  --help                     Show this help.

Checks:
  health       GET  /v1/health
  tts          POST /v1/speech/synthesize
  stt          POST /v1/speech/transcribe
  embeddings   PUT  /v1/memory/longterm, which triggers semantic indexing; check backend logs for fallback warnings
  llm          POST /v1/chat
`);
}
