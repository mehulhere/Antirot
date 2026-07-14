#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { execFileSync, spawnSync } from "node:child_process";
import crypto from "node:crypto";

import {
    api,
    authHeaders,
    readDotEnv,
    repoRoot,
    resetFixture,
    snapshot
} from "./backend-userflow-test-lib.mjs";

const defaultBaseUrl = "https://api.antirot.org";
const defaultRunsDir = path.join(repoRoot, ".antirot", "scenario-runs");
const scenarioDir = path.join(repoRoot, "scenarios", "antirot");
const memoryKeys = [
    "personality",
    "user_profile",
    "durable",
    "longterm",
    "shortterm",
    "behavior",
    "tasks",
    "routine",
    "sleep",
    "achievements",
    "miscellaneous_todo"
];

function parseArgs(argv) {
    const args = {
        scenarioPath: "",
        baseUrl: process.env.ANTIROT_BACKEND_URL || defaultBaseUrl,
        runsDir: defaultRunsDir,
        continueRun: "",
        reportRun: "",
        message: "",
        list: false,
        listRuns: false,
        fresh: true,
        copyReport: true,
        saveReport: true
    };

    for (let index = 0; index < argv.length; index += 1) {
        const arg = argv[index];
        if (arg === "--base-url") {
            args.baseUrl = requiredValue(argv, index += 1, arg).replace(/\/+$/u, "");
        } else if (arg === "--runs-dir") {
            args.runsDir = path.resolve(requiredValue(argv, index += 1, arg));
        } else if (arg === "--continue") {
            args.continueRun = requiredValue(argv, index += 1, arg);
            args.fresh = false;
        } else if (arg === "--report") {
            args.reportRun = requiredValue(argv, index += 1, arg);
        } else if (arg === "--message") {
            args.message = requiredValue(argv, index += 1, arg);
        } else if (arg === "--list") {
            args.list = true;
        } else if (arg === "--runs") {
            args.listRuns = true;
        } else if (arg === "--no-copy-report") {
            args.copyReport = false;
        } else if (arg === "--no-save-report") {
            args.saveReport = false;
        } else if (arg === "--fresh") {
            args.fresh = true;
        } else if (arg.startsWith("-")) {
            throw new Error(`Unknown option: ${arg}`);
        } else if (!args.scenarioPath) {
            args.scenarioPath = path.resolve(arg);
        } else {
            throw new Error(`Unexpected argument: ${arg}`);
        }
    }
    return args;
}

function requiredValue(argv, index, flag) {
    const value = argv[index];
    if (!value || value.startsWith("--")) {
        throw new Error(`${flag} requires a value`);
    }
    return value;
}

function listScenarios() {
    if (!fs.existsSync(scenarioDir)) {
        console.log("No scenarios directory found.");
        return;
    }
    for (const file of fs.readdirSync(scenarioDir).filter((entry) => entry.endsWith(".md")).sort()) {
        const fullPath = path.join(scenarioDir, file);
        const scenario = readScenario(fullPath);
        console.log(`${file} - ${scenario.name || "Unnamed scenario"}`);
    }
}

function listRuns(runsDir) {
    if (!fs.existsSync(runsDir)) {
        console.log("No scenario runs found.");
        return;
    }
    const files = fs.readdirSync(runsDir)
        .filter((entry) => entry.endsWith(".json"))
        .sort()
        .reverse();
    if (!files.length) {
        console.log("No scenario runs found.");
        return;
    }
    for (const file of files) {
        const run = JSON.parse(fs.readFileSync(path.join(runsDir, file), "utf8"));
        console.log(`${run.id} - ${run.name} - ${run.updatedAt}`);
    }
}

function readScenario(filePath) {
    const markdown = fs.readFileSync(filePath, "utf8");
    const match = markdown.match(/```json\s*([\s\S]*?)```/u);
    if (!match) {
        throw new Error(`Scenario ${filePath} must contain a json code fence`);
    }
    const parsed = JSON.parse(match[1]);
    if (!Array.isArray(parsed.messages) || parsed.messages.length === 0) {
        throw new Error(`Scenario ${filePath} must define a non-empty messages array`);
    }
    return {
        name: parsed.name || path.basename(filePath, ".md"),
        description: parsed.description || "",
        messages: parsed.messages.map(String),
        checks: Array.isArray(parsed.checks) ? parsed.checks : []
    };
}

function slug(value) {
    return value
        .toLowerCase()
        .replace(/[^a-z0-9]+/gu, "-")
        .replace(/^-|-$/gu, "")
        .slice(0, 60) || "scenario";
}

