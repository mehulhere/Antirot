import type { LinearTask } from "./types.js";
export declare function parseLinearTasks(text: string): LinearTask[];
export declare function getLinearPlan(workspaceDir: string, remainingHours: number): Promise<{
    tasks: LinearTask[];
    totalHours: number;
    skippedCompleted: number;
}>;
