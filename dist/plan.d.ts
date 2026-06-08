import type { LinearTask } from "./types.js";
export declare function parseLinearTasks(text: string): LinearTask[];
export declare function getLinearPlan(workspaceDir: string, remainingHours: number): Promise<{
    tasks: LinearTask[];
    totalHours: number;
    skippedCompleted: number;
}>;
export declare function addPipelineTask(workspaceDir: string, title: string, hours: number): Promise<void>;
export declare function updatePipelineTaskStatus(workspaceDir: string, taskIndex: number, status: "completed" | "deleted"): Promise<void>;
