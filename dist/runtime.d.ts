import type { AntirotConfig } from "./types.js";
export type CronResult = {
    ok: boolean;
    message: string;
    cronJobId?: string;
    requestedDelayMins?: number;
    scheduledDelayMins?: number;
    jitterPercent?: number;
};
export declare function scheduleCronReminder(params: {
    config: AntirotConfig;
    name: string;
    delayMins: number;
    systemEvent: string;
}): Promise<CronResult>;
export declare function cancelCronReminder(params: {
    config: AntirotConfig;
    cronJobId?: string;
}): Promise<CronResult>;
export declare function triggerAlarmCommand(config: AntirotConfig): Promise<CronResult>;
export declare function triggerNormalAlarmCommand(config: AntirotConfig): Promise<CronResult>;