function nowIso() {
    return new Date().toISOString();
}

function readVpsAdminToken() {
    try {
        const token = execFileSync("ssh", [
            "antirot",
            "set -a; . /etc/antirot/backend.env; set +a; printf %s \"$ANTIROT_ADMIN_TOKEN\""
        ], {
            cwd: repoRoot,
            encoding: "utf8",
            stdio: ["ignore", "pipe", "ignore"]
        }).trim();
        return token;
    } catch {
        return "";
    }
}

function resolveAdminToken(baseUrl) {
    const dotEnv = readDotEnv();
    if (/^https:\/\/api\.antirot\.org\/?$/u.test(baseUrl)) {
        const vpsToken = readVpsAdminToken();
        if (vpsToken) {
            return vpsToken;
        }
    }
    if (process.env.ANTIROT_ADMIN_TOKEN) {
        return process.env.ANTIROT_ADMIN_TOKEN.trim();
    }
    if (dotEnv.ANTIROT_ADMIN_TOKEN) {
        return dotEnv.ANTIROT_ADMIN_TOKEN.trim();
    }
    const vpsToken = readVpsAdminToken();
    if (vpsToken) {
        return vpsToken;
    }
    throw new Error("Missing ANTIROT_ADMIN_TOKEN and could not read it over ssh antirot.");
}

function runPaths(runsDir, runId) {
    return {
        json: path.join(runsDir, `${runId}.json`),
        markdown: path.join(runsDir, `${runId}.md`),
        report: path.join(runsDir, `${runId}.report.md`)
    };
}

function loadRun(runsDir, runId) {
    const paths = runPaths(runsDir, runId);
    if (!fs.existsSync(paths.json)) {
        throw new Error(`Run not found: ${runId}. Expected ${paths.json}`);
    }
    return JSON.parse(fs.readFileSync(paths.json, "utf8"));
}

async function loadMemory(baseUrl, token) {
    const rows = [];
    for (const key of memoryKeys) {
        try {
            const body = await api(baseUrl, `/v1/memory/${encodeURIComponent(key)}`, {
                headers: authHeaders(token)
            });
            rows.push({ key, content: body.content || "" });
        } catch (error) {
            rows.push({ key, error: error instanceof Error ? error.message : String(error) });
        }
    }
    return rows;
}

function summarizeMemoryDiff(beforeRows, afterRows) {
    const beforeByKey = new Map(beforeRows.map((row) => [row.key, row]));
    return afterRows
        .map((after) => {
            const before = beforeByKey.get(after.key);
            if (after.error || before?.error) {
                return { key: after.key, summary: `load issue: ${after.error || before?.error}` };
            }
            if (!before) {
                return { key: after.key, summary: "newly observed" };
            }
            if (before.content === after.content) {
                return null;
            }
            return {
                key: after.key,
                summary: `changed (${before.content.length} chars -> ${after.content.length} chars)`
            };
        })
        .filter(Boolean);
}

async function loadDiagnostics(baseUrl, userId, adminToken) {
    try {
        const body = await api(
            baseUrl,
            `/v1/admin/context?userId=${encodeURIComponent(userId)}&provider=gemini&model=gemini-3.5-flash`,
            { headers: authHeaders(adminToken) }
        );
        return body;
    } catch (error) {
        return { ok: false, error: error instanceof Error ? error.message : String(error) };
    }
}

function compactSnapshot(value) {
    return {
        runtimeState: value?.runtimeState || null,
        alarmCounts: value?.alarmCounts || []
    };
}

async function sendChat(baseUrl, token, message) {
    const startedAt = Date.now();
    const body = await api(baseUrl, "/v1/chat", {
        method: "POST",
        headers: authHeaders(token),
        body: JSON.stringify({ requestId: crypto.randomUUID(), message })
    });
    return {
        reply: body.reply,
        latencyMs: Date.now() - startedAt
    };
}

function applyChecks(checks, turns) {
    return checks.map((check) => {
        const turn = turns[Number(check.turn) - 1];
        if (!turn) {
            return { ...check, ok: false, detail: "turn not found" };
        }
        const target = check.target === "state" ? JSON.stringify(turn.snapshot) : turn.reply;
        if (check.notContains) {
            const ok = !target.toLowerCase().includes(String(check.notContains).toLowerCase());
            return { ...check, ok, detail: ok ? "absent" : "unexpected text present" };
        }
        if (check.contains) {
            const ok = target.toLowerCase().includes(String(check.contains).toLowerCase());
            return { ...check, ok, detail: ok ? "present" : "missing text" };
        }
        if (check.state) {
            const ok = turn.snapshot?.runtimeState?.state === check.state;
            return { ...check, ok, detail: turn.snapshot?.runtimeState?.state || "none" };
        }
        return { ...check, ok: false, detail: "unknown check" };
    });
}

