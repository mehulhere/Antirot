export type MiscTask = {
    text: string;
    source?: string;
    reason?: string;
    createdAt?: string;
};
export declare function addMiscTask(workspaceDir: string, task: MiscTask): Promise<MiscTask>;
export declare function listMiscTasks(workspaceDir: string, limit?: number): Promise<string[]>;
export declare function popMiscTasks(workspaceDir: string, count?: number): Promise<string[]>;
