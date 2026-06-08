import { appendFile, mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import type {
    AntirotConfig,
    AntirotEvent,
    AntirotState,
    AntirotTriggerRegistry,
    BehavioralStats,
    ProtectedEditIntent,
    ProtectedEditIntents,
    SleepStats,
    StrategyPerformance
} from "./types.js";

const protectedMarkdownFiles = [
    "longterm.md",
    "shortterm.md",
    "behavior.md",
    "sleep.md",
    "tasks.md",
    "achievements.md",
    "miscellaneous_todo.md"
] as const;

const defaultMarkdown: Record<(typeof protectedMarkdownFiles)[number], string> = {
    "longterm.md": "# Long-Term Goals\n\n## Direction\n- Onboarding will ask what the user is trying to build or become, then Antirot will summarize durable goals here.\n\n## Standards\n- High standards, honest recovery, no fake praise.\n",
    "shortterm.md": "# Short-Term State\n\n## Current Priorities\n- Onboarding will ask what the user is working on now, then Antirot will summarize near-term priorities here.\n\n## Constraints\n- Sleep, travel, health, relationship time, and vacation mode belong here.\n",
    "behavior.md": "# Behavior Memory\n\n## Recurring Patterns\n- Onboarding will ask what helps or derails the user, then Antirot will summarize stable patterns here.\n\n## Drift Tendencies\n- Known drift loops go here.\n\n## Accountability Styles\n- Tactics that work or fail go here.\n",
    "sleep.md": "# Sleep Ledger\n\n## Rules\n- Planning tomorrow and going to sleep are different states.\n- Sleep recovery is calculated from recent sleep debt and tiredness.\n\n",
    "tasks.md": "# Task Pipeline\n\n[ ] 1.0h - Define the first serious task\n",
    "achievements.md": "# Achievements\n\n- Baseline established.\n",
    "miscellaneous_todo.md": "# Miscellaneous Todo\n\n- Drink water\n- Clear one tiny admin task\n"
};

export const protectedFileNames = new Set<string>([
    ...protectedMarkdownFiles,
    "personality.md",
    "behavior.md",
    ".antirot/state.json",
    ".antirot/behavioral_stats.json",
    ".antirot/sleep_stats.json",
    ".antirot/triggers.json",
    ".antirot/strategy_performance.json",
    ".antirot/protected_edit_intents.json"
]);

export function todayKey(date = new Date()): string {
    return date.toISOString().slice(0, 10);
}

export function nowIso(): string {
    return new Date().toISOString();
}

function readString(value: unknown): string | undefined {
    return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

export function resolveWorkspaceDir(params: {
    config?: AntirotConfig;
    workspaceDir?: string;
    openClawConfig?: unknown;
}): string {
    const configured = readString(params.config?.workspaceDir);
    if (configured) {
        return path.resolve(configured);
    }
    const runtimeWorkspace = readString(params.workspaceDir);
    if (runtimeWorkspace) {
        return path.resolve(runtimeWorkspace);
    }
    const envWorkspace = readString(process.env.ANTIROT_WORKSPACE_DIR);
    if (envWorkspace) {
        return path.resolve(envWorkspace);
    }
    const configWorkspace = readWorkspaceFromOpenClawConfig(params.openClawConfig);
    if (configWorkspace) {
        return path.resolve(configWorkspace);
    }
    return process.cwd();
}

function readWorkspaceFromOpenClawConfig(config: unknown): string | undefined {
    if (!config || typeof config !== "object") {
        return undefined;
    }
    const agents = (config as { agents?: unknown }).agents;
    if (!agents || typeof agents !== "object") {
        return undefined;
    }
    const defaults = (agents as { defaults?: unknown }).defaults;
    if (defaults && typeof defaults === "object") {
        const workspace = readString((defaults as { workspace?: unknown }).workspace);
        if (workspace) {
            return workspace;
        }
    }
    return undefined;
}

export function antirotDir(workspaceDir: string): string {
    return path.join(workspaceDir, ".antirot");
}

export function statePath(workspaceDir: string): string {
    return path.join(antirotDir(workspaceDir), "state.json");
}

export function statsPath(workspaceDir: string): string {
    return path.join(antirotDir(workspaceDir), "behavioral_stats.json");
}

export function sleepStatsPath(workspaceDir: string): string {
    return path.join(antirotDir(workspaceDir), "sleep_stats.json");
}

export function strategyPath(workspaceDir: string): string {
    return path.join(antirotDir(workspaceDir), "strategy_performance.json");
}

export function eventsPath(workspaceDir: string): string {
    return path.join(antirotDir(workspaceDir), "events.jsonl");
}

export function triggersPath(workspaceDir: string): string {
    return path.join(antirotDir(workspaceDir), "triggers.json");
}

export function protectedIntentsPath(workspaceDir: string): string {
    return path.join(antirotDir(workspaceDir), "protected_edit_intents.json");
}

export async function ensureWorkspace(workspaceDir: string): Promise<void> {
    await mkdir(antirotDir(workspaceDir), { recursive: true });
    for (const file of protectedMarkdownFiles) {
        const target = path.join(workspaceDir, file);
        try {
            await readFile(target, "utf8");
        } catch {
            await writeFile(target, defaultMarkdown[file], "utf8");
        }
    }
    await readState(workspaceDir);
    await readStats(workspaceDir);
    await readSleepStats(workspaceDir);
    await readTriggerRegistry(workspaceDir);
    await readStrategyPerformance(workspaceDir);
    await readProtectedIntents(workspaceDir);
}

async function readJsonFile<T>(filePath: string, fallback: T): Promise<T> {
    try {
        const text = await readFile(filePath, "utf8");
        return JSON.parse(text) as T;
    } catch {
        await mkdir(path.dirname(filePath), { recursive: true });
        await writeJsonFile(filePath, fallback);
        return fallback;
    }
}

async function writeJsonFile(filePath: string, value: unknown): Promise<void> {
    await mkdir(path.dirname(filePath), { recursive: true });
    await writeFile(filePath, `${JSON.stringify(value, null, 4)}\n`, "utf8");
}

export async function readState(workspaceDir: string): Promise<AntirotState> {
    return await readJsonFile<AntirotState>(statePath(workspaceDir), {
        version: 1,
        mode: "idle",
        vacation: false,
        currentStrategies: [],
        updatedAt: nowIso()
    });
}

export async function writeState(workspaceDir: string, state: AntirotState): Promise<void> {
    await writeJsonFile(statePath(workspaceDir), {
        ...state,
        updatedAt: nowIso()
    });
}

export async function readStats(workspaceDir: string): Promise<BehavioralStats> {
    const stats = await readJsonFile<BehavioralStats>(statsPath(workspaceDir), {
        version: 1,
        overrides: {},
        productiveMins: {},
        onTableWastedMins: {},
        sessionsCompleted: {},
        vacationDays: {},
        loudAlarmsTriggered: {},
        normalAlarmsTriggered: {}
    });
    return {
        version: 1,
        overrides: stats.overrides ?? {},
        productiveMins: stats.productiveMins ?? {},
        onTableWastedMins: stats.onTableWastedMins ?? {},
        sessionsCompleted: stats.sessionsCompleted ?? {},
        vacationDays: stats.vacationDays ?? {},
        loudAlarmsTriggered: stats.loudAlarmsTriggered ?? {},
        normalAlarmsTriggered: stats.normalAlarmsTriggered ?? {}
    };
}

export async function writeStats(workspaceDir: string, stats: BehavioralStats): Promise<void> {
    await writeJsonFile(statsPath(workspaceDir), stats);
}

export async function readSleepStats(workspaceDir: string): Promise<SleepStats> {
    const stats = await readJsonFile<SleepStats>(sleepStatsPath(workspaceDir), {
        version: 1,
        baseRequirementHours: 8,
        debtHours: 0,
        entries: [],
        updatedAt: nowIso()
    });
    return {
        version: 1,
        baseRequirementHours: stats.baseRequirementHours || 8,
        debtHours: stats.debtHours ?? 0,
        activeSleep: stats.activeSleep,
        entries: stats.entries ?? [],
        updatedAt: stats.updatedAt ?? nowIso()
    };
}

export async function writeSleepStats(workspaceDir: string, stats: SleepStats): Promise<void> {
    await writeJsonFile(sleepStatsPath(workspaceDir), {
        ...stats,
        updatedAt: nowIso()
    });
}

export async function readTriggerRegistry(workspaceDir: string): Promise<AntirotTriggerRegistry> {
    const registry = await readJsonFile<AntirotTriggerRegistry>(triggersPath(workspaceDir), {
        version: 1,
        triggers: [],
        updatedAt: nowIso()
    });
    return {
        version: 1,
        triggers: registry.triggers ?? [],
        updatedAt: registry.updatedAt ?? nowIso()
    };
}

export async function writeTriggerRegistry(
    workspaceDir: string,
    registry: AntirotTriggerRegistry
): Promise<void> {
    await writeJsonFile(triggersPath(workspaceDir), {
        ...registry,
        updatedAt: nowIso()
    });
}

export async function readStrategyPerformance(workspaceDir: string): Promise<StrategyPerformance> {
    return await readJsonFile<StrategyPerformance>(strategyPath(workspaceDir), {
        version: 1,
        strategies: {}
    });
}

export async function writeStrategyPerformance(
    workspaceDir: string,
    performance: StrategyPerformance
): Promise<void> {
    await writeJsonFile(strategyPath(workspaceDir), performance);
}

export async function readProtectedIntents(workspaceDir: string): Promise<ProtectedEditIntents> {
    return await readJsonFile<ProtectedEditIntents>(protectedIntentsPath(workspaceDir), {
        version: 1,
        intents: []
    });
}

export async function writeProtectedIntents(
    workspaceDir: string,
    intents: ProtectedEditIntents
): Promise<void> {
    await writeJsonFile(protectedIntentsPath(workspaceDir), intents);
}

export async function appendEvent(workspaceDir: string, event: Omit<AntirotEvent, "at">): Promise<void> {
    await mkdir(antirotDir(workspaceDir), { recursive: true });
    await appendFile(
        eventsPath(workspaceDir),
        `${JSON.stringify({ at: nowIso(), ...event } satisfies AntirotEvent)}\n`,
        "utf8"
    );
}

export function getDailyWorkLogName(date = new Date()): string {
    return `${todayKey(date)}_WorkLog.md`;
}

export function getDailySummaryName(date = new Date()): string {
    return `${todayKey(date)}_Summary.md`;
}

export function getWeeklyOverrideLogName(date = new Date()): string {
    const d = new Date(Date.UTC(date.getFullYear(), date.getMonth(), date.getDate()));
    const dayNum = d.getUTCDay() || 7;
    d.setUTCDate(d.getUTCDate() + 4 - dayNum);
    const yearStart = new Date(Date.UTC(d.getUTCFullYear(), 0, 1));
    const weekNo = Math.ceil((((d.getTime() - yearStart.getTime()) / 86400000) + 1) / 7);
    const paddedWeek = String(weekNo).padStart(2, "0");
    return `${d.getUTCFullYear()}_W${paddedWeek}_Override.md`;
}

export async function appendWorkEntry(workspaceDir: string, markdown: string): Promise<void> {
    const workPath = path.join(workspaceDir, getDailyWorkLogName());
    await appendFile(workPath, markdown, "utf8");
}

export async function appendWeeklyOverrideEntry(workspaceDir: string, markdown: string): Promise<void> {
    const fullPath = path.join(workspaceDir, getWeeklyOverrideLogName());
    await appendFile(fullPath, markdown, "utf8");
}

export async function appendBehaviorEntry(workspaceDir: string, markdown: string): Promise<void> {
    const behaviorPath = path.join(workspaceDir, "behavior.md");
    await appendFile(behaviorPath, markdown, "utf8");
}

export async function appendLongtermEntry(workspaceDir: string, markdown: string): Promise<void> {
    const longtermPath = path.join(workspaceDir, "longterm.md");
    await appendFile(longtermPath, markdown, "utf8");
}

export async function appendShorttermEntry(workspaceDir: string, markdown: string): Promise<void> {
    const shorttermPath = path.join(workspaceDir, "shortterm.md");
    await appendFile(shorttermPath, markdown, "utf8");
}

export async function appendSleepEntry(workspaceDir: string, markdown: string): Promise<void> {
    const sleepPath = path.join(workspaceDir, "sleep.md");
    await appendFile(sleepPath, markdown, "utf8");
}

export async function writeWorkspaceTextFile(
    workspaceDir: string,
    relativePath: string,
    text: string
): Promise<void> {
    const target = path.join(workspaceDir, normalizeWorkspaceRelativePath(relativePath));
    await writeFile(target, text, "utf8");
}

export async function readTextIfExists(filePath: string): Promise<string> {
    try {
        return await readFile(filePath, "utf8");
    } catch {
        return "";
    }
}

export function normalizeWorkspaceRelativePath(value: string): string {
    return value.replaceAll("\\", "/").replace(/^\/+/, "").replace(/^\.\//, "");
}

export function isProtectedPath(value: string, workspaceDir: string): boolean {
    const normalized = normalizeWorkspaceRelativePath(path.relative(workspaceDir, path.resolve(workspaceDir, value)));
    const direct = normalizeWorkspaceRelativePath(value);
    if (protectedFileNames.has(normalized) || protectedFileNames.has(direct)) {
        return true;
    }
    const dailyFilePattern = /^\d{4}-\d{2}-\d{2}_(WorkLog|Summary)\.md$/;
    if (dailyFilePattern.test(normalized) || dailyFilePattern.test(direct)) {
        return true;
    }
    const weeklyOverridePattern = /^\d{4}_W\d{2}_Override\.md$/;
    return weeklyOverridePattern.test(normalized) || weeklyOverridePattern.test(direct);
}

export async function hasFreshProtectedIntent(
    workspaceDir: string,
    file: string,
    requestedChange?: string
): Promise<boolean> {
    const now = Date.now();
    const intents = await readProtectedIntents(workspaceDir);
    const normalizedFile = normalizeWorkspaceRelativePath(file);
    const active = intents.intents.filter((intent) => Date.parse(intent.expiresAt) > now);
    if (active.length !== intents.intents.length) {
        await writeProtectedIntents(workspaceDir, { version: 1, intents: active });
    }
    return active.some((intent) => {
        const sameFile = normalizeWorkspaceRelativePath(intent.file) === normalizedFile;
        if (!sameFile) {
            return false;
        }
        if (!requestedChange?.trim()) {
            return true;
        }
        return intent.requestedChange.toLowerCase().includes(requestedChange.toLowerCase().slice(0, 80));
    });
}

export async function addProtectedIntent(
    workspaceDir: string,
    intent: Omit<ProtectedEditIntent, "createdAt" | "expiresAt">
): Promise<ProtectedEditIntent> {
    const createdAt = nowIso();
    const expiresAt = new Date(Date.now() + 10 * 60 * 1000).toISOString();
    const entry = { ...intent, createdAt, expiresAt };
    const intents = await readProtectedIntents(workspaceDir);
    intents.intents = [
        ...intents.intents.filter((candidate) => Date.parse(candidate.expiresAt) > Date.now()),
        entry
    ];
    await writeProtectedIntents(workspaceDir, intents);
    await appendEvent(workspaceDir, {
        type: "protected_edit_intent",
        details: entry
    });
    return entry;
}
