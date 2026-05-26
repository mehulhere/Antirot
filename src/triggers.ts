import { randomUUID } from "node:crypto";
import { cancelCronReminder, scheduleCronReminder, type CronResult } from "./runtime.js";
import {
    appendEvent,
    nowIso,
    readTriggerRegistry,
    writeTriggerRegistry
} from "./storage.js";
import type {
    AntirotConfig,
    AntirotTrigger,
    AntirotTriggerKind,
    AntirotTriggerScope
} from "./types.js";

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

export async function createAntirotTrigger(params: CreateTriggerParams): Promise<{
    trigger: AntirotTrigger;
    cron: CronResult;
}> {
    const cron = await scheduleCronReminder({
        config: params.config,
        name: params.cronName,
        delayMins: params.delayMins,
        systemEvent: `${params.systemEvent}\nBefore acting, call list_active_triggers and ignore this callback if the matching Antirot trigger is no longer active.`
    });
    const trigger: AntirotTrigger = {
        id: randomUUID(),
        kind: params.kind,
        scope: params.scope,
        label: params.label,
        reason: params.reason,
        createdAt: nowIso(),
        requestedDelayMins: Math.max(1, Math.round(params.delayMins)),
        scheduledDelayMins: cron.scheduledDelayMins,
        jitterPercent: cron.jitterPercent,
        cronJobId: cron.cronJobId,
        status: "active"
    };
    const registry = await readTriggerRegistry(params.workspaceDir);
    registry.triggers = [...registry.triggers, trigger].slice(-200);
    await writeTriggerRegistry(params.workspaceDir, registry);
    await appendEvent(params.workspaceDir, {
        type: "trigger_created",
        details: { trigger, cron }
    });
    return { trigger, cron };
}

export async function listActiveTriggers(workspaceDir: string): Promise<AntirotTrigger[]> {
    const registry = await readTriggerRegistry(workspaceDir);
    return registry.triggers.filter((trigger) => trigger.status === "active");
}

export async function clearTrigger(params: {
    workspaceDir: string;
    config: AntirotConfig;
    triggerId: string;
    reason: string;
}): Promise<{
    trigger?: AntirotTrigger;
    cron: CronResult;
}> {
    const registry = await readTriggerRegistry(params.workspaceDir);
    const trigger = registry.triggers.find((candidate) => candidate.id === params.triggerId);
    if (!trigger) {
        return {
            cron: {
                ok: false,
                message: "🔴 FALLBACK: trigger clear skipped - Reason: Antirot trigger id was not found - Impact: no registry change was made"
            }
        };
    }
    trigger.status = "cleared";
    trigger.clearedAt = nowIso();
    trigger.clearReason = params.reason;
    await writeTriggerRegistry(params.workspaceDir, registry);
    const cron = await cancelCronReminder({ config: params.config, cronJobId: trigger.cronJobId });
    await appendEvent(params.workspaceDir, {
        type: "trigger_cleared",
        details: { trigger, cron }
    });
    return { trigger, cron };
}

export async function clearMatchingTriggers(params: {
    workspaceDir: string;
    config: AntirotConfig;
    kinds: AntirotTriggerKind[];
    label?: string;
    reason: string;
}): Promise<Array<{ trigger: AntirotTrigger; cron: CronResult }>> {
    const active = await listActiveTriggers(params.workspaceDir);
    const matched = active.filter((trigger) => {
        const kindMatches = params.kinds.includes(trigger.kind);
        const labelMatches = params.label ? trigger.label === params.label : true;
        return kindMatches && labelMatches;
    });
    const cleared: Array<{ trigger: AntirotTrigger; cron: CronResult }> = [];
    for (const trigger of matched) {
        const result = await clearTrigger({
            workspaceDir: params.workspaceDir,
            config: params.config,
            triggerId: trigger.id,
            reason: params.reason
        });
        if (result.trigger) {
            cleared.push({ trigger: result.trigger, cron: result.cron });
        }
    }
    return cleared;
}

export async function rescheduleTrigger(params: {
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
}> {
    const registry = await readTriggerRegistry(params.workspaceDir);
    const oldTrigger = registry.triggers.find((candidate) => candidate.id === params.triggerId);
    if (!oldTrigger) {
        return {
            clearCron: {
                ok: false,
                message: "🔴 FALLBACK: trigger reschedule skipped - Reason: Antirot trigger id was not found - Impact: no registry change was made"
            }
        };
    }
    const clearResult = await clearTrigger({
        workspaceDir: params.workspaceDir,
        config: params.config,
        triggerId: oldTrigger.id,
        reason: `rescheduled: ${params.reason}`
    });
    const created = await createAntirotTrigger({
        workspaceDir: params.workspaceDir,
        config: params.config,
        kind: oldTrigger.kind,
        scope: oldTrigger.scope,
        label: oldTrigger.label,
        reason: params.reason,
        delayMins: params.delayMins,
        cronName: `antirot-rescheduled-${oldTrigger.kind}-${oldTrigger.label}`,
        systemEvent: `Antirot rescheduled trigger fired: ${oldTrigger.kind} ${oldTrigger.label}. Reason: ${params.reason}.`
    });
    const updatedRegistry = await readTriggerRegistry(params.workspaceDir);
    const clearedOld = updatedRegistry.triggers.find((candidate) => candidate.id === oldTrigger.id);
    if (clearedOld) {
        clearedOld.status = "rescheduled";
        clearedOld.supersededBy = created.trigger.id;
        await writeTriggerRegistry(params.workspaceDir, updatedRegistry);
    }
    await appendEvent(params.workspaceDir, {
        type: "trigger_rescheduled",
        details: { oldTriggerId: oldTrigger.id, newTriggerId: created.trigger.id, reason: params.reason }
    });
    return {
        oldTrigger: clearResult.trigger,
        newTrigger: created.trigger,
        clearCron: clearResult.cron,
        scheduleCron: created.cron
    };
}

export function formatActiveTriggersForModel(triggers: AntirotTrigger[]): string {
    if (triggers.length === 0) {
        return "No active Antirot daily triggers.";
    }
    return triggers.map((trigger, index) => {
        return [
            `${index + 1}. id=${trigger.id}`,
            `kind=${trigger.kind}`,
            `scope=${trigger.scope}`,
            `label=${trigger.label}`,
            `reason=${trigger.reason}`,
            "time=hidden-buffered"
        ].join(" | ");
    }).join("\n");
}
