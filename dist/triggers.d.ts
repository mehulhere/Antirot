import { type CronResult } from "./runtime.js";
import type { AntirotConfig, AntirotTrigger, AntirotTriggerKind, AntirotTriggerScope } from "./types.js";
export type CreateTriggerParams = {
    workspaceDir: string;
    config: AntirotConfig;
    kind: AntirotTriggerKind;
    scope: AntirotTriggerScope;
    label: string;
    reason: string;
    delayMins: number;
    cronName: string;
    systemEvent: string;
};
export declare function createAntirotTrigger(params: CreateTriggerParams): Promise<{
    trigger: AntirotTrigger;
    cron: CronResult;
}>;
export declare function listActiveTriggers(workspaceDir: string): Promise<AntirotTrigger[]>;
export declare function clearTrigger(params: {
    workspaceDir: string;
    config: AntirotConfig;
    triggerId: string;
    reason: string;
}): Promise<{
    trigger?: AntirotTrigger;
    cron: CronResult;
}>;
export declare function clearMatchingTriggers(params: {
    workspaceDir: string;
    config: AntirotConfig;
    kinds: AntirotTriggerKind[];
    label?: string;
    reason: string;
}): Promise<Array<{
    trigger: AntirotTrigger;
    cron: CronResult;
}>>;
export declare function rescheduleTrigger(params: {
    workspaceDir: string;
    config: AntirotConfig;
    triggerId: string;
    delayMins: number;
    reason: string;
}): Promise<{
    oldTrigger?: AntirotTrigger;
    newTrigger?: AntirotTrigger;
    clearCron: CronResult;
    scheduleCron?: CronResult;
}>;
export declare function formatActiveTriggersForModel(triggers: AntirotTrigger[]): string;
