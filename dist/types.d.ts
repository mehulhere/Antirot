export type AntirotMode = "idle" | "working" | "routine" | "break" | "sleeping" | "vacation";
export type AntirotConfig = {
    workspaceDir?: string;
    openclawCommand?: string;
    normalAlarmCommand?: string;
    alarmCommand?: string;
    enableCron?: boolean;
    bestStrategiesCount?: number;
    randomStrategiesCount?: number;
};
export type ActiveBlock = {
    kind: "session" | "routine" | "timer" | "sleep";
    name: string;
    startedAt: string;
    durationMins: number;
    callbackReason?: string;
};
export type AntirotState = {
    version: 1;
    mode: AntirotMode;
    vacation: boolean;
    activeBlock?: ActiveBlock;
    currentStrategies: string[];
    lastStrategySelectionDate?: string;
    lastPlanRequestedAt?: string;
    lastPlanSubmittedAt?: string;
    lastRolloverDate?: string;
    lastNightlySummaryDate?: string;
    lastOnboardingPromptAt?: string;
    onboardingCompletedAt?: string;
    lastGoalReviewAt?: string;
    updatedAt: string;
};
export type BehavioralStats = {
    version: 1;
    overrides: Record<string, number>;
    productiveMins: Record<string, number>;
    onTableWastedMins: Record<string, number>;
    sessionsCompleted: Record<string, number>;
    vacationDays: Record<string, boolean>;
    loudAlarmsTriggered: Record<string, number>;
    normalAlarmsTriggered: Record<string, number>;
};
export type SleepEntry = {
    date: string;
    sleepStartedAt: string;
    wokeAt: string;
    durationHours: number;
    requiredHours: number;
    debtBeforeHours: number;
    debtAfterHours: number;
    tirednessLevel: number;
    stillTired?: boolean;
    sleepQuality?: number;
    notes?: string;
};
export type ActiveSleepSession = {
    sleepStartedAt: string;
    requiredHours: number;
    debtBeforeHours: number;
    tirednessLevel: number;
    normalAlarmAt: string;
    loudAlarmAt: string;
};
export type SleepStats = {
    version: 1;
    baseRequirementHours: number;
    debtHours: number;
    activeSleep?: ActiveSleepSession;
    entries: SleepEntry[];
    updatedAt: string;
};
export type StrategyAttempt = {
    at: string;
    status: boolean;
};
export type StrategyRecord = {
    attempts: StrategyAttempt[];
};
export type StrategyPerformance = {
    version: 1;
    strategies: Record<string, StrategyRecord>;
};
export type ProtectedEditIntent = {
    file: string;
    requestedChange: string;
    explanation: string;
    createdAt: string;
    expiresAt: string;
};
export type ProtectedEditIntents = {
    version: 1;
    intents: ProtectedEditIntent[];
};
export type AntirotEvent = {
    at: string;
    type: string;
    details: Record<string, unknown>;
};
export type AntirotTriggerKind = "routine" | "session" | "timer" | "alignment_check" | "sleep_normal_alarm" | "sleep_loud_alarm";
export type AntirotTriggerStatus = "active" | "cleared" | "rescheduled" | "fired";
export type AntirotTriggerScope = "daily" | "sleep";
export type AntirotTrigger = {
    id: string;
    kind: AntirotTriggerKind;
    scope: AntirotTriggerScope;
    label: string;
    reason: string;
    createdAt: string;
    requestedDelayMins: number;
    scheduledDelayMins?: number;
    jitterPercent?: number;
    cronJobId?: string;
    status: AntirotTriggerStatus;
    clearedAt?: string;
    clearReason?: string;
    supersededBy?: string;
};
export type AntirotTriggerRegistry = {
    version: 1;
    triggers: AntirotTrigger[];
    updatedAt: string;
};
export type LinearTask = {
    raw: string;
    title: string;
    hours: number;
    checked: boolean;
};
