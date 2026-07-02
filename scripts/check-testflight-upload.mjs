#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import process from "node:process";

const defaultWorkflow = "deploy-ios-testflight.yml";
const defaultBranch = "main";
const deployJobName = "Deploy to TestFlight";
const uploadStepName = "Upload to TestFlight";
const errorContextChars = 100;
const defaultPollIntervalSeconds = 30;
const defaultTimeoutSeconds = 1800;

export function parseArgs(argv) {
    const options = {
        branch: defaultBranch,
        json: false,
        pollIntervalSeconds: defaultPollIntervalSeconds,
        repo: null,
        runId: null,
        timeoutSeconds: defaultTimeoutSeconds,
        wait: true,
        workflow: defaultWorkflow
    };

    for (let index = 0; index < argv.length; index += 1) {
        const arg = argv[index];
        if (arg === "--json") {
            options.json = true;
            continue;
        }
        if (arg === "--no-wait") {
            options.wait = false;
            continue;
        }
        if (arg === "--poll-interval") {
            options.pollIntervalSeconds = parsePositiveInteger(readValue(argv, index, arg), arg);
            index += 1;
            continue;
        }
        if (arg === "--timeout") {
            options.timeoutSeconds = parsePositiveInteger(readValue(argv, index, arg), arg);
            index += 1;
            continue;
        }
        if (arg === "--run-id") {
            options.runId = readValue(argv, index, arg);
            index += 1;
            continue;
        }
        if (arg === "--workflow") {
            options.workflow = readValue(argv, index, arg);
            index += 1;
            continue;
        }
        if (arg === "--branch") {
            options.branch = readValue(argv, index, arg);
            index += 1;
            continue;
        }
        if (arg === "--repo") {
            options.repo = readValue(argv, index, arg);
            index += 1;
            continue;
        }
        if (arg === "--help" || arg === "-h") {
            printUsage();
            process.exit(0);
        }
        throw new Error(`Unknown argument: ${arg}`);
    }

    return options;
}

export function evaluateTestFlightUpload(run, errorLog = "") {
    const runId = run.databaseId ?? run.id ?? null;
    const base = {
        ok: false,
        runId,
        runUrl: run.url ?? null,
        status: "error"
    };

    if (run.status !== "completed") {
        return {
            ...base,
            status: "running",
            message: "running"
        };
    }

    const job = (run.jobs ?? []).find((candidate) => candidate.name === deployJobName);
    if (!job) {
        return {
            ...base,
            message: `${deployJobName} job was not found.`
        };
    }

    const uploadStep = (job.steps ?? []).find((step) => step.name === uploadStepName);
    if (!uploadStep) {
        return {
            ...base,
            message: `${uploadStepName} step was not found.`
        };
    }

    if (uploadStep.conclusion !== "success") {
        return {
            ...withErrorLog(base, errorLog),
            message: `${uploadStepName} step concluded with ${uploadStep.conclusion ?? "unknown"}.`
        };
    }

    if (job.conclusion !== "success") {
        return {
            ...withErrorLog(base, errorLog),
            message: `${deployJobName} job concluded with ${job.conclusion ?? "unknown"} after ${uploadStepName} succeeded.`
        };
    }

    if (run.conclusion !== "success") {
        return {
            ...withErrorLog(base, errorLog),
            message: `Workflow run concluded with ${run.conclusion ?? "unknown"} after ${uploadStepName} succeeded.`
        };
    }

    return {
        ...base,
        ok: true,
        status: "succeeded",
        message: `${uploadStepName} succeeded.`
    };
}

export function excerptErrorLog(logText) {
    if (!logText) {
        return "";
    }

    const firstCaret = logText.indexOf("^");
    const lastCaret = logText.lastIndexOf("^");
    if (firstCaret === -1) {
        return logText.slice(-Math.min(logText.length, errorContextChars * 2));
    }

    const firstStart = Math.max(0, firstCaret - errorContextChars);
    const firstEnd = Math.min(logText.length, firstCaret + errorContextChars + 1);
    const lastStart = Math.max(0, lastCaret - errorContextChars);
    const lastEnd = Math.min(logText.length, lastCaret + errorContextChars + 1);

    if (firstEnd >= lastStart) {
        return logText.slice(firstStart, lastEnd);
    }

    return `${logText.slice(firstStart, firstEnd)}\n...\n${logText.slice(lastStart, lastEnd)}`;
}

function withErrorLog(result, errorLog) {
    const error = excerptErrorLog(errorLog);
    if (!error) {
        return result;
    }
    return {
        ...result,
        error
    };
}

function readValue(argv, index, name) {
    const value = argv[index + 1];
    if (!value || value.startsWith("--")) {
        throw new Error(`${name} requires a value.`);
    }
    return value;
}

