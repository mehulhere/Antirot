import {
    appendEvent,
    appendSleepEntry,
    nowIso,
    readSleepStats,
    todayKey,
    writeSleepStats
} from "./storage.js";
import type { ActiveSleepSession, SleepEntry, SleepStats } from "./types.js";

const sleepDebtLookbackDays = 5;
const maxDebtAppliedToRequirementHours = 4;
const maxSleepRequirementHours = 11;
const minSleepRequirementHours = 6;

export type SleepRequirement = {
    requiredHours: number;
    debtHours: number;
    baseRequirementHours: number;
    tirednessBonusHours: number;
    debtBonusHours: number;
};

export function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
}

export function roundToQuarterHour(value: number): number {
    return Math.round(value * 4) / 4;
}

export function dateAfterHours(date: Date, hours: number): Date {
    return new Date(date.getTime() + hours * 60 * 60 * 1000);
}

function normalizeTiredness(value: number | undefined): number {
    return clamp(Number.isFinite(value ?? 0) ? value ?? 0 : 0, 0, 3);
}

function rollingDebt(entries: SleepEntry[], fallbackDebtHours: number): number {
    const recent = entries.slice(-sleepDebtLookbackDays);
    if (recent.length === 0) {
        return Math.max(0, fallbackDebtHours);
    }
    return roundToQuarterHour(recent.reduce((total, entry) => {
        return total + Math.max(0, entry.requiredHours - entry.durationHours);
    }, 0));
}

export function calculateSleepRequirement(stats: SleepStats, tirednessLevel?: number, plannedSleepHours?: number): SleepRequirement {
    const baseRequirementHours = stats.baseRequirementHours || 8;
    const debtHours = rollingDebt(stats.entries, stats.debtHours);
    const normalizedTiredness = normalizeTiredness(tirednessLevel);
    const tirednessBonusHours = normalizedTiredness * 0.5;
    const debtBonusHours = Math.min(debtHours, maxDebtAppliedToRequirementHours) * 0.5;
    const calculated = baseRequirementHours + tirednessBonusHours + debtBonusHours;
    const requiredHours = roundToQuarterHour(clamp(plannedSleepHours ?? calculated, minSleepRequirementHours, maxSleepRequirementHours));
    return {
        requiredHours,
        debtHours,
        baseRequirementHours,
        tirednessBonusHours,
        debtBonusHours
    };
}

export async function beginSleep(params: {
    workspaceDir: string;
    tirednessLevel?: number;
    plannedSleepHours?: number;
    sleepStartedAt?: string;
}): Promise<{
    stats: SleepStats;
    requirement: SleepRequirement;
    session: ActiveSleepSession;
}> {
    const stats = await readSleepStats(params.workspaceDir);
    const startedAt = params.sleepStartedAt ? new Date(params.sleepStartedAt) : new Date();
    const requirement = calculateSleepRequirement(stats, params.tirednessLevel, params.plannedSleepHours);
    const normalAlarmAt = dateAfterHours(startedAt, requirement.requiredHours + 0.5);
    const loudAlarmAt = new Date(normalAlarmAt.getTime() + 15 * 60 * 1000);
    const session: ActiveSleepSession = {
        sleepStartedAt: startedAt.toISOString(),
        requiredHours: requirement.requiredHours,
        debtBeforeHours: requirement.debtHours,
        tirednessLevel: normalizeTiredness(params.tirednessLevel),
        normalAlarmAt: normalAlarmAt.toISOString(),
        loudAlarmAt: loudAlarmAt.toISOString()
    };
    const nextStats = {
        ...stats,
        debtHours: requirement.debtHours,
        activeSleep: session
    };
    await writeSleepStats(params.workspaceDir, nextStats);
    await appendEvent(params.workspaceDir, {
        type: "sleep_started",
        details: session
    });
    await appendSleepEntry(
        params.workspaceDir,
        `\n## ${todayKey(startedAt)} Sleep Started\n\n- Started: ${session.sleepStartedAt}\n- Required: ${session.requiredHours}h\n- Debt before: ${session.debtBeforeHours}h\n- Tiredness level: ${session.tirednessLevel}/3\n- Wake alarm: after recovery with a hidden buffer\n- Escalation: loud alarm after an additional hidden buffer if wake is not confirmed\n`
    );
    return { stats: nextStats, requirement, session };
}

