import path from "node:path";
import { readTextIfExists, writeWorkspaceTextFile } from "./storage.js";
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

export async function addPipelineTask(
    workspaceDir: string,
    title: string,
    hours: number
): Promise<void> {
    const tasksPath = path.join(workspaceDir, "tasks.md");
    const text = await readTextIfExists(tasksPath);
    const lines = text.split(/\r?\n/u);
    // Remove last element if it's empty
    if (lines.length > 0 && lines[lines.length - 1].trim() === "") {
        lines.pop();
    }
    lines.push(`[ ] ${hours.toFixed(1)}h - ${title}`);
    await writeWorkspaceTextFile(workspaceDir, "tasks.md", lines.join("\n") + "\n");
}

export async function updatePipelineTaskStatus(
    workspaceDir: string,
    taskIndex: number,
    status: "completed" | "deleted"
): Promise<void> {
    const tasksPath = path.join(workspaceDir, "tasks.md");
    const text = await readTextIfExists(tasksPath);
    const lines = text.split(/\r?\n/u);
    
    let currentTaskCount = 0;
    const newLines: string[] = [];

    for (const line of lines) {
        const isTask = taskLinePattern.test(line);
        if (isTask) {
            currentTaskCount++;
            if (currentTaskCount === taskIndex) {
                if (status === "deleted") {
                    continue;
                } else if (status === "completed") {
                    const match = taskLinePattern.exec(line);
                    if (match && match.groups) {
                        newLines.push(`[x] ${match.groups.hours}h - ${match.groups.title}`);
                    } else {
                        newLines.push(line);
                    }
                    continue;
                }
            }
        }
        newLines.push(line);
    }
    
    await writeWorkspaceTextFile(workspaceDir, "tasks.md", newLines.join("\n") + "\n");
}

