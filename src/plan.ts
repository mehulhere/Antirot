import path from "node:path";
import { readTextIfExists } from "./storage.js";
import type { LinearTask } from "./types.js";

const taskLinePattern = /^\s*[-*]?\s*\[(?<checked>[ xX])\]\s*(?<hours>\d+(?:\.\d+)?)h\s*-\s*(?<title>.+?)\s*$/u;

export function parseLinearTasks(text: string): LinearTask[] {
    return text
        .split(/\r?\n/u)
        .map((line) => {
            const match = taskLinePattern.exec(line);
            if (!match?.groups) {
                return null;
            }
            return {
                raw: line,
                title: match.groups.title.trim(),
                hours: Number(match.groups.hours),
                checked: match.groups.checked.trim().toLowerCase() === "x"
            } satisfies LinearTask;
        })
        .filter((task): task is LinearTask => Boolean(task));
}

export async function getLinearPlan(workspaceDir: string, remainingHours: number): Promise<{
    tasks: LinearTask[];
    totalHours: number;
    skippedCompleted: number;
}> {
    const text = await readTextIfExists(path.join(workspaceDir, "tasks.md"));
    const tasks = parseLinearTasks(text);
    const selected: LinearTask[] = [];
    let totalHours = 0;
    let skippedCompleted = 0;
    for (const task of tasks) {
        if (task.checked) {
            skippedCompleted += 1;
            continue;
        }
        if (selected.length > 0 && totalHours + task.hours > remainingHours) {
            break;
        }
        if (selected.length === 0 || totalHours + task.hours <= remainingHours) {
            selected.push(task);
            totalHours += task.hours;
        }
    }
    return { tasks: selected, totalHours, skippedCompleted };
}