function writeRunFiles(run) {
    fs.mkdirSync(run.runsDir, { recursive: true });
    const paths = runPaths(run.runsDir, run.id);
    fs.writeFileSync(paths.json, `${JSON.stringify(run, null, 2)}\n`);
    fs.writeFileSync(paths.markdown, `${renderRunMarkdown(run)}\n`);
    return paths;
}

function renderRunMarkdown(run) {
    const checkLines = run.checkResults?.length
        ? run.checkResults.map((check) => `- ${check.ok ? "PASS" : "FAIL"} turn ${check.turn}: ${check.description || JSON.stringify(check)} (${check.detail})`).join("\n")
        : "No checks configured.";
    const commands = [
        "```bash",
        `npm run scenario -- --continue ${run.id} --message "Your next message here"`,
        `npm run scenario -- --report ${run.id}`,
        "```"
    ].join("\n");
    const turnLines = run.turns.map((turn, index) => [
        `## Turn ${index + 1}`,
        "",
        `At: ${turn.at}`,
        `Latency: ${turn.latencyMs}ms`,
        "",
        "### User",
        turn.user,
        "",
        "### Assistant",
        turn.reply,
        "",
        "### State",
        "```json",
        JSON.stringify(compactSnapshot(turn.snapshot), null, 2),
        "```"
    ].join("\n")).join("\n\n");

    return [
        `# Antirot Scenario Run: ${run.name}`,
        "",
        `Run ID: ${run.id}`,
        `Backend: ${run.baseUrl}`,
        `Created: ${run.createdAt}`,
        `Updated: ${run.updatedAt}`,
        `User ID: ${run.userId}`,
        `Device ID: ${run.deviceId}`,
        "",
        "## Continue Or Report",
        commands,
        "",
        "## Checks",
        checkLines,
        "",
        turnLines
    ].join("\n");
}

function renderReportMarkdown(run) {
    const memoryDiff = summarizeMemoryDiff(run.initialMemory || [], run.latestMemory || []);
    const memoryLines = memoryDiff.length
        ? memoryDiff.map((row) => `- ${row.key}.md: ${row.summary}`).join("\n")
        : "No observed memory file changes.";
    const eventLines = run.turns.map((turn, index) => [
        `### ${index + 1}. chat.turn @ ${turn.at}`,
        `User: ${turn.user}`,
        `Latency: ${turn.latencyMs}ms`,
        `State: ${turn.snapshot?.runtimeState?.state || "unknown"}`,
        "",
        "Assistant:",
        turn.reply
    ].join("\n")).join("\n\n");

    return [
        "# Antirot Terminal Scenario Report",
        "",
        `Run ID: ${run.id}`,
        `Scenario: ${run.name}`,
        `Created: ${nowIso()}`,
        `Backend: ${run.baseUrl}`,
        `User ID: ${run.userId}`,
        `Device ID: ${run.deviceId}`,
        "",
        "## Diagnostics",
        "```json",
        JSON.stringify(run.diagnostics || {}, null, 2),
        "```",
        "",
        "## Checks",
        run.checkResults?.length
            ? run.checkResults.map((check) => `- ${check.ok ? "PASS" : "FAIL"} turn ${check.turn}: ${check.description || JSON.stringify(check)} (${check.detail})`).join("\n")
            : "No checks configured.",
        "",
        "## Memory Changes",
        memoryLines,
        "",
        "## Timeline",
        eventLines
    ].join("\n");
}

function copyToClipboard(text) {
    const commands = [
        ["wl-copy", []],
        ["xclip", ["-selection", "clipboard"]],
        ["xsel", ["--clipboard", "--input"]],
        ["pbcopy", []]
    ];
    for (const [command, args] of commands) {
        const result = spawnSync(command, args, {
            input: text,
            encoding: "utf8",
            stdio: ["pipe", "ignore", "ignore"]
        });
        if (result.status === 0) {
            return command;
        }
    }
    return "";
}