export async function completeSleep(params: {
    workspaceDir: string;
    wokeAt?: string;
    stillTired?: boolean;
    sleepQuality?: number;
    notes?: string;
}): Promise<{
    entry?: SleepEntry;
    stats: SleepStats;
    message: string;
}> {
    const stats = await readSleepStats(params.workspaceDir);
    if (!stats.activeSleep) {
        return {
            stats,
            message: "No active sleep session was found. Wake noted, but there was nothing to close."
        };
    }
    const wokeAt = params.wokeAt ? new Date(params.wokeAt) : new Date();
    const startedAt = new Date(stats.activeSleep.sleepStartedAt);
    const durationHours = roundToQuarterHour(Math.max(0, (wokeAt.getTime() - startedAt.getTime()) / 3_600_000));
    const rawDebtAfter = stats.activeSleep.debtBeforeHours + stats.activeSleep.requiredHours - durationHours;
    const debtAfterHours = roundToQuarterHour(Math.max(0, rawDebtAfter + (params.stillTired ? 0.5 : 0)));
    const entry: SleepEntry = {
        date: todayKey(wokeAt),
        sleepStartedAt: stats.activeSleep.sleepStartedAt,
        wokeAt: wokeAt.toISOString(),
        durationHours,
        requiredHours: stats.activeSleep.requiredHours,
        debtBeforeHours: stats.activeSleep.debtBeforeHours,
        debtAfterHours,
        tirednessLevel: stats.activeSleep.tirednessLevel,
        stillTired: params.stillTired,
        sleepQuality: params.sleepQuality,
        notes: params.notes
    };
    const nextStats = {
        ...stats,
        debtHours: debtAfterHours,
        activeSleep: undefined,
        entries: [...stats.entries, entry].slice(-30)
    };
    await writeSleepStats(params.workspaceDir, nextStats);
    await appendEvent(params.workspaceDir, {
        type: "sleep_completed",
        details: entry
    });
    await appendSleepEntry(
        params.workspaceDir,
        `\n## ${entry.date} Wake\n\n- Woke: ${entry.wokeAt}\n- Duration: ${entry.durationHours}h\n- Required: ${entry.requiredHours}h\n- Debt after: ${entry.debtAfterHours}h\n- Still tired: ${entry.stillTired ?? "unknown"}\n- Quality: ${entry.sleepQuality ?? "unknown"}\n- Notes: ${entry.notes ?? "none"}\n`
    );
    return {
        entry,
        stats: nextStats,
        message: `Wake logged. Slept ${durationHours}h against ${entry.requiredHours}h required. Current sleep debt: ${debtAfterHours}h.`
    };
}

export async function getSleepSummary(workspaceDir: string, tirednessLevel?: number): Promise<string> {
    const stats = await readSleepStats(workspaceDir);
    const requirement = calculateSleepRequirement(stats, tirednessLevel);
    const recent = stats.entries.slice(-sleepDebtLookbackDays);
    const recentLines = recent.length
        ? recent.map((entry) => `- ${entry.date}: ${entry.durationHours}h slept, ${entry.requiredHours}h required, debt after ${entry.debtAfterHours}h`)
        : ["- No completed sleep sessions yet."];
    return [
        `Sleep debt: ${requirement.debtHours}h`,
        `Recommended sleep now: ${requirement.requiredHours}h`,
        `Base: ${requirement.baseRequirementHours}h, debt bonus: ${requirement.debtBonusHours}h, tiredness bonus: ${requirement.tirednessBonusHours}h`,
        stats.activeSleep ? "Active sleep: yes, wake alarms use hidden buffers" : "Active sleep: none",
        "Recent sleep:",
        ...recentLines
    ].join("\n");
}

export function isGoodMorningVariant(text: string): boolean {
    return /\b(good\s*morning|gm|morning|i\s*(am|'m)\s*awake|i\s*woke\s*up|woke\s*up|i\s*am\s*up|i'm\s*up)\b/iu.test(text);
}

export function currentIso(): string {
    return nowIso();
}
