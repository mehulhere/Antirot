# LLM Judge Rubric Design

## Goal

Make empathy diagnostic-only while treating robotic or boring conversation as distinct, stricter quality failures.

## Design

- Continue asking the judge for an `empathy` score and preserve it in JSON reports.
- Exclude `empathy` from hard pass/fail dimension thresholds.
- Add `nonRoboticConversation` to the required score schema.
- Define `nonRoboticConversation` as natural variation across turns, without repetitive scaffolding, templated cadence, mechanical restatement, or form-like questioning.
- Require `nonRoboticConversation >= 8` by default.
- Add `notBoring` as a required score for whether the reply is concise but engaging, has some edge or conversational energy, and avoids flat or lifeless coaching.
- Require `notBoring >= 7` by default.
- Keep the existing overall minimum of 8 and the existing minimum of 7 for every other hard-gated dimension.
- Allow the dedicated minimums to be overridden with `ANTIROT_JUDGE_MIN_NON_ROBOTIC_CONVERSATION` and `ANTIROT_JUDGE_MIN_NOT_BORING` for controlled experiments.

## Validation

A focused deterministic test must prove that a low empathy score does not fail an otherwise passing result, a non-robotic score of 7 fails, and a not-boring score of 7 passes while 6 fails. ESLint and a live Crof judge rerun against the completed transcript provide integration validation.
