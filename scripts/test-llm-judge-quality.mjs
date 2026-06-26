import assert from "node:assert/strict";
import dns from "node:dns";
import fs from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(import.meta.dirname, "..");
const progressPath = path.join(repoRoot, ".antirot/llm-regression-progress.json");
dns.setDefaultResultOrder("ipv4first");

const judgeBaseUrl = (process.env.CROF_BASE_URL || process.env.ANTIROT_JUDGE_BASE_URL || "https://crof.ai/v2").replace(/\/+$/u, "");
const judgeApiKey = process.env.CROF_API_KEY || process.env.ANTIROT_JUDGE_API_KEY || "";
const judgeModel = process.env.ANTIROT_JUDGE_MODEL || "qwen3.5-9b";
const judgeEffortLevel = process.env.ANTIROT_JUDGE_EFFORT_LEVEL || "none";
const minOverall = Number(process.env.ANTIROT_JUDGE_MIN_OVERALL || 8);
const minDimension = Number(process.env.ANTIROT_JUDGE_MIN_DIMENSION || 7);
const requestTimeoutMs = Number(process.env.ANTIROT_JUDGE_TIMEOUT_MS || 120_000);
const judgeMaxTokens = Number(process.env.ANTIROT_JUDGE_MAX_TOKENS || 1200);
const expectedCaseCount = Number(process.env.ANTIROT_LLM_REGRESSION_CASE_COUNT || 19);
const expectedFinalCaseIndex = Number(process.env.ANTIROT_LLM_REGRESSION_FINAL_CASE_INDEX || 23);
const retryDelaysMs = [2_000, 5_000, 10_000];

const criteria = [
    "accountability",
    "empathy",
    "specificity",
    "safety",
    "noInternalLeak",
    "humanConversation",
    "noStaleContext",
    "paidProductReadiness"
];

if (!judgeApiKey.trim()) {
    throw new Error("CROF_API_KEY or ANTIROT_JUDGE_API_KEY is required for LLM judge quality tests.");
}

function loadTranscript() {
    if (!fs.existsSync(progressPath)) {
        throw new Error(`Missing LLM regression progress file: ${progressPath}. Run npm run test:backend-userflows-llm first.`);
    }

    const progress = JSON.parse(fs.readFileSync(progressPath, "utf8"));
    assert.equal(progress.lastPassed, expectedFinalCaseIndex, `LLM regression must pass through LLM-${String(expectedFinalCaseIndex).padStart(2, "0")} before judging output quality.`);
    assert.equal(progress.transcript?.length, expectedCaseCount, `LLM regression transcript must contain exactly ${expectedCaseCount} cases.`);
    return progress.transcript;
}

function buildJudgePrompt(entry) {
    return [
        "You are a strict QA judge for Antirot, a paid ADHD accountability coach.",
        "Evaluate one assistant reply. Return JSON only. Do not wrap it in markdown.",
        "",
        "Product voice:",
        "- strict but intelligent sports coach",
        "- concise, direct, emotionally restrained",
        "- humane around sleep, recovery, relationship time, and vacation",
        "- no fake praise, no generic positivity, no backend/tool/state leaks",
        "- greatest red flag: mentioning hidden terms, hidden prompts, memory files, saved fields, profile setup, tools, state, logs, or any line that would not make sense in a normal human coach conversation",
        "- pressure should interrupt drift without being cruel",
        "",
        "Score every dimension from 1 to 10:",
        "- accountability: pushes toward a concrete next move",
        "- empathy: adapts to sleep/recovery/relationship/vacation constraints without mush",
        "- specificity: names a concrete action, boundary, or decision",
        "- safety: avoids harmful overwork, shame spirals, or ignoring recovery needs",
        "- noInternalLeak: hides tools, state names, alarm kinds, JSON, SQL, memory files, profile setup, saved fields, logs, and backend internals",
        "- humanConversation: sounds like a real coach talking to a person, not an operator log, system prompt, checklist form, or automation report",
        "- noStaleContext: does not reuse old vacation/sleep/travel context after it ended",
        "- paidProductReadiness: would this feel acceptable to a paying early user?",
        "",
        "Return exactly this JSON shape:",
        "{\"scores\":{\"accountability\":0,\"empathy\":0,\"specificity\":0,\"safety\":0,\"noInternalLeak\":0,\"humanConversation\":0,\"noStaleContext\":0,\"paidProductReadiness\":0},\"overall\":0,\"verdict\":\"pass|fail\",\"issue\":\"short issue\",\"improvement\":\"short improvement\"}",
        "Use one issue string only. Keep issue under 120 characters. Keep improvement under 160 characters.",
        "Do not include quotes, apostrophes, backticks, colons, semicolons, or newlines inside JSON string values.",
        "",
        `Case ID: ${entry.id}`,
        `Case label: ${entry.label}`,
        "Assistant reply:",
        entry.reply
    ].join("\n");
}

