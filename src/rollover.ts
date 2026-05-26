import path from "node:path";
import {
    appendEvent,
    appendWorkEntry,
    readTextIfExists,
    todayKey,
    writeWorkspaceTextFile
} from "./storage.js";

const taskPattern = /^\s*[-*]?\s*\[(?<checked>[ xX])\]\s*(?<rest>.+?)\s*$/u;
const estimatedTaskPattern = /^\s*\d+(?:\.\d+)?h\s*-\s*.+/u;

export type RolloverResult = {
    date: string;
    carried: string[];
    completed: string[];
    added: string[];
};

function normalizeTaskLine(value: string): string {
    const text = value.trim();
    if (!text) {
        return "";
    }
    if (/^\s*[-*]?\s*\[[ xX]\]/u.test(text)) {
        return text.startsWith("-") || text.startsWith("*") ? text : `[ ] ${text.replace(/^\s*\[[ xX]\]\s*/u, "")}`;
    }
    if (estimatedTaskPattern.test(text)) {
        return `[ ] ${text}`;
    }
    return `[ ] 0.5h - ${text}`;
}

export async function rolloverTasks(params: {
    workspaceDir: string;
    newTasks?: string[];
    summary?: string;
}): Promise<RolloverResult> {
    const current = await readTextIfExists(path.join(params.workspaceDir, "tasks.md"));
    const carried: string[] = [];
    const completed: string[] = [];
    for (const line of current.split(/\r?\n/u)) {
        const match = taskPattern.exec(line);
        if (!match?.groups) {
            continue;
        }
        const normalized = `[ ] ${match.groups.rest.trim()}`;
        if (match.groups.checked.trim().toLowerCase() === "x") {
            completed.push(normalized);
        } else {
            carried.push(normalized);
        }
    }
    const added = (params.newTasks ?? [])
        .map(normalizeTaskLine)
        .filter(Boolean);
    const date = todayKey();
    const nextLines = [
        "# Task Pipeline",
        "",
        ...carried,
        ...added
    ];
    await writeWorkspaceTextFile(params.workspaceDir, "tasks.md", `${nextLines.join("\n").trimEnd()}\n`);
    await appendWorkEntry(
        params.workspaceDir,
        [
            `\n## ${date} Nightly Rollover`,
            "",
            `- Completed tasks cleared: ${completed.length}`,
            `- Tasks carried forward: ${carried.length}`,
            `- New tasks added: ${added.length}`,
            params.summary ? `- Summary: ${params.summary}` : undefined,
            completed.length ? "\n### Completed" : undefined,
            ...completed.map((task) => `- ${task.replace(/^\[ \]\s*/u, "")}`)
        ].filter(Boolean).join("\n") + "\n"
    );
    await appendEvent(params.workspaceDir, {
        type: "nightly_rollover",
        details: { date, carried: carried.length, completed: completed.length, added: added.length }
    });
    return { date, carried, completed, added };
}
