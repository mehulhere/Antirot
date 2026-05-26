import type { ActiveSleepSession, SleepEntry, SleepStats } from "./types.js";
export type SleepRequirement = {
    requiredHours: number;
    debtHours: number;
    baseRequirementHours: number;
    tirednessBonusHours: number;
    debtBonusHours: number;
};
export declare function clamp(value: number, min: number, max: number): number;
export declare function roundToQuarterHour(value: number): number;
export declare function dateAfterHours(date: Date, hours: number): Date;
export declare function calculateSleepRequirement(stats: SleepStats, tirednessLevel?: number, plannedSleepHours?: number): SleepRequirement;
export declare function beginSleep(params: {
    workspaceDir: string;
    tirednessLevel?: number;
    plannedSleepHours?: number;
    sleepStartedAt?: string;
}): Promise<{
    stats: SleepStats;
    requirement: SleepRequirement;
    session: ActiveSleepSession;
}>;
export declare function completeSleep(params: {
    workspaceDir: string;
    wokeAt?: string;
    stillTired?: boolean;
    sleepQuality?: number;
    notes?: string;
}): Promise<{
    entry?: SleepEntry;
    stats: SleepStats;
    message: string;
}>;
export declare function getSleepSummary(workspaceDir: string, tirednessLevel?: number): Promise<string>;
export declare function isGoodMorningVariant(text: string): boolean;
export declare function currentIso(): string;
