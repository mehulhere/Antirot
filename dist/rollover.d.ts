export type RolloverResult = {
    date: string;
    carried: string[];
    completed: string[];
    added: string[];
};
export declare function rolloverTasks(params: {
    workspaceDir: string;
    newTasks?: string[];
    summary?: string;
}): Promise<RolloverResult>;