function parsePositiveInteger(value, name) {
    const parsed = Number.parseInt(value, 10);
    if (!Number.isInteger(parsed) || parsed <= 0 || String(parsed) !== value) {
        throw new Error(`${name} must be a positive integer.`);
    }
    return parsed;
}

function ghJson(args) {
    try {
        return JSON.parse(execFileSync("gh", args, { encoding: "utf8" }));
    } catch (error) {
        const stderr = error.stderr?.toString().trim();
        const detail = stderr ? ` ${stderr}` : "";
        throw new Error(`gh command failed: gh ${args.join(" ")}.${detail}`);
    }
}

function ghText(args) {
    try {
        return execFileSync("gh", args, { encoding: "utf8" });
    } catch (error) {
        return error.stdout?.toString() ?? error.stderr?.toString() ?? error.message;
    }
}

function ghRunArgs(options) {
    return options.repo ? ["--repo", options.repo] : [];
}

function latestRunId(options) {
    const runs = ghJson([
        "run",
        "list",
        "--workflow",
        options.workflow,
        "--branch",
        options.branch,
        "--limit",
        "1",
        "--json",
        "databaseId,status,conclusion,displayTitle,headBranch,url,createdAt,updatedAt",
        ...ghRunArgs(options)
    ]);

    const latestRun = runs[0];
    if (!latestRun?.databaseId) {
        throw new Error(`No workflow runs found for ${options.workflow} on ${options.branch}.`);
    }

    return String(latestRun.databaseId);
}

function fetchRun(options) {
    const runId = options.runId ?? latestRunId(options);
    return ghJson([
        "run",
        "view",
        runId,
        "--json",
        "databaseId,status,conclusion,displayTitle,headBranch,url,jobs",
        ...ghRunArgs(options)
    ]);
}

function sleep(seconds) {
    execFileSync("sleep", [String(seconds)], { stdio: "ignore" });
}

function fetchFailedLog(runId, options) {
    return ghText([
        "run",
        "view",
        runId,
        "--log-failed",
        ...ghRunArgs(options)
    ]);
}

function printResult(result, options) {
    if (options.json) {
        console.log(JSON.stringify(result, null, 4));
        return;
    }

    console.log(result.message);
    if (result.error) {
        console.log("Error excerpt:");
        console.log(result.error);
    }
    if (result.runId) {
        console.log(`Run ID: ${result.runId}`);
    }
    if (result.runUrl) {
        console.log(`Run URL: ${result.runUrl}`);
    }
}

function printUsage() {
    console.log(`Usage: node scripts/check-testflight-upload.mjs [options]

Checks whether the iOS TestFlight workflow's "Upload to TestFlight" step succeeded.

Options:
  --run-id <id>       Check a specific GitHub Actions run.
  --workflow <file>   Workflow file to inspect. Default: ${defaultWorkflow}
  --branch <branch>   Branch used when finding the latest run. Default: ${defaultBranch}
  --repo <owner/repo> GitHub repo for gh commands. Defaults to current repo.
  --no-wait           Return immediately when the workflow is still running.
  --poll-interval <s> Seconds between polls while waiting. Default: ${defaultPollIntervalSeconds}
  --timeout <s>       Maximum seconds to wait. Default: ${defaultTimeoutSeconds}
  --json              Print machine-readable JSON.
  --help              Show this help.
`);
}

function checkOnce(options) {
    const run = fetchRun(options);
    const runId = run.databaseId ?? run.id ?? options.runId;
    const shouldFetchErrorLog = run.status === "completed" && run.conclusion !== "success" && runId;
    const errorLog = shouldFetchErrorLog ? fetchFailedLog(String(runId), options) : "";
    return evaluateTestFlightUpload(run, errorLog);
}

function checkUntilTerminal(options) {
    const startedAt = Date.now();
    const checkedOptions = options.runId ? options : { ...options, runId: latestRunId(options) };

    while (true) {
        const result = checkOnce(checkedOptions);
        if (!options.wait || result.status !== "running") {
            return result;
        }

        const elapsedSeconds = Math.floor((Date.now() - startedAt) / 1000);
        if (elapsedSeconds + options.pollIntervalSeconds > options.timeoutSeconds) {
            return {
                ...result,
                status: "timeout",
                message: `Timed out after ${options.timeoutSeconds}s waiting for TestFlight upload action to finish.`
            };
        }

        sleep(options.pollIntervalSeconds);
    }
}

function main() {
    const options = parseArgs(process.argv.slice(2));
    const result = checkUntilTerminal(options);

    printResult(result, options);
    process.exit(result.ok ? 0 : result.status === "running" ? 2 : 1);
}

if (import.meta.url === `file://${process.argv[1]}`) {
    try {
        main();
    } catch (error) {
        console.error(error.message);
        process.exit(1);
    }
}
