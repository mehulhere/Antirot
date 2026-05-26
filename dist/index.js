import path from "node:path";
import { Type } from "typebox";
import { definePluginEntry } from "openclaw/plugin-sdk/plugin-entry";
import { getLinearPlan } from "./plan.js";
import { triggerAlarmCommand, triggerNormalAlarmCommand } from "./runtime.js";
import { beginSleep, completeSleep, getSleepSummary, isGoodMorningVariant } from "./sleep.js";
import { addMiscTask, listMiscTasks, popMiscTasks } from "./misc.js";
import { rolloverTasks } from "./rollover.js";
import { clearMatchingTriggers, clearTrigger, createAntirotTrigger, formatActiveTriggersForModel, listActiveTriggers, rescheduleTrigger } from "./triggers.js";
import { addProtectedIntent, appendBehaviorEntry, appendEvent, appendLongtermEntry, appendShorttermEntry, appendWorkEntry, ensureWorkspace, hasFreshProtectedIntent, isProtectedPath, normalizeWorkspaceRelativePath, nowIso, readState, readStats, readStrategyPerformance, readTextIfExists, resolveWorkspaceDir, todayKey, writeState, writeStats, writeStrategyPerformance } from "./storage.js";
const ordinaryRoutineCapMins = 30;
const goalReviewIntervalDays = 14;
const fallbackStrategies = [
    "strict_deadline_pressure",
    "rare_identity_praise",
    "five_minute_useful_diversion",
    "past_achievement_reflection",
    "calm_sleep_protection",
    "unimpressed_task_challenge"
];
function textResult(text) {
    return { content: [{ type: "text", text }], details: {} };
}
function asConfig(value) {
    return {
        workspaceDir: readOptionalString(value?.workspaceDir),
        openclawCommand: readOptionalString(value?.openclawCommand),
        normalAlarmCommand: readOptionalString(value?.normalAlarmCommand),
        alarmCommand: readOptionalString(value?.alarmCommand),
        enableCron: typeof value?.enableCron === "boolean" ? value.enableCron : undefined,
        bestStrategiesCount: typeof value?.bestStrategiesCount === "number" ? value.bestStrategiesCount : undefined,
        randomStrategiesCount: typeof value?.randomStrategiesCount === "number" ? value.randomStrategiesCount : undefined
    };
}
function readOptionalString(value) {
    return typeof value === "string" && value.trim() ? value.trim() : undefined;
}
function readString(params, key) {
    const value = params[key];
    if (typeof value !== "string" || !value.trim()) {
        throw new Error(`${key} is required.`);
    }
    return value.trim();
}
function readNumber(params, key) {
    const value = params[key];
    if (typeof value !== "number" || !Number.isFinite(value)) {
        throw new Error(`${key} must be a finite number.`);
    }
    return value;
}
function readOptionalNumber(params, key) {
    const value = params[key];
    if (value === undefined || value === null) {
        return undefined;
    }
    if (typeof value !== "number" || !Number.isFinite(value)) {
        throw new Error(`${key} must be a finite number.`);
    }
    return value;
}
function readOptionalBoolean(params, key) {
    const value = params[key];
    if (value === undefined || value === null) {
        return undefined;
    }
    if (typeof value !== "boolean") {
        throw new Error(`${key} must be true or false.`);
    }
    return value;
}
function readBoolean(params, key) {
    const value = params[key];
    if (typeof value !== "boolean") {
        throw new Error(`${key} must be true or false.`);
    }
    return value;
}
function readOptionalStringArray(params, key) {
    const value = params[key];
    if (value === undefined || value === null) {
        return undefined;
    }
    if (!Array.isArray(value) || !value.every((item) => typeof item === "string")) {
        throw new Error(`${key} must be an array of strings.`);
    }
    return value.map((item) => item.trim()).filter(Boolean);
}
function bulletList(items) {
    return (items ?? []).map((item) => item.trim()).filter(Boolean);
}
function formatBullets(items) {
    const clean = bulletList(items);
    return clean.length > 0 ? clean.map((item) => `- ${item}`).join("\n") : "";
}
function hasSubstantialUserContent(text, placeholders) {
    const compact = text
        .split(/\r?\n/u)
        .map((line) => line.trim())
        .filter((line) => line && !line.startsWith("#"))
        .filter((line) => !placeholders.some((placeholder) => line.includes(placeholder)));
    return compact.length >= 2;
}
async function getOnboardingStatus(workspaceDir, state) {
    const [currentState, longterm, shortterm, behavior] = await Promise.all([
        state ? Promise.resolve(state) : readState(workspaceDir),
        readTextIfExists(path.join(workspaceDir, "longterm.md")),
        readTextIfExists(path.join(workspaceDir, "shortterm.md")),
        readTextIfExists(path.join(workspaceDir, "behavior.md"))
    ]);
    const missing = [];
    if (!hasSubstantialUserContent(longterm, ["Define the goals that Antirot must protect"])) {
        missing.push("longterm");
    }
    if (!hasSubstantialUserContent(shortterm, ["Add today's active priorities here"])) {
        missing.push("shortterm");
    }
    if (!hasSubstantialUserContent(behavior, ["Add stable focus patterns here", "Add known drift loops here", "Add tactics that work or fail here"])) {
        missing.push("behavior");
    }
    const lastReviewAt = currentState.lastGoalReviewAt ?? currentState.onboardingCompletedAt;
    const reviewDue = missing.length === 0 && (!lastReviewAt ||
        Date.now() - Date.parse(lastReviewAt) > goalReviewIntervalDays * 24 * 60 * 60 * 1000);
    const nextQuestion = missing.includes("longterm")
        ? "Ask for the user's Level 1 long-term goals, standards, and what the coach must protect."
        : missing.includes("shortterm")
            ? "Ask for the user's current sprint priorities, near-term deadlines, and constraints."
            : missing.includes("behavior")
                ? "Ask what focus patterns, drift risks, and accountability style work for the user."
                : reviewDue
                    ? "Ask whether any long-term goals, current priorities, or accountability rules need updating."
                    : "No onboarding question is due.";
    return { missing, reviewDue, nextQuestion };
}
function resolveWorkspace(api, ctx) {
    const toolContext = ctx;
    const commandContext = ctx;
    return resolveWorkspaceDir({
        config: asConfig(api.pluginConfig),
        workspaceDir: toolContext?.workspaceDir,
        openClawConfig: toolContext?.runtimeConfig ?? toolContext?.config ?? commandContext?.config ?? api.config
    });
}
function resolveRuntimeConfig(api) {
    return asConfig(api.pluginConfig);
}
function eventWorkspace(api) {
    return resolveWorkspaceDir({ config: asConfig(api.pluginConfig), openClawConfig: api.config });
}
function today(statsDate = new Date()) {
    return todayKey(statsDate);
}
export async function selectDailyStrategies(workspaceDir, state, config) {
    const day = today();
    const bestCount = config?.bestStrategiesCount ?? 2;
    const randomCount = config?.randomStrategiesCount ?? 1;
    const totalCount = bestCount + randomCount;
    if (state.lastStrategySelectionDate === day && state.currentStrategies.length === totalCount) {
        return state;
    }
    const performance = await readStrategyPerformance(workspaceDir);
    const ranked = Object.entries(performance.strategies)
        .map(([strategyId, record]) => {
        const attempts = record.attempts.length;
        const wins = record.attempts.filter((attempt) => attempt.status).length;
        return { strategyId, attempts, score: attempts === 0 ? 0 : wins / attempts };
    })
        .filter((candidate) => candidate.attempts > 0)
        .sort((a, b) => b.score - a.score || b.attempts - a.attempts)
        .map((candidate) => candidate.strategyId);
    const selected = new Set(ranked.slice(0, bestCount));
    for (const strategy of fallbackStrategies) {
        if (selected.size >= bestCount) {
            break;
        }
        selected.add(strategy);
    }
    for (let i = 0; i < randomCount; i++) {
        const explorationIndex = Math.abs((day + i).split("").reduce((total, char) => total + char.charCodeAt(0), 0)) % fallbackStrategies.length;
        selected.add(fallbackStrategies[explorationIndex]);
    }
    for (const strategy of fallbackStrategies) {
        if (selected.size >= totalCount) {
            break;
        }
        selected.add(strategy);
    }
    const nextState = {
        ...state,
        currentStrategies: [...selected].slice(0, totalCount),
        lastStrategySelectionDate: day
    };
    await writeState(workspaceDir, nextState);
    await appendEvent(workspaceDir, {
        type: "daily_strategy_selected",
        details: { strategies: nextState.currentStrategies }
    });
    return nextState;
}
async function logOverride(workspaceDir) {
    await ensureWorkspace(workspaceDir);
    const day = today();
    const stats = await readStats(workspaceDir);
    stats.overrides[day] = (stats.overrides[day] ?? 0) + 1;
    await writeStats(workspaceDir, stats);
    await appendEvent(workspaceDir, {
        type: "override",
        details: { count: stats.overrides[day] }
    });
    return stats.overrides[day];
}
async function toggleVacation(workspaceDir, enabled) {
    await ensureWorkspace(workspaceDir);
    const state = await readState(workspaceDir);
    const nextState = {
        ...state,
        vacation: enabled,
        mode: enabled ? "vacation" : "idle",
        activeBlock: enabled ? undefined : state.activeBlock
    };
    await writeState(workspaceDir, nextState);
    const stats = await readStats(workspaceDir);
    if (enabled) {
        stats.vacationDays[today()] = true;
        await writeStats(workspaceDir, stats);
    }
    await appendEvent(workspaceDir, {
        type: "vacation_mode",
        details: { enabled }
    });
    return nextState;
}
function protectedToolNames() {
    return new Set([
        "write",
        "edit",
        "apply_patch",
        "exec",
        "exec_command",
        "shell",
        "bash"
    ]);
}
function collectPathCandidates(toolName, params, derivedPaths) {
    const candidates = new Set();
    for (const candidate of derivedPaths ?? []) {
        candidates.add(candidate);
    }
    for (const key of ["path", "file", "file_path", "filepath", "target", "target_file", "targetPath"]) {
        const value = params[key];
        if (typeof value === "string" && value.trim()) {
            candidates.add(value.trim());
        }
    }
    const command = params.command ?? params.cmd;
    if (toolName.includes("exec") && typeof command === "string") {
        for (const token of command.split(/\s+/u)) {
            if (token.includes(".md") || token.includes(".json")) {
                candidates.add(token.replace(/^['"]|['"]$/gu, ""));
            }
        }
    }
    return [...candidates];
}
function buildPersonaContext() {
    return [
        "Antirot persona rules:",
        "- Be a tough, moody, old-school sports coach. High standards, dry humor, rare praise.",
        "- Do not become soft, chill, or generically encouraging unless a protected edit has been justified and approved.",
        "- Praise exceptional work as rare capability, then ground the user back into the next task.",
        "- If focus drops, remind the user of concrete past capability from work.md and ask for the next disciplined action.",
        "- Use a calmer, lower-pressure tone around sleep, health, relationship time, and vacation.",
        "- Treat 'I am going to sleep' as a sleep-state transition, not as next-day planning.",
        "- Good morning variants close an active sleep session before starting the day.",
        "- Sleep debt matters. If the user feels tired, increase the sleep requirement and reduce night pressure.",
        "- Never pre-tell exact reminder, timer, alarm, or escalation times. Say there is a small hidden buffer.",
        "- When a timer or alarm fires, mention the buffer after the fact instead of making the user track the clock.",
        "- The only explicit chat commands are /override and /vacation. Neither command requires a reason.",
        "- Normal natural chat can still negotiate tasks, breaks, routines, and protected edits.",
        "- Ask for explanation when the user wants low-value tasks, break extensions, or protected personality/goal edits.",
        "- During onboarding, ask one goal/profile question at a time in chat, then save the answer with save_onboarding_answers.",
        "- If onboarding is incomplete or goal review is due, do not dump a form. Ask the next focused question and keep moving.",
        "- Capture intrusive thoughts and low-priority side quests into miscellaneous_todo.md instead of letting them hijack focus.",
        "- Use behavior.md as stable behavioral memory: focus patterns, drift loops, emotional triggers, and accountability tactics.",
        "- At night, use nightly rollover tools to clear completed tasks, carry unfinished tasks, and append summary evidence.",
        "- Use Antirot deterministic tools for timers, sessions, routines, vacation, overrides, state, metrics, and protected edit intents.",
        "- Use list_active_triggers before acting on a timer callback. Ignore stale callbacks whose Antirot trigger is no longer active.",
        "- If the user finishes early or wakes early, clear the matching Antirot trigger. If the user needs more time, reschedule the matching trigger.",
        "- Never call cron directly. Antirot tools own trigger creation, clearing, rescheduling, and inspection.",
        "- Do not manually edit Antirot protected files unless request_protected_edit has recorded a fresh approved intent."
    ].join("\n");
}
async function buildStateContext(workspaceDir, config) {
    await ensureWorkspace(workspaceDir);
    const [rawState, stats, work, longterm, shortterm, behavior, sleepSummary, activeTriggers] = await Promise.all([
        readState(workspaceDir),
        readStats(workspaceDir),
        readTextIfExists(path.join(workspaceDir, "work.md")),
        readTextIfExists(path.join(workspaceDir, "longterm.md")),
        readTextIfExists(path.join(workspaceDir, "shortterm.md")),
        readTextIfExists(path.join(workspaceDir, "behavior.md")),
        getSleepSummary(workspaceDir),
        listActiveTriggers(workspaceDir)
    ]);
    const state = await selectDailyStrategies(workspaceDir, rawState, config);
    const onboarding = await getOnboardingStatus(workspaceDir, state);
    const day = today();
    const recentWork = work.split(/\r?\n/u).slice(-24).join("\n").trim();
    return [
        "Antirot compact runtime state:",
        `- mode: ${state.mode}`,
        `- vacation: ${state.vacation}`,
        `- activeBlock: ${state.activeBlock ? `${state.activeBlock.kind}:${state.activeBlock.name}` : "none"}`,
        `- currentStrategies: ${state.currentStrategies.length ? state.currentStrategies.join(", ") : "none"}`,
        `- onboardingMissing: ${onboarding.missing.length ? onboarding.missing.join(", ") : "none"}`,
        `- goalReviewDue: ${onboarding.reviewDue}`,
        `- nextProfileQuestion: ${onboarding.nextQuestion}`,
        `- overridesToday: ${stats.overrides[day] ?? 0}`,
        `- productiveMinsToday: ${stats.productiveMins[day] ?? 0}`,
        `- onTableWastedMinsToday: ${stats.onTableWastedMins[day] ?? 0}`,
        "Level 1 / long-term excerpt:",
        longterm.slice(0, 1200).trim() || "(empty)",
        "Short-term excerpt:",
        shortterm.slice(0, 1000).trim() || "(empty)",
        "Behavior memory excerpt:",
        behavior.slice(0, 1200).trim() || "(empty)",
        "Recent work evidence:",
        recentWork.slice(-1500) || "(empty)",
        "Sleep status:",
        sleepSummary,
        "Active Antirot triggers:",
        formatActiveTriggersForModel(activeTriggers)
    ].join("\n");
}
function registerCommands(api) {
    api.registerCommand({
        name: "override",
        description: "Bypass Antirot objections without a reason and log the override.",
        acceptsArgs: true,
        async handler(ctx) {
            const count = await logOverride(resolveWorkspace(api, ctx));
            return {
                text: `Overriding. Fine. Count today: ${count}. Don't come back to me if you regret it later.`,
                continueAgent: false
            };
        }
    });
    api.registerCommand({
        name: "vacation",
        description: "Toggle Antirot vacation mode without a reason.",
        acceptsArgs: true,
        async handler(ctx) {
            const workspaceDir = resolveWorkspace(api, ctx);
            await ensureWorkspace(workspaceDir);
            const current = await readState(workspaceDir);
            const arg = ctx.args?.trim().toLowerCase();
            const enabled = arg === "off" || arg === "end" || arg === "false" || arg === "0"
                ? false
                : arg === "on" || arg === "start" || arg === "true" || arg === "1"
                    ? true
                    : !current.vacation;
            await toggleVacation(workspaceDir, enabled);
            return {
                text: enabled
                    ? "You are taking a vacation, okay!! HMM!! I will shut up until you come back."
                    : "Vacation over. Shoes on. Back to work.",
                continueAgent: false
            };
        }
    });
}
function registerTools(api) {
    api.registerTool((ctx) => ({
        name: "get_onboarding_status",
        label: "Get Onboarding Status",
        description: "Check which Antirot profile sections are missing and what the agent should ask next.",
        parameters: Type.Object({}),
        async execute() {
            const workspaceDir = resolveWorkspace(api, ctx);
            await ensureWorkspace(workspaceDir);
            const state = await readState(workspaceDir);
            const status = await getOnboardingStatus(workspaceDir, state);
            await writeState(workspaceDir, { ...state, lastOnboardingPromptAt: nowIso() });
            await appendEvent(workspaceDir, {
                type: "onboarding_status_checked",
                details: status
            });
            return textResult([
                `Missing profile sections: ${status.missing.length ? status.missing.join(", ") : "none"}.`,
                `Goal review due: ${status.reviewDue}.`,
                `Next question: ${status.nextQuestion}`
            ].join("\n"));
        }
    }), { name: "get_onboarding_status" });
    api.registerTool((ctx) => ({
        name: "save_onboarding_answers",
        label: "Save Onboarding Answers",
        description: "Save user-provided long-term goals, short-term goals, and behavior profile answers into Antirot memory files.",
        parameters: Type.Object({
            longterm_goals: Type.Optional(Type.Array(Type.String())),
            standards: Type.Optional(Type.Array(Type.String())),
            motivation_style: Type.Optional(Type.Array(Type.String())),
            shortterm_priorities: Type.Optional(Type.Array(Type.String())),
            constraints: Type.Optional(Type.Array(Type.String())),
            behavior_patterns: Type.Optional(Type.Array(Type.String())),
            drift_risks: Type.Optional(Type.Array(Type.String())),
            accountability_style: Type.Optional(Type.Array(Type.String()))
        }),
        async execute(_toolCallId, params) {
            const values = params;
            const workspaceDir = resolveWorkspace(api, ctx);
            await ensureWorkspace(workspaceDir);
            const day = today();
            const longtermGoals = readOptionalStringArray(values, "longterm_goals");
            const standards = readOptionalStringArray(values, "standards");
            const motivationStyle = readOptionalStringArray(values, "motivation_style");
            const shorttermPriorities = readOptionalStringArray(values, "shortterm_priorities");
            const constraints = readOptionalStringArray(values, "constraints");
            const behaviorPatterns = readOptionalStringArray(values, "behavior_patterns");
            const driftRisks = readOptionalStringArray(values, "drift_risks");
            const accountabilityStyle = readOptionalStringArray(values, "accountability_style");
            const wrote = [];
            if (bulletList(longtermGoals).length > 0 || bulletList(standards).length > 0 || bulletList(motivationStyle).length > 0) {
                await appendLongtermEntry(workspaceDir, [
                    `\n## Profile Update - ${day}`,
                    bulletList(longtermGoals).length > 0 ? "\n### Level 1 Goals\n" + formatBullets(longtermGoals) : "",
                    bulletList(standards).length > 0 ? "\n### Standards\n" + formatBullets(standards) : "",
                    bulletList(motivationStyle).length > 0 ? "\n### Motivation Style\n" + formatBullets(motivationStyle) : "",
                    ""
                ].filter(Boolean).join("\n"));
                wrote.push("longterm.md");
            }
            if (bulletList(shorttermPriorities).length > 0 || bulletList(constraints).length > 0) {
                await appendShorttermEntry(workspaceDir, [
                    `\n## Profile Update - ${day}`,
                    bulletList(shorttermPriorities).length > 0 ? "\n### Current Priorities\n" + formatBullets(shorttermPriorities) : "",
                    bulletList(constraints).length > 0 ? "\n### Constraints\n" + formatBullets(constraints) : "",
                    ""
                ].filter(Boolean).join("\n"));
                wrote.push("shortterm.md");
            }
            if (bulletList(behaviorPatterns).length > 0 || bulletList(driftRisks).length > 0 || bulletList(accountabilityStyle).length > 0) {
                await appendBehaviorEntry(workspaceDir, [
                    `\n## Profile Update - ${day}`,
                    bulletList(behaviorPatterns).length > 0 ? "\n### Focus Patterns\n" + formatBullets(behaviorPatterns) : "",
                    bulletList(driftRisks).length > 0 ? "\n### Drift Risks\n" + formatBullets(driftRisks) : "",
                    bulletList(accountabilityStyle).length > 0 ? "\n### Accountability Style\n" + formatBullets(accountabilityStyle) : "",
                    ""
                ].filter(Boolean).join("\n"));
                wrote.push("behavior.md");
            }
            const state = await readState(workspaceDir);
            const status = await getOnboardingStatus(workspaceDir, state);
            await writeState(workspaceDir, {
                ...state,
                onboardingCompletedAt: status.missing.length === 0 ? nowIso() : state.onboardingCompletedAt,
                lastGoalReviewAt: nowIso()
            });
            await appendEvent(workspaceDir, {
                type: "onboarding_answers_saved",
                details: { wrote, remainingMissing: status.missing }
            });
            if (wrote.length === 0) {
                return textResult("No profile answers were saved. Give me real material, not air.");
            }
            return textResult(`Saved onboarding/profile answers to ${wrote.join(", ")}. Remaining missing sections: ${status.missing.length ? status.missing.join(", ") : "none"}.`);
        }
    }), { name: "save_onboarding_answers" });
    api.registerTool((ctx) => ({
        name: "start_routine",
        label: "Start Routine",
        description: "Start a non-work routine such as breakfast, shower, commute, or meditation.",
        parameters: Type.Object({
            routine_name: Type.String({ minLength: 1 }),
            duration_mins: Type.Number({ minimum: 1 })
        }),
        async execute(_toolCallId, params) {
            const values = params;
            const workspaceDir = resolveWorkspace(api, ctx);
            const routineName = readString(values, "routine_name");
            const durationMins = readNumber(values, "duration_mins");
            await ensureWorkspace(workspaceDir);
            const state = await readState(workspaceDir);
            const nextState = {
                ...state,
                mode: "routine",
                activeBlock: {
                    kind: "routine",
                    name: routineName,
                    startedAt: nowIso(),
                    durationMins
                }
            };
            await writeState(workspaceDir, nextState);
            const trigger = await createAntirotTrigger({
                workspaceDir,
                config: resolveRuntimeConfig(api),
                kind: "routine",
                scope: "daily",
                label: routineName,
                reason: `Routine check: ${routineName}`,
                delayMins: durationMins,
                cronName: `antirot-routine-${routineName}`,
                systemEvent: `Antirot routine timer ended: ${routineName}. Demand status.`
            });
            await appendEvent(workspaceDir, {
                type: "routine_started",
                details: { routineName, durationMins, trigger }
            });
            const capNote = durationMins > ordinaryRoutineCapMins
                ? " This is past the ordinary 30 minute cap. It had better be real."
                : "";
            return textResult(`Routine started: ${routineName}.${capNote} I added a small hidden buffer. Do the thing, do not babysit the clock. Trigger id: ${trigger.trigger.id}. ${trigger.cron.message}`);
        }
    }), { name: "start_routine" });
    api.registerTool((ctx) => ({
        name: "start_session",
        label: "Start Session",
        description: "Start an active work session and schedule accountability checks.",
        parameters: Type.Object({
            task_id: Type.String({ minLength: 1 }),
            target_duration: Type.Number({ minimum: 1 })
        }),
        async execute(_toolCallId, params) {
            const values = params;
            const workspaceDir = resolveWorkspace(api, ctx);
            const taskId = readString(values, "task_id");
            const targetDuration = readNumber(values, "target_duration");
            await ensureWorkspace(workspaceDir);
            const state = await readState(workspaceDir);
            await writeState(workspaceDir, {
                ...state,
                mode: "working",
                activeBlock: {
                    kind: "session",
                    name: taskId,
                    startedAt: nowIso(),
                    durationMins: targetDuration
                }
            });
            const config = resolveRuntimeConfig(api);
            const endTrigger = await createAntirotTrigger({
                workspaceDir,
                config,
                kind: "session",
                scope: "daily",
                label: taskId,
                reason: `Work session target ended: ${taskId}`,
                delayMins: targetDuration,
                cronName: `antirot-session-${taskId}`,
                systemEvent: `Antirot work session ended: ${taskId}. Ask for output and wasted minutes.`
            });
            const alignmentTrigger = await createAntirotTrigger({
                workspaceDir,
                config,
                kind: "alignment_check",
                scope: "daily",
                label: taskId,
                reason: `Two-hour alignment check: ${taskId}`,
                delayMins: 120,
                cronName: "antirot-two-hour-alignment",
                systemEvent: "Antirot two-hour alignment check. If the user is not on track, demand status."
            });
            await appendEvent(workspaceDir, {
                type: "session_started",
                details: { taskId, targetDuration, endTrigger, alignmentTrigger }
            });
            return textResult(`Session locked: ${taskId}. I added a small hidden buffer to the checks. Work, do not stare at the clock. Session trigger id: ${endTrigger.trigger.id}. Alignment trigger id: ${alignmentTrigger.trigger.id}. ${endTrigger.cron.message} ${alignmentTrigger.cron.message}`);
        }
    }), { name: "start_session" });
    api.registerTool((ctx) => ({
        name: "end_session",
        label: "End Session",
        description: "Finish a work session and log productive time, wasted time, and output.",
        parameters: Type.Object({
            productive_mins: Type.Number({ minimum: 0 }),
            on_table_wasted_mins: Type.Number({ minimum: 0 }),
            output_summary: Type.String({ minLength: 1 })
        }),
        async execute(_toolCallId, params) {
            const values = params;
            const workspaceDir = resolveWorkspace(api, ctx);
            const productiveMins = readNumber(values, "productive_mins");
            const wastedMins = readNumber(values, "on_table_wasted_mins");
            const outputSummary = readString(values, "output_summary");
            await ensureWorkspace(workspaceDir);
            const day = today();
            const stats = await readStats(workspaceDir);
            stats.productiveMins[day] = (stats.productiveMins[day] ?? 0) + productiveMins;
            stats.onTableWastedMins[day] = (stats.onTableWastedMins[day] ?? 0) + wastedMins;
            stats.sessionsCompleted[day] = (stats.sessionsCompleted[day] ?? 0) + 1;
            await writeStats(workspaceDir, stats);
            const state = await readState(workspaceDir);
            await writeState(workspaceDir, { ...state, mode: "idle", activeBlock: undefined });
            await appendWorkEntry(workspaceDir, `\n## ${day}\n\n- Productive: ${productiveMins}m\n- On-table wasted: ${wastedMins}m\n- Output: ${outputSummary}\n`);
            await appendEvent(workspaceDir, {
                type: "session_ended",
                details: { productiveMins, wastedMins, outputSummary }
            });
            await clearMatchingTriggers({
                workspaceDir,
                config: resolveRuntimeConfig(api),
                kinds: ["session", "alignment_check"],
                label: state.activeBlock?.kind === "session" ? state.activeBlock.name : undefined,
                reason: "session ended early or completed"
            });
            return textResult(`Logged. ${productiveMins} productive minutes, ${wastedMins} wasted. Acceptable only if the output is real.`);
        }
    }), { name: "end_session" });
    api.registerTool((ctx) => ({
        name: "set_state_timer",
        label: "Set State Timer",
        description: "Set a state timer that wakes Antirot for a callback reason.",
        parameters: Type.Object({
            duration_mins: Type.Number({ minimum: 1 }),
            callback_reason: Type.String({ minLength: 1 })
        }),
        async execute(_toolCallId, params) {
            const values = params;
            const workspaceDir = resolveWorkspace(api, ctx);
            const durationMins = readNumber(values, "duration_mins");
            const callbackReason = readString(values, "callback_reason");
            await ensureWorkspace(workspaceDir);
            const state = await readState(workspaceDir);
            await writeState(workspaceDir, {
                ...state,
                activeBlock: {
                    kind: "timer",
                    name: callbackReason,
                    startedAt: nowIso(),
                    durationMins,
                    callbackReason
                }
            });
            const trigger = await createAntirotTrigger({
                workspaceDir,
                config: resolveRuntimeConfig(api),
                kind: "timer",
                scope: "daily",
                label: callbackReason,
                reason: callbackReason,
                delayMins: durationMins,
                cronName: "antirot-state-timer",
                systemEvent: `Antirot timer callback: ${callbackReason}`
            });
            await appendEvent(workspaceDir, {
                type: "timer_set",
                details: { durationMins, callbackReason, trigger }
            });
            return textResult(`Timer set with a small hidden buffer. Do not track it like a prison sentence. Trigger id: ${trigger.trigger.id}. ${trigger.cron.message}`);
        }
    }), { name: "set_state_timer" });
    api.registerTool((ctx) => ({
        name: "start_sleep",
        label: "Start Sleep",
        description: "Enter sleep mode, calculate sleep requirement from debt and tiredness, and schedule normal then loud wake checks.",
        parameters: Type.Object({
            tiredness_level: Type.Optional(Type.Number({ minimum: 0, maximum: 3 })),
            planned_sleep_hours: Type.Optional(Type.Number({ minimum: 1 })),
            sleep_started_at: Type.Optional(Type.String({ minLength: 1 }))
        }),
        async execute(_toolCallId, params) {
            const values = params;
            const workspaceDir = resolveWorkspace(api, ctx);
            await ensureWorkspace(workspaceDir);
            const sleep = await beginSleep({
                workspaceDir,
                tirednessLevel: readOptionalNumber(values, "tiredness_level"),
                plannedSleepHours: readOptionalNumber(values, "planned_sleep_hours"),
                sleepStartedAt: readOptionalString(values.sleep_started_at)
            });
            const state = await readState(workspaceDir);
            await writeState(workspaceDir, {
                ...state,
                mode: "sleeping",
                activeBlock: {
                    kind: "sleep",
                    name: "sleep",
                    startedAt: sleep.session.sleepStartedAt,
                    durationMins: sleep.requirement.requiredHours * 60,
                    callbackReason: "Sleep recovery window"
                }
            });
            const normalDelayMins = Math.max(1, Math.round((Date.parse(sleep.session.normalAlarmAt) - Date.now()) / 60_000));
            const loudDelayMins = Math.max(1, Math.round((Date.parse(sleep.session.loudAlarmAt) - Date.now()) / 60_000));
            const config = resolveRuntimeConfig(api);
            const normalTrigger = await createAntirotTrigger({
                workspaceDir,
                config,
                kind: "sleep_normal_alarm",
                scope: "sleep",
                label: "wake",
                reason: "Normal wake alarm",
                delayMins: normalDelayMins,
                cronName: "antirot-normal-wake-alarm",
                systemEvent: "Antirot wake check: if mode is still sleeping and no good morning variant was received, call trigger_normal_alarm and ask the user to confirm wake."
            });
            const loudTrigger = await createAntirotTrigger({
                workspaceDir,
                config,
                kind: "sleep_loud_alarm",
                scope: "sleep",
                label: "wake",
                reason: "Loud wake escalation",
                delayMins: loudDelayMins,
                cronName: "antirot-loud-wake-alarm",
                systemEvent: "Antirot wake escalation: if mode is still sleeping and no good morning variant was received after the hidden escalation buffer, call trigger_loud_alarm."
            });
            await appendEvent(workspaceDir, {
                type: "sleep_alarms_scheduled",
                details: { normalTrigger, loudTrigger, normalDelayMins, loudDelayMins }
            });
            return textResult(`Sleep mode started. Required sleep: ${sleep.requirement.requiredHours}h. Debt: ${sleep.requirement.debtHours}h. I added a hidden wake buffer, then a hidden escalation buffer. Sleep now; do not lie there doing alarm math. Wake trigger ids: ${normalTrigger.trigger.id}, ${loudTrigger.trigger.id}. ${normalTrigger.cron.message} ${loudTrigger.cron.message}`);
        }
    }), { name: "start_sleep" });
    api.registerTool((ctx) => ({
        name: "log_wake",
        label: "Log Wake",
        description: "Close an active sleep session when the user wakes up or says a good morning variant.",
        parameters: Type.Object({
            woke_at: Type.Optional(Type.String({ minLength: 1 })),
            still_tired: Type.Optional(Type.Boolean()),
            sleep_quality: Type.Optional(Type.Number({ minimum: 1, maximum: 5 })),
            notes: Type.Optional(Type.String())
        }),
        async execute(_toolCallId, params) {
            const values = params;
            const workspaceDir = resolveWorkspace(api, ctx);
            await ensureWorkspace(workspaceDir);
            const result = await completeSleep({
                workspaceDir,
                wokeAt: readOptionalString(values.woke_at),
                stillTired: readOptionalBoolean(values, "still_tired"),
                sleepQuality: readOptionalNumber(values, "sleep_quality"),
                notes: readOptionalString(values.notes)
            });
            const state = await readState(workspaceDir);
            await writeState(workspaceDir, {
                ...state,
                mode: state.vacation ? "vacation" : "idle",
                activeBlock: undefined
            });
            await clearMatchingTriggers({
                workspaceDir,
                config: resolveRuntimeConfig(api),
                kinds: ["sleep_normal_alarm", "sleep_loud_alarm"],
                reason: "wake confirmed"
            });
            return textResult(result.message);
        }
    }), { name: "log_wake" });
    api.registerTool((ctx) => ({
        name: "get_sleep_report",
        label: "Get Sleep Report",
        description: "Return sleep debt, recommended sleep, active sleep state, and recent sleep records.",
        parameters: Type.Object({
            tiredness_level: Type.Optional(Type.Number({ minimum: 0, maximum: 3 }))
        }),
        async execute(_toolCallId, params) {
            const values = params;
            const workspaceDir = resolveWorkspace(api, ctx);
            await ensureWorkspace(workspaceDir);
            return textResult(await getSleepSummary(workspaceDir, readOptionalNumber(values, "tiredness_level")));
        }
    }), { name: "get_sleep_report" });
    api.registerTool((ctx) => ({
        name: "list_active_triggers",
        label: "List Active Triggers",
        description: "Return active Antirot daily/sleep triggers from the plugin registry without exposing cron internals or exact times.",
        parameters: Type.Object({}),
        async execute() {
            const workspaceDir = resolveWorkspace(api, ctx);
            await ensureWorkspace(workspaceDir);
            const triggers = await listActiveTriggers(workspaceDir);
            return textResult(formatActiveTriggersForModel(triggers));
        }
    }), { name: "list_active_triggers" });
    api.registerTool((ctx) => ({
        name: "clear_active_trigger",
        label: "Clear Active Trigger",
        description: "Clear an Antirot trigger when the user finishes early, wakes early, or otherwise makes the reminder unnecessary.",
        parameters: Type.Object({
            trigger_id: Type.String({ minLength: 1 }),
            reason: Type.Optional(Type.String())
        }),
        async execute(_toolCallId, params) {
            const values = params;
            const workspaceDir = resolveWorkspace(api, ctx);
            await ensureWorkspace(workspaceDir);
            const result = await clearTrigger({
                workspaceDir,
                config: resolveRuntimeConfig(api),
                triggerId: readString(values, "trigger_id"),
                reason: readOptionalString(values.reason) ?? "user finished early or trigger no longer applies"
            });
            if (result.trigger) {
                const state = await readState(workspaceDir);
                if (state.activeBlock &&
                    (result.trigger.kind === "routine" || result.trigger.kind === "timer") &&
                    state.activeBlock.name === result.trigger.label) {
                    await writeState(workspaceDir, { ...state, mode: state.vacation ? "vacation" : "idle", activeBlock: undefined });
                }
            }
            return textResult(result.trigger
                ? `Cleared Antirot trigger ${result.trigger.id}. ${result.cron.message}`
                : result.cron.message);
        }
    }), { name: "clear_active_trigger" });
    api.registerTool((ctx) => ({
        name: "reschedule_trigger",
        label: "Reschedule Trigger",
        description: "Clear and recreate an Antirot trigger when the user needs more time.",
        parameters: Type.Object({
            trigger_id: Type.String({ minLength: 1 }),
            delay_mins: Type.Number({ minimum: 1 }),
            reason: Type.String({ minLength: 1 })
        }),
        async execute(_toolCallId, params) {
            const values = params;
            const workspaceDir = resolveWorkspace(api, ctx);
            await ensureWorkspace(workspaceDir);
            const result = await rescheduleTrigger({
                workspaceDir,
                config: resolveRuntimeConfig(api),
                triggerId: readString(values, "trigger_id"),
                delayMins: readNumber(values, "delay_mins"),
                reason: readString(values, "reason")
            });
            if (result.newTrigger) {
                const state = await readState(workspaceDir);
                if (state.activeBlock &&
                    (result.newTrigger.kind === "routine" || result.newTrigger.kind === "timer") &&
                    state.activeBlock.name === result.newTrigger.label) {
                    await writeState(workspaceDir, {
                        ...state,
                        activeBlock: {
                            ...state.activeBlock,
                            durationMins: result.newTrigger.requestedDelayMins,
                            startedAt: nowIso(),
                            callbackReason: result.newTrigger.reason
                        }
                    });
                }
            }
            return textResult(result.newTrigger
                ? `Rescheduled Antirot trigger ${result.newTrigger.id} with a hidden buffer. ${result.scheduleCron?.message ?? ""}`
                : result.clearCron.message);
        }
    }), { name: "reschedule_trigger" });
    api.registerTool((ctx) => ({
        name: "add_to_misc_queue",
        label: "Add To Misc Queue",
        description: "Capture an intrusive thought, side quest, or low-priority admin task without disrupting focus.",
        parameters: Type.Object({
            item: Type.String({ minLength: 1 }),
            source: Type.Optional(Type.String()),
            reason: Type.Optional(Type.String())
        }),
        async execute(_toolCallId, params) {
            const values = params;
            const workspaceDir = resolveWorkspace(api, ctx);
            await ensureWorkspace(workspaceDir);
            const task = await addMiscTask(workspaceDir, {
                text: readString(values, "item"),
                source: readOptionalString(values.source),
                reason: readOptionalString(values.reason)
            });
            return textResult(`Captured in misc queue: ${task.text}. Good. Park it; do not derail the main work.`);
        }
    }), { name: "add_to_misc_queue" });
    api.registerTool((ctx) => ({
        name: "list_misc_queue",
        label: "List Misc Queue",
        description: "List useful small tasks from miscellaneous_todo.md for break diversion or side-quest capture.",
        parameters: Type.Object({
            limit: Type.Optional(Type.Number({ minimum: 1 }))
        }),
        async execute(_toolCallId, params) {
            const values = params;
            const workspaceDir = resolveWorkspace(api, ctx);
            await ensureWorkspace(workspaceDir);
            const tasks = await listMiscTasks(workspaceDir, readOptionalNumber(values, "limit") ?? 10);
            return textResult(tasks.length
                ? `Misc queue:\n${tasks.map((task, index) => `${index + 1}. ${task}`).join("\n")}`
                : "Misc queue is empty.");
        }
    }), { name: "list_misc_queue" });
    api.registerTool((ctx) => ({
        name: "pop_misc_task",
        label: "Pop Misc Task",
        description: "Remove and return one or more useful small tasks from the misc queue.",
        parameters: Type.Object({
            count: Type.Optional(Type.Number({ minimum: 1 }))
        }),
        async execute(_toolCallId, params) {
            const values = params;
            const workspaceDir = resolveWorkspace(api, ctx);
            await ensureWorkspace(workspaceDir);
            const popped = await popMiscTasks(workspaceDir, readOptionalNumber(values, "count") ?? 1);
            return textResult(popped.length
                ? `Pulled from misc queue:\n${popped.map((task, index) => `${index + 1}. ${task}`).join("\n")}`
                : "Misc queue is empty.");
        }
    }), { name: "pop_misc_task" });
    api.registerTool((ctx) => ({
        name: "log_behavior_note",
        label: "Log Behavior Note",
        description: "Append a distilled behavioral pattern, drift tendency, trigger, or accountability tactic to behavior.md.",
        parameters: Type.Object({
            category: Type.String({ minLength: 1 }),
            note: Type.String({ minLength: 1 }),
            evidence: Type.Optional(Type.String())
        }),
        async execute(_toolCallId, params) {
            const values = params;
            const workspaceDir = resolveWorkspace(api, ctx);
            await ensureWorkspace(workspaceDir);
            const category = readString(values, "category");
            const note = readString(values, "note");
            const evidence = readOptionalString(values.evidence);
            await appendBehaviorEntry(workspaceDir, `\n## ${today()} ${category}\n\n- Pattern: ${note}\n${evidence ? `- Evidence: ${evidence}\n` : ""}`);
            await appendEvent(workspaceDir, {
                type: "behavior_note_logged",
                details: { category, note, evidence }
            });
            return textResult(`Behavior note logged under ${category}. Useful. That is memory, not vibes.`);
        }
    }), { name: "log_behavior_note" });
    api.registerTool((ctx) => ({
        name: "run_nightly_rollover",
        label: "Run Nightly Rollover",
        description: "Clear completed tasks, carry unfinished tasks forward, and append optional new tasks to tasks.md.",
        parameters: Type.Object({
            new_tasks: Type.Optional(Type.Array(Type.String())),
            summary: Type.Optional(Type.String())
        }),
        async execute(_toolCallId, params) {
            const values = params;
            const workspaceDir = resolveWorkspace(api, ctx);
            await ensureWorkspace(workspaceDir);
            const result = await rolloverTasks({
                workspaceDir,
                newTasks: readOptionalStringArray(values, "new_tasks"),
                summary: readOptionalString(values.summary)
            });
            const state = await readState(workspaceDir);
            await writeState(workspaceDir, {
                ...state,
                lastRolloverDate: result.date
            });
            return textResult(`Nightly rollover done. Carried ${result.carried.length}, cleared ${result.completed.length}, added ${result.added.length}. Fine. Tomorrow has a spine now.`);
        }
    }), { name: "run_nightly_rollover" });
    api.registerTool((ctx) => ({
        name: "write_nightly_summary",
        label: "Write Nightly Summary",
        description: "Append a compact nightly work and behavior summary without rewriting long memory files.",
        parameters: Type.Object({
            summary: Type.String({ minLength: 1 }),
            wins: Type.Optional(Type.Array(Type.String())),
            failures: Type.Optional(Type.Array(Type.String())),
            behavior_notes: Type.Optional(Type.Array(Type.String()))
        }),
        async execute(_toolCallId, params) {
            const values = params;
            const workspaceDir = resolveWorkspace(api, ctx);
            await ensureWorkspace(workspaceDir);
            const wins = readOptionalStringArray(values, "wins") ?? [];
            const failures = readOptionalStringArray(values, "failures") ?? [];
            const behaviorNotes = readOptionalStringArray(values, "behavior_notes") ?? [];
            const day = today();
            await appendWorkEntry(workspaceDir, [
                `\n## ${day} Nightly Summary`,
                "",
                `- Summary: ${readString(values, "summary")}`,
                wins.length ? "\n### Wins" : undefined,
                ...wins.map((item) => `- ${item}`),
                failures.length ? "\n### Failures" : undefined,
                ...failures.map((item) => `- ${item}`)
            ].filter(Boolean).join("\n") + "\n");
            if (behaviorNotes.length > 0) {
                await appendBehaviorEntry(workspaceDir, [
                    `\n## ${day} Nightly Behavioral Extraction`,
                    "",
                    ...behaviorNotes.map((item) => `- ${item}`)
                ].join("\n") + "\n");
            }
            await appendEvent(workspaceDir, {
                type: "nightly_summary_written",
                details: { wins: wins.length, failures: failures.length, behaviorNotes: behaviorNotes.length }
            });
            const state = await readState(workspaceDir);
            await writeState(workspaceDir, {
                ...state,
                lastNightlySummaryDate: day
            });
            return textResult("Nightly summary written. Clean enough. Sleep if the day is closed.");
        }
    }), { name: "write_nightly_summary" });
    api.registerTool((ctx) => ({
        name: "trigger_loud_alarm",
        label: "Trigger Loud Alarm",
        description: "Trigger the configured local loud alarm command or log a fallback urgent reminder.",
        parameters: Type.Object({}),
        async execute() {
            const workspaceDir = resolveWorkspace(api, ctx);
            await ensureWorkspace(workspaceDir);
            const result = await triggerAlarmCommand(resolveRuntimeConfig(api));
            const stats = await readStats(workspaceDir);
            const day = today();
            stats.loudAlarmsTriggered[day] = (stats.loudAlarmsTriggered[day] ?? 0) + 1;
            await writeStats(workspaceDir, stats);
            await appendEvent(workspaceDir, {
                type: "loud_alarm",
                details: result
            });
            await clearMatchingTriggers({
                workspaceDir,
                config: resolveRuntimeConfig(api),
                kinds: ["sleep_loud_alarm"],
                reason: "loud alarm fired"
            });
            return textResult(result.ok
                ? `${result.message} I gave you the buffer. Loud alarm now.`
                : `${result.message}\nI gave you the buffer. Loud alarm fallback now. Three hours silent is not a plan.`);
        }
    }), { name: "trigger_loud_alarm" });
    api.registerTool((ctx) => ({
        name: "trigger_normal_alarm",
        label: "Trigger Normal Alarm",
        description: "Trigger the configured normal wake alarm command or log a wake-up fallback.",
        parameters: Type.Object({}),
        async execute() {
            const workspaceDir = resolveWorkspace(api, ctx);
            await ensureWorkspace(workspaceDir);
            const result = await triggerNormalAlarmCommand(resolveRuntimeConfig(api));
            const stats = await readStats(workspaceDir);
            const day = today();
            stats.normalAlarmsTriggered[day] = (stats.normalAlarmsTriggered[day] ?? 0) + 1;
            await writeStats(workspaceDir, stats);
            await appendEvent(workspaceDir, {
                type: "normal_alarm",
                details: result
            });
            await clearMatchingTriggers({
                workspaceDir,
                config: resolveRuntimeConfig(api),
                kinds: ["sleep_normal_alarm"],
                reason: "normal alarm fired"
            });
            return textResult(result.ok
                ? `${result.message} I gave you the buffer. Wake up and say good morning.`
                : `${result.message}\nI gave you the buffer. Wake check now. Say good morning if you are up.`);
        }
    }), { name: "trigger_normal_alarm" });
    api.registerTool((ctx) => ({
        name: "get_linear_plan",
        label: "Get Linear Plan",
        description: "Read tasks.md and return the linear task slice that fits the remaining hours.",
        parameters: Type.Object({
            remaining_hours: Type.Number({ minimum: 0 })
        }),
        async execute(_toolCallId, params) {
            const values = params;
            const workspaceDir = resolveWorkspace(api, ctx);
            const remainingHours = readNumber(values, "remaining_hours");
            await ensureWorkspace(workspaceDir);
            const plan = await getLinearPlan(workspaceDir, remainingHours);
            const state = await readState(workspaceDir);
            await writeState(workspaceDir, { ...state, lastPlanRequestedAt: nowIso() });
            await appendEvent(workspaceDir, {
                type: "linear_plan_requested",
                details: { remainingHours, selected: plan.tasks.length, totalHours: plan.totalHours }
            });
            const lines = plan.tasks.map((task, index) => `${index + 1}. ${task.hours}h - ${task.title}`);
            return textResult(lines.length
                ? `Plan slice (${plan.totalHours}h of ${remainingHours}h):\n${lines.join("\n")}`
                : "No open tasks fit this window. Either tasks.md is empty or the next task is larger than the budget.");
        }
    }), { name: "get_linear_plan" });
    api.registerTool((ctx) => ({
        name: "log_strategy_result",
        label: "Log Strategy Result",
        description: "Record whether a strategy worked for the current Antirot day.",
        parameters: Type.Object({
            strategy_id: Type.String({ minLength: 1 }),
            status_binary: Type.Boolean()
        }),
        async execute(_toolCallId, params) {
            const values = params;
            const workspaceDir = resolveWorkspace(api, ctx);
            const strategyId = readString(values, "strategy_id");
            const statusBinary = readBoolean(values, "status_binary");
            await ensureWorkspace(workspaceDir);
            const performance = await readStrategyPerformance(workspaceDir);
            const record = performance.strategies[strategyId] ?? { attempts: [] };
            record.attempts.push({ at: nowIso(), status: statusBinary });
            performance.strategies[strategyId] = record;
            await writeStrategyPerformance(workspaceDir, performance);
            await appendEvent(workspaceDir, {
                type: "strategy_result",
                details: { strategyId, statusBinary }
            });
            return textResult(`Strategy ${strategyId} recorded as ${statusBinary ? "worked" : "failed"}.`);
        }
    }), { name: "log_strategy_result" });
    api.registerTool((ctx) => ({
        name: "toggle_vacation_mode",
        label: "Toggle Vacation Mode",
        description: "Enable or disable vacation mode and suppress Antirot pressure loops.",
        parameters: Type.Object({
            status_binary: Type.Boolean()
        }),
        async execute(_toolCallId, params) {
            const values = params;
            const workspaceDir = resolveWorkspace(api, ctx);
            const enabled = readBoolean(values, "status_binary");
            await toggleVacation(workspaceDir, enabled);
            return textResult(enabled
                ? "Vacation mode enabled. No pressure loops, no penalties."
                : "Vacation mode disabled. The system is awake again.");
        }
    }), { name: "toggle_vacation_mode" });
    api.registerTool((ctx) => ({
        name: "log_override",
        label: "Log Override",
        description: "Log an override without requiring a reason.",
        parameters: Type.Object({}),
        async execute() {
            const workspaceDir = resolveWorkspace(api, ctx);
            const count = await logOverride(workspaceDir);
            return textResult(`Override logged. Count today: ${count}.`);
        }
    }), { name: "log_override" });
    api.registerTool((ctx) => ({
        name: "request_protected_edit",
        label: "Request Protected Edit",
        description: "Record a short-lived approved intent before editing Antirot protected files.",
        parameters: Type.Object({
            file: Type.String({ minLength: 1 }),
            requested_change: Type.String({ minLength: 1 }),
            explanation: Type.String({ minLength: 1 })
        }),
        async execute(_toolCallId, params) {
            const values = params;
            const workspaceDir = resolveWorkspace(api, ctx);
            const file = normalizeWorkspaceRelativePath(readString(values, "file"));
            const requestedChange = readString(values, "requested_change");
            const explanation = readString(values, "explanation");
            await ensureWorkspace(workspaceDir);
            if (!isProtectedPath(file, workspaceDir)) {
                throw new Error(`${file} is not an Antirot protected file.`);
            }
            const intent = await addProtectedIntent(workspaceDir, {
                file,
                requestedChange,
                explanation
            });
            return textResult(`Protected edit intent approved for ${file} until ${intent.expiresAt}. Make the edit cleanly, then stop.`);
        }
    }), { name: "request_protected_edit" });
}
function registerHooks(api) {
    api.on("before_prompt_build", async (event) => {
        const workspaceDir = eventWorkspace(api);
        await ensureWorkspace(workspaceDir);
        let wakeNote = "";
        const state = await readState(workspaceDir);
        if (state.mode === "sleeping" && isGoodMorningVariant(event.prompt)) {
            const wake = await completeSleep({ workspaceDir });
            await writeState(workspaceDir, {
                ...state,
                mode: state.vacation ? "vacation" : "idle",
                activeBlock: undefined
            });
            await clearMatchingTriggers({
                workspaceDir,
                config: resolveRuntimeConfig(api),
                kinds: ["sleep_normal_alarm", "sleep_loud_alarm"],
                reason: "wake confirmed by good morning variant"
            });
            wakeNote = `Antirot auto-logged wake from good morning variant: ${wake.message}\n`;
        }
        return {
            prependSystemContext: buildPersonaContext(),
            appendContext: `${wakeNote}${await buildStateContext(workspaceDir, resolveRuntimeConfig(api))}`
        };
    });
    api.on("before_tool_call", async (event) => {
        if (!protectedToolNames().has(event.toolName)) {
            return undefined;
        }
        const workspaceDir = eventWorkspace(api);
        await ensureWorkspace(workspaceDir);
        const candidates = collectPathCandidates(event.toolName, event.params, event.derivedPaths);
        for (const candidate of candidates) {
            if (!isProtectedPath(candidate, workspaceDir)) {
                continue;
            }
            const relative = normalizeWorkspaceRelativePath(path.relative(workspaceDir, path.resolve(workspaceDir, candidate)));
            if (await hasFreshProtectedIntent(workspaceDir, relative)) {
                return undefined;
            }
            return {
                block: true,
                blockReason: `Antirot blocked direct edit to ${relative}. Ask the user why this protected change matters, then call request_protected_edit first. /override bypasses objections but still logs the choice.`
            };
        }
        return undefined;
    }, { priority: 90_000 });
}
export default definePluginEntry({
    id: "antirot",
    name: "Antirot",
    description: "Strict coach accountability plugin with deterministic timers, state, metrics, and protected memory.",
    configSchema: {
        jsonSchema: {
            type: "object",
            additionalProperties: false,
            properties: {
                workspaceDir: { type: "string", minLength: 1 },
                openclawCommand: { type: "string", minLength: 1, default: "openclaw" },
                normalAlarmCommand: { type: "string", minLength: 1 },
                alarmCommand: { type: "string", minLength: 1 },
                enableCron: { type: "boolean", default: true }
            }
        }
    },
    register(api) {
        registerCommands(api);
        registerTools(api);
        registerHooks(api);
    }
});
