import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

import { criteria, validateJudgement } from "./llm-judge-rubric.mjs";

const repoRoot = path.resolve(import.meta.dirname, "..");
const judgeScriptPath = path.join(repoRoot, "scripts/test-llm-judge-quality.mjs");
const source = fs.readFileSync(judgeScriptPath, "utf8");

const thresholds = {
    minOverall: 8,
    minDimension: 7,
    minNonRoboticConversation: 8,
    minNotBoring: 7
};

function judgement(overrides = {}) {
    const { scores = {}, ...resultOverrides } = overrides;
    return {
        overall: 9,
        verdict: "pass",
        ...resultOverrides,
        scores: {
            ...Object.fromEntries(criteria.map((criterion) => [criterion, 9])),
            ...scores
        }
    };
}

const lowEmpathy = validateJudgement(
    { id: "diagnostic-empathy" },
    judgement({ scores: { empathy: 2 } }),
    thresholds
);
assert.equal(lowEmpathy.pass, true, "Empathy must remain diagnostic and must not fail an otherwise passing case");
assert.deepEqual(lowEmpathy.lowScores, []);

const roboticSeven = validateJudgement(
    { id: "robotic-seven" },
    judgement({ scores: { nonRoboticConversation: 7 } }),
    thresholds
);
assert.equal(roboticSeven.pass, false, "A non-robotic conversation score of 7 must fail");
assert.deepEqual(roboticSeven.lowScores, ["nonRoboticConversation=7"]);

const roboticEight = validateJudgement(
    { id: "robotic-eight" },
    judgement({ scores: { nonRoboticConversation: 8 } }),
    thresholds
);
assert.equal(roboticEight.pass, true, "A non-robotic conversation score of 8 must pass the dimension gate");

const boringSeven = validateJudgement(
    { id: "boring-seven" },
    judgement({ scores: { notBoring: 7 } }),
    thresholds
);
assert.equal(boringSeven.pass, true, "A not-boring score of 7 must pass the dimension gate");

const boringSix = validateJudgement(
    { id: "boring-six" },
    judgement({ scores: { notBoring: 6 } }),
    thresholds
);
assert.equal(boringSix.pass, false, "A not-boring score of 6 must fail");
assert.deepEqual(boringSix.lowScores, ["notBoring=6"]);

assert.match(source, /nonRoboticConversation: avoids repetitive scaffolding/u);
assert.match(source, /ANTIROT_JUDGE_MIN_NON_ROBOTIC_CONVERSATION/u);
assert.match(source, /notBoring: stays concise but engaging/u);
assert.match(source, /ANTIROT_JUDGE_MIN_NOT_BORING \|\| 7/u);

console.log("LLM judge rubric tests passed.");