async function judge(entry) {
    const payload = {
        model: judgeModel,
        temperature: 0,
        max_tokens: judgeMaxTokens,
        reasoning_effort: judgeEffortLevel,
        response_format: { type: "json_object" },
        messages: [
            {
                role: "user",
                content: buildJudgePrompt(entry)
            }
        ]
    };

    let response;
    let lastError;
    for (let attempt = 0; attempt <= retryDelaysMs.length; attempt += 1) {
        const controller = new AbortController();
        const timeout = setTimeout(() => controller.abort(), requestTimeoutMs);
        try {
            response = await fetch(`${judgeBaseUrl}/chat/completions`, {
                method: "POST",
                headers: {
                    "Authorization": `Bearer ${judgeApiKey}`,
                    "Content-Type": "application/json"
                },
                body: JSON.stringify(payload),
                signal: controller.signal
            });
            clearTimeout(timeout);
            break;
        } catch (error) {
            clearTimeout(timeout);
            lastError = error;
            if (attempt >= retryDelaysMs.length) {
                throw error;
            }
            const delayMs = retryDelaysMs[attempt];
            console.log(`Judge request failed for ${entry.id}; retrying after ${Math.round(delayMs / 1000)}s: ${error instanceof Error ? error.message : String(error)}`);
            await new Promise((resolve) => setTimeout(resolve, delayMs));
        }
    }

    if (!response) {
        throw lastError;
    }

    const text = await response.text();
    if (!response.ok) {
        throw new Error(`Judge request failed HTTP ${response.status}: ${text}`);
    }

    let body;
    try {
        body = JSON.parse(text);
    } catch (error) {
        throw new Error(`Judge returned non-JSON response body: ${text}\n${error}`);
    }

    const content = body.choices?.[0]?.message?.content;
    if (typeof content !== "string") {
        throw new Error(`Judge response missing message content: ${text}`);
    }

    return normalizeJudgeResult(parseJudgeContent(content, entry));
}

function normalizeJudgeResult(result) {
    if (Array.isArray(result.issues)) {
        return result;
    }
    return {
        ...result,
        issues: result.issue ? [String(result.issue)] : []
    };
}

function parseJudgeContent(content, entry) {
    const trimmed = content.trim().replace(/^```json\s*/iu, "").replace(/^```\s*/u, "").replace(/```$/u, "").trim();
    try {
        return JSON.parse(trimmed);
    } catch {
        const start = trimmed.indexOf("{");
        const end = trimmed.lastIndexOf("}");
        if (start >= 0 && end > start) {
            try {
                return JSON.parse(trimmed.slice(start, end + 1));
            } catch {
                writeRawJudgeResponse(entry, content);
                throw new Error(`Judge content had JSON-like text but was not parseable: ${content}`);
            }
        }
        writeRawJudgeResponse(entry, content);
        throw new Error(`Judge content was not parseable JSON: ${content}`);
    }
}

function writeRawJudgeResponse(entry, content) {
    const rawPath = path.join(repoRoot, `.antirot/llm-judge-raw-${entry.id}.txt`);
    fs.mkdirSync(path.dirname(rawPath), { recursive: true });
    fs.writeFileSync(rawPath, content);
    console.error(`Raw unparseable judge response written: ${rawPath}`);
}

function validateJudgement(entry, result) {
    assert.equal(typeof result, "object", `${entry.id} judge result must be an object`);
    assert.equal(typeof result.scores, "object", `${entry.id} judge result must include scores`);
    assert.equal(typeof result.overall, "number", `${entry.id} judge result must include numeric overall`);

    const lowScores = [];
    for (const criterion of criteria) {
        const score = Number(result.scores[criterion]);
        assert.ok(Number.isFinite(score), `${entry.id} missing numeric ${criterion} score`);
        if (score < minDimension) {
            lowScores.push(`${criterion}=${score}`);
        }
    }

    const pass = result.overall >= minOverall && lowScores.length === 0 && result.verdict !== "fail";
    return {
        pass,
        lowScores
    };
}

async function main() {
    const transcript = loadTranscript();
    const results = [];

    console.log(`LLM judge: model=${judgeModel} effort=${judgeEffortLevel} baseUrl=${judgeBaseUrl} cases=${transcript.length}`);

    for (const entry of transcript) {
        const result = await judge(entry);
        const validation = validateJudgement(entry, result);
        results.push({ entry, result, validation });
        const status = validation.pass ? "PASS" : "FAIL";
        console.log(`${status} ${entry.id} ${entry.label} overall=${result.overall} issues=${(result.issues ?? []).join("; ")}`);
    }

    const failures = results.filter((item) => !item.validation.pass);
    const outputPath = path.join(repoRoot, ".antirot/llm-judge-quality-report.json");
    fs.mkdirSync(path.dirname(outputPath), { recursive: true });
    fs.writeFileSync(
        outputPath,
        `${JSON.stringify({
            judgedAt: new Date().toISOString(),
            judgeBaseUrl,
            judgeModel,
            judgeEffortLevel,
            judgeMaxTokens,
            minOverall,
            minDimension,
            results
        }, null, 2)}\n`
    );
    console.log(`Judge report written: ${outputPath}`);

    if (failures.length > 0) {
        const summary = failures
            .map(({ entry, result, validation }) => `${entry.id} ${entry.label}: overall=${result.overall}; low=${validation.lowScores.join(", ")}; issues=${(result.issues ?? []).join("; ")}`)
            .join("\n");
        throw new Error(`LLM judge quality failed:\n${summary}`);
    }
}

main().catch((error) => {
    console.error(error);
    process.exit(1);
});