async function saveReportToBackend(run, reportMarkdown) {
    const body = await api(run.baseUrl, "/v1/reports", {
        method: "POST",
        headers: authHeaders(run.deviceToken),
        body: JSON.stringify({
            deviceId: run.deviceId,
            title: `Terminal scenario report: ${run.name}`,
            windowStart: run.createdAt,
            windowEnd: nowIso(),
            reportMarkdown,
            events: run.turns.map((turn, index) => ({
                at: turn.at,
                kind: "terminal.chat",
                summary: `Turn ${index + 1}: ${turn.user}`,
                detail: turn.reply
            }))
        })
    });
    return body.reportId;
}

async function createFreshRun(args, scenario, adminToken) {
    const fixture = await resetFixture(args.baseUrl, slug(scenario.name));
    return {
        id: `${new Date().toISOString().replace(/[:.]/gu, "-")}-${slug(scenario.name)}`,
        name: scenario.name,
        description: scenario.description,
        baseUrl: args.baseUrl,
        runsDir: args.runsDir,
        createdAt: nowIso(),
        updatedAt: nowIso(),
        userId: fixture.userId,
        deviceId: fixture.deviceId,
        deviceToken: fixture.deviceToken,
        adminTokenSource: adminToken ? "resolved" : "missing",
        turns: [],
        checks: scenario.checks,
        checkResults: [],
        initialMemory: await loadMemory(args.baseUrl, fixture.deviceToken),
        latestMemory: []
    };
}

async function runMessages(args, run, messages, adminToken) {
    for (const message of messages) {
        const sent = await sendChat(run.baseUrl, run.deviceToken, message);
        const state = await snapshot(run.baseUrl, run.userId, run.deviceId);
        run.turns.push({
            at: nowIso(),
            user: message,
            reply: sent.reply,
            latencyMs: sent.latencyMs,
            snapshot: compactSnapshot(state)
        });
        console.log(`\nUSER: ${message}`);
        console.log(`ASSISTANT (${sent.latencyMs}ms): ${sent.reply}`);
        console.log(`STATE: ${state.runtimeState?.state || "unknown"}`);
    }
    run.updatedAt = nowIso();
    run.latestMemory = await loadMemory(run.baseUrl, run.deviceToken);
    run.diagnostics = await loadDiagnostics(run.baseUrl, run.userId, adminToken);
    run.checkResults = applyChecks(run.checks || [], run.turns);
    const paths = writeRunFiles(run);
    console.log(`\nRun saved: ${paths.markdown}`);
    if (run.checkResults.length) {
        for (const check of run.checkResults) {
            console.log(`${check.ok ? "PASS" : "FAIL"} turn ${check.turn}: ${check.description || check.detail}`);
        }
    }
    return run;
}

async function main() {
    const args = parseArgs(process.argv.slice(2));
    if (args.list) {
        listScenarios();
        return;
    }
    if (args.listRuns) {
        listRuns(args.runsDir);
        return;
    }

    const adminToken = resolveAdminToken(args.baseUrl);
    process.env.ANTIROT_ADMIN_TOKEN = adminToken;

    if (args.reportRun) {
        const run = loadRun(args.runsDir, args.reportRun);
        run.latestMemory = await loadMemory(run.baseUrl, run.deviceToken);
        run.diagnostics = await loadDiagnostics(run.baseUrl, run.userId, adminToken);
        run.checkResults = applyChecks(run.checks || [], run.turns);
        const reportMarkdown = renderReportMarkdown(run);
        const paths = runPaths(args.runsDir, run.id);
        fs.writeFileSync(paths.report, `${reportMarkdown}\n`);
        if (args.saveReport) {
            const reportId = await saveReportToBackend(run, reportMarkdown);
            console.log(`Backend report saved: ${reportId}`);
        }
        if (args.copyReport) {
            const command = copyToClipboard(reportMarkdown);
            console.log(command ? `Report copied with ${command}.` : "No clipboard command found; report saved to disk.");
        }
        console.log(`Report written: ${paths.report}`);
        return;
    }

    let run;
    let messages;
    if (args.continueRun) {
        run = loadRun(args.runsDir, args.continueRun);
        messages = args.message ? [args.message] : [];
        if (!messages.length) {
            throw new Error("--continue requires --message for the next user turn.");
        }
    } else {
        if (!args.scenarioPath) {
            throw new Error("Provide a scenario markdown file or use --list.");
        }
        const scenario = readScenario(args.scenarioPath);
        run = await createFreshRun(args, scenario, adminToken);
        messages = scenario.messages;
    }
    await runMessages(args, run, messages, adminToken);
}

main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
});
