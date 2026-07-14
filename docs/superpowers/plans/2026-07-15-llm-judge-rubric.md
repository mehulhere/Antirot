# LLM Judge Rubric Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep empathy diagnostic-only, hard-gate non-robotic conversation at 8/10, and hard-gate not-boring conversation at 7/10.

**Architecture:** Extend the existing judge schema and prompt in `scripts/test-llm-judge-quality.mjs`. Keep validation deterministic by separating diagnostic dimensions from hard-gated dimensions and selecting the dedicated non-robotic threshold during validation.

**Tech Stack:** Node.js ESM, `node:assert`, ESLint, Crof OpenAI-compatible API.

## Global Constraints

- Empathy remains a required numeric score in judge output and reports.
- Empathy never contributes to `lowScores` and cannot fail a case.
- `nonRoboticConversation` is required and defaults to a hard minimum of 8.
- `notBoring` is required and defaults to a hard minimum of 7.
- Existing overall and other dimension thresholds remain unchanged.

---

### Task 1: Add deterministic rubric coverage

**Files:**
- Create: `scripts/test-llm-judge-rubric.mjs`
- Modify: `package.json`

**Interfaces:**
- Consumes: `validateJudgement(entry, result)` from the judge harness source.
- Produces: a repository command that proves the rubric threshold contract without a network call.

- [x] **Step 1: Write a failing test for diagnostic empathy and the 8/10 non-robotic boundary**
- [x] **Step 2: Run `node scripts/test-llm-judge-rubric.mjs` and confirm the current empathy hard gate fails the test**

### Task 2: Implement the rubric

**Files:**
- Modify: `scripts/test-llm-judge-quality.mjs`

**Interfaces:**
- Consumes: `ANTIROT_JUDGE_MIN_NON_ROBOTIC_CONVERSATION` and `ANTIROT_JUDGE_MIN_NOT_BORING`.
- Produces: judge prompts and reports containing the new dimension and threshold.

- [x] **Step 1: Add the dimension, prompt guidance, diagnostic exclusion, and threshold selection**
- [x] **Step 2: Run the focused test and confirm it passes**
- [x] **Step 3: Run ESLint on changed scripts**
- [x] **Step 4: Rerun the Crof judge against the completed transcript**
- [x] **Step 5: Commit and push the completed rubric change**
