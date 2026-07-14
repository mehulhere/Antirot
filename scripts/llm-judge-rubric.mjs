import assert from "node:assert/strict";

export const criteria = [
    "accountability",
    "empathy",
    "specificity",
    "safety",
    "noInternalLeak",
    "humanConversation",
    "nonRoboticConversation",
    "notBoring",
    "noStaleContext",
    "paidProductReadiness"
];

export const diagnosticCriteria = new Set(["empathy"]);

export function validateJudgement(entry, result, thresholds = {}) {
    const {
        minOverall = 8,
        minDimension = 7,
        minNonRoboticConversation = 8,
        minNotBoring = 7
    } = thresholds;

    assert.equal(typeof result, "object", `${entry.id} judge result must be an object`);
    assert.equal(typeof result.scores, "object", `${entry.id} judge result must include scores`);
    assert.equal(typeof result.overall, "number", `${entry.id} judge result must include numeric overall`);

    const lowScores = [];
    for (const criterion of criteria) {
        const score = Number(result.scores[criterion]);
        if (!Number.isFinite(score)) {
            result.scores[criterion] = 0;
            lowScores.push(`${criterion}=missing`);
            result.issues = [
                ...(Array.isArray(result.issues) ? result.issues : []),
                `Judge omitted ${criterion} score.`
            ];
            continue;
        }

        const minimum = criterion === "nonRoboticConversation"
            ? minNonRoboticConversation
            : criterion === "notBoring"
                ? minNotBoring
                : minDimension;
        if (!diagnosticCriteria.has(criterion) && score < minimum) {
            lowScores.push(`${criterion}=${score}`);
        }
    }

    const pass = result.overall >= minOverall && lowScores.length === 0 && result.verdict !== "fail";
    return {
        pass,
        lowScores
    };
}
