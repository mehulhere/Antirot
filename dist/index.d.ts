import type { AntirotConfig, AntirotState } from "./types.js";
export declare function getOnboardingStatus(workspaceDir: string, state?: AntirotState): Promise<{
    missing: string[];
    reviewDue: boolean;
    nextQuestion: string;
}>;
export declare function selectDailyStrategies(workspaceDir: string, state: AntirotState, config?: AntirotConfig): Promise<AntirotState>;
declare const _default: {
    id: string;
    name: string;
    description: string;
    configSchema: import("openclaw/plugin-sdk/plugin-entry").OpenClawPluginConfigSchema;
    register: NonNullable<import("openclaw/plugin-sdk/plugin-entry").OpenClawPluginDefinition["register"]>;
} & Pick<import("openclaw/plugin-sdk/plugin-entry").OpenClawPluginDefinition, "kind" | "reload" | "nodeHostCommands" | "securityAuditCollectors">;
export default _default;
