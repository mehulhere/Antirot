import path from "node:path";
import { appendEvent, nowIso, readTextIfExists, writeWorkspaceTextFile } from "./storage.js";
function miscPath(workspaceDir) {
    return path.join(workspaceDir, "miscellaneous_todo.md");
}
function formatMiscTask(task) {
    const meta = [
        task.source ? `source=${task.source}` : undefined,
        task.reason ? `reason=${task.reason}` : undefined,
        `created=${task.createdAt ?? nowIso()}`
    ].filter(Boolean).join("; ");
    return `- ${task.text.trim()} <!-- ${meta} -->`;
}
function taskTextFromLine(line) {
    const match = /^\s*[-*]\s+(?<text>.+?)\s*(?:<!--.*-->)?\s*$/u.exec(line);
    return match?.groups?.text.trim();
}
export async function addMiscTask(workspaceDir, task) {
    const entry = {
        ...task,
        createdAt: task.createdAt ?? nowIso()
    };
    const current = await readTextIfExists(miscPath(workspaceDir));
    const next = current.trim()
        ? `${current.trimEnd()}\n${formatMiscTask(entry)}\n`
        : `# Miscellaneous Todo\n\n${formatMiscTask(entry)}\n`;
    await writeWorkspaceTextFile(workspaceDir, "miscellaneous_todo.md", next);
    await appendEvent(workspaceDir, {
        type: "misc_task_added",
        details: entry
    });
    return entry;
}
export async function listMiscTasks(workspaceDir, limit = 10) {
    const current = await readTextIfExists(miscPath(workspaceDir));
    return current
        .split(/\r?\n/u)
        .map(taskTextFromLine)
        .filter((task) => Boolean(task))
        .slice(0, Math.max(1, Math.round(limit)));
}
export async function popMiscTasks(workspaceDir, count = 1) {
    const current = await readTextIfExists(miscPath(workspaceDir));
    const remaining = [];
    const popped = [];
    const target = Math.max(1, Math.round(count));
    for (const line of current.split(/\r?\n/u)) {
        const task = taskTextFromLine(line);
        if (task && popped.length < target) {
            popped.push(task);
            continue;
        }
        remaining.push(line);
    }
    await writeWorkspaceTextFile(workspaceDir, "miscellaneous_todo.md", `${remaining.join("\n").trimEnd()}\n`);
    await appendEvent(workspaceDir, {
        type: "misc_tasks_popped",
        details: { popped }
    });
    return popped;
}
