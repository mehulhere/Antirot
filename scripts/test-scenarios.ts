import fs from "node:fs/promises";
import path from "node:path";
import assert from "node:assert";
import { fileURLToPath } from "node:url";

// Relative imports from the built JS folder
import { getLinearPlan } from "../dist/plan.js";
import {
    beginSleep,
    completeSleep,
    isGoodMorningVariant,
    getSleepSummary
} from "../dist/sleep.js";
import {
    createAntirotTrigger,
    listActiveTriggers,
    clearTrigger,
    rescheduleTrigger
} from "../dist/triggers.js";
import {
    readState,
    writeState,
    readStats,
    writeStats,
    readStrategyPerformance,
    writeStrategyPerformance,
    ensureWorkspace
} from "../dist/storage.js";
import { triggerAlarmCommand } from "../dist/runtime.js";
import { selectDailyStrategies, getOnboardingStatus } from "../dist/index.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const testWorkspaceDir = path.resolve(__dirname, "../test-workspace");

// Helper to clean up the test workspace
async function cleanWorkspace() {
    try {
        await fs.rm(testWorkspaceDir, { recursive: true, force: true });
    } catch {
        // Ignore
    }
}

// Config to disable cron scheduling for testing purposes
const testConfig = {
    workspaceDir: testWorkspaceDir,
    enableCron: false
};

// Main test runner
async function runTests() {
    console.log("🚀 Starting Antirot Scenarios Test Suite...\n");

    try {
        // Setup initial clean environment
        await cleanWorkspace();
        await fs.mkdir(testWorkspaceDir, { recursive: true });
        await ensureWorkspace(testWorkspaceDir);

        // ----------------------------------------------------
        // Scenario A: Onboarding Answers Saving Flow
        // ----------------------------------------------------
        console.log("⏳ Testing Scenario A: Onboarding Answers Saving Flow...");

        // 1. Initialize files to baseline template (placeholder content)
        await fs.writeFile(path.join(testWorkspaceDir, "longterm.md"), "# Long-Term Goals\n\n## Direction\n- Onboarding will ask what the user is trying to build or become\n", "utf8");
        await fs.writeFile(path.join(testWorkspaceDir, "shortterm.md"), "# Short-Term State\n\n## Current Priorities\n- Onboarding will ask what the user is working on now\n", "utf8");
        await fs.writeFile(path.join(testWorkspaceDir, "behavior.md"), "# Behavior Memory\n\n## Recurring Patterns\n- Onboarding will ask what helps or derails the user\n- Known drift loops go here\n- Tactics that work or fail go here\n", "utf8");

        // 2. Check initial onboarding status: missing goals (longterm & shortterm)
        const initialOnboard = await getOnboardingStatus(testWorkspaceDir);
        assert.deepStrictEqual(initialOnboard.missing, ["longterm", "shortterm"], "Should initially miss longterm and shortterm files");
        assert.ok(initialOnboard.nextQuestion.includes("complete mental rot"), "Greeting should match the cool-but-rude prompt");

        // 3. User responds to goals collection prompt -> we append answers to both files
        const dayKey = new Date().toISOString().slice(0, 10);
        await fs.appendFile(path.join(testWorkspaceDir, "longterm.md"), `\n## Profile Update - ${dayKey}\n\n### Level 1 Goals\n- Finish coding this cool behavioral OS\n- Build high-performance AI tools\n`, "utf8");
        await fs.appendFile(path.join(testWorkspaceDir, "shortterm.md"), `\n## Profile Update - ${dayKey}\n\n### Level 3 Goals\n- Finish task-scenarios onboarding tests\n- Fix any linter complaints\n`, "utf8");

        // Write complete state just like save_onboarding_answers does to prevent immediate review trigger
        const finalState = await readState(testWorkspaceDir);
        finalState.onboardingCompletedAt = new Date().toISOString();
        finalState.lastGoalReviewAt = new Date().toISOString();
        await writeState(testWorkspaceDir, finalState);

        // 4. Verify onboarding is completely finished
        const finalOnboard = await getOnboardingStatus(testWorkspaceDir);
        assert.deepStrictEqual(finalOnboard.missing, [], "Onboarding should be completely finished");
        assert.strictEqual(finalOnboard.nextQuestion, "No onboarding question is due.", "No further questions should be due");

        console.log("✅ Scenario A Passed!");

        // ----------------------------------------------------
        // Scenario B: The Midnight Task Planning Loop (00:00 – 06:00)
        // ----------------------------------------------------
        console.log("\n⏳ Testing Scenario B: Midnight Planning Loop & Linear Plan...");
        
        // 1. Demands next day's tasks if vacation=False in stats config
        await readStats(testWorkspaceDir);
        const initialState = await readState(testWorkspaceDir);
        assert.strictEqual(initialState.vacation, false, "Vacation mode should be disabled by default");
        
        // 2. User inputs tasks in tasks.md
        const tasksContent = `
# Task Pipeline

[ ] 1.5h - fixing Auth API endpoints
[ ] 2.0h - database indexing configuration
[x] 1.0h - write README updates
[ ] 3.0h - implement push notifications
        `;
        await fs.writeFile(path.join(testWorkspaceDir, "tasks.md"), tasksContent.trim(), "utf8");

        // 3. Shuts off reminder loop, calls getLinearPlan() against wake hours runway
        const remainingRunwayHours = 4.0; // Say user has 4 hours left until sleep
        const plan = await getLinearPlan(testWorkspaceDir, remainingRunwayHours);
        
        // The first task is 1.5h. The second is 2.0h. Total = 3.5h.
        // The third is checked (x) so it should be skipped.
        // The fourth task is 3.0h, which exceeds the remaining budget (3.5 + 3 = 6.5h > 4h), so it is not selected.
        assert.strictEqual(plan.tasks.length, 2, "Should select 2 tasks within runway");
        assert.strictEqual(plan.tasks[0].title, "fixing Auth API endpoints");
        assert.strictEqual(plan.tasks[0].hours, 1.5);
        assert.strictEqual(plan.tasks[1].title, "database indexing configuration");
        assert.strictEqual(plan.tasks[1].hours, 2.0);
        assert.strictEqual(plan.totalHours, 3.5, "Total hours of plan should be 3.5h");
        assert.strictEqual(plan.skippedCompleted, 1, "Should skip 1 completed task");

        console.log("✅ Scenario B Passed!");

        // ----------------------------------------------------
        // Scenario C: The Evening Strategy Shift
        // ----------------------------------------------------
        console.log("\n⏳ Testing Scenario C: Evening Strategy Shift (RL Probability Math)...");
        
        // Populate strategy performance history
        const initialPerformance = await readStrategyPerformance(testWorkspaceDir);
        initialPerformance.strategies = {
            "strict_deadline_pressure": { attempts: [{ at: "2026-05-20T22:00:00Z", status: true }] }, // Score 1.0
            "rare_identity_praise": { attempts: [{ at: "2026-05-20T22:00:00Z", status: false }] }, // Score 0.0
            "five_minute_useful_diversion": {
                attempts: [
                    { at: "2026-05-20T22:00:00Z", status: true },
                    { at: "2026-05-21T22:00:00Z", status: true }
                ]
            }, // Score 1.0
            "calm_sleep_protection": { attempts: [] } // Score 0.0
        };
        await writeStrategyPerformance(testWorkspaceDir, initialPerformance);

        // Calculate and select strategies
        const stateWithStrategies = await selectDailyStrategies(testWorkspaceDir, initialState);
        
        // Should select:
        // 1. "five_minute_useful_diversion" (2 attempts, 100% win rate)
        // 2. "strict_deadline_pressure" (1 attempt, 100% win rate)
        // 3. A wild-card/exploratory strategy based on the day key hash
        assert.strictEqual(stateWithStrategies.currentStrategies.length, 3, "Should select exactly 3 strategies");
        assert.ok(stateWithStrategies.currentStrategies.includes("five_minute_useful_diversion"), "Should contain top strategy");
        assert.ok(stateWithStrategies.currentStrategies.includes("strict_deadline_pressure"), "Should contain second top strategy");
        
        console.log(`🧠 Selected daily strategies: ${stateWithStrategies.currentStrategies.join(", ")}`);
        console.log("✅ Scenario C Passed!");

        // ----------------------------------------------------
        // Scenario D: Morning Initialization
        // ----------------------------------------------------
        console.log("\n⏳ Testing Scenario D: Morning Initialization (GM Wake Detection)...");
        
        // Put system in sleep mode first
        const beforeSleepState = await readState(testWorkspaceDir);
        const sleep = await beginSleep({
            workspaceDir: testWorkspaceDir,
            tirednessLevel: 2
        });
        await writeState(testWorkspaceDir, {
            ...beforeSleepState,
            mode: "sleeping",
            activeBlock: {
                kind: "sleep",
                name: "sleep",
                startedAt: sleep.session.sleepStartedAt,
                durationMins: sleep.requirement.requiredHours * 60,
                callbackReason: "Sleep recovery window"
            }
        });
        const sleepingState = await readState(testWorkspaceDir);
        assert.strictEqual(sleepingState.mode, "sleeping", "Mode should be sleeping");
        assert.ok(sleepingState.activeBlock, "Active block should be scheduled for sleep");

        // Verify GM variants
        assert.ok(isGoodMorningVariant("good morning"), "good morning should match");
        assert.ok(isGoodMorningVariant("GM coach"), "GM coach should match");
        assert.ok(isGoodMorningVariant("i'm awake"), "i'm awake should match");
        assert.ok(!isGoodMorningVariant("not awake yet"), "should not false match other strings");

        // Complete sleep (simulate GM wake logs)
        await completeSleep({
            workspaceDir: testWorkspaceDir,
            stillTired: false,
            sleepQuality: 4,
            notes: "Felt rested"
        });
        const stateAfterComplete = await readState(testWorkspaceDir);
        await writeState(testWorkspaceDir, {
            ...stateAfterComplete,
            mode: stateAfterComplete.vacation ? "vacation" : "idle",
            activeBlock: undefined
        });
        const afterWakeState = await readState(testWorkspaceDir);
        assert.strictEqual(afterWakeState.mode, "idle", "Mode should transition back to idle upon waking");
        assert.strictEqual(afterWakeState.activeBlock, undefined, "Active block should be cleared");

        console.log("✅ Scenario D Passed!");

        // ----------------------------------------------------
        // Scenario E & J: Starting a Deep-Work Session & Two-Hour Alignment
        // ----------------------------------------------------
        console.log("\n⏳ Testing Scenario E & J: Starting Deep-Work & Two-Hour Alignment check...");
        
        // Simulate start_session tool call behavior
        const targetDuration = 45; // 45-minute focus session
        const taskId = "auth-endpoints";
        const preSessionState = await readState(testWorkspaceDir);
        
        await writeState(testWorkspaceDir, {
            ...preSessionState,
            mode: "working",
            activeBlock: {
                kind: "session",
                name: taskId,
                startedAt: new Date().toISOString(),
                durationMins: targetDuration
            }
        });

        // Register session ending trigger and alignment check trigger
        await createAntirotTrigger({
            workspaceDir: testWorkspaceDir,
            config: testConfig,
            kind: "session",
            scope: "daily",
            label: taskId,
            reason: `Work session target ended: ${taskId}`,
            delayMins: targetDuration,
            cronName: `antirot-session-${taskId}`,
            systemEvent: `Antirot work session ended: ${taskId}`
        });

        await createAntirotTrigger({
            workspaceDir: testWorkspaceDir,
            config: testConfig,
            kind: "alignment_check",
            scope: "daily",
            label: taskId,
            reason: `Two-hour alignment check: ${taskId}`,
            delayMins: 120, // Scenario I: 2-hour alignment trigger
            cronName: "antirot-two-hour-alignment",
            systemEvent: "Antirot two-hour alignment check."
        });

        const activeTriggers = await listActiveTriggers(testWorkspaceDir);
        assert.strictEqual(activeTriggers.length, 2, "Two triggers should be registered");
        
        const registeredSession = activeTriggers.find(t => t.kind === "session");
        const registeredAlignment = activeTriggers.find(t => t.kind === "alignment_check");
        assert.ok(registeredSession, "Session trigger should be active");
        assert.ok(registeredAlignment, "Alignment check trigger should be active");
        assert.strictEqual(registeredAlignment.requestedDelayMins, 120, "Alignment check should be exactly 120 mins");

        console.log("✅ Scenario E & J Passed!");

        // ----------------------------------------------------
        // Scenario F: Closing a Deep-Work Session
        // ----------------------------------------------------
        console.log("\n⏳ Testing Scenario F: Closing Deep-Work Session...");
        
        // Log end of session
        const statsBeforeEnd = await readStats(testWorkspaceDir);
        const today = new Date().toISOString().slice(0, 10);
        
        // Call end_session simulation
        const stats = await readStats(testWorkspaceDir);
        stats.productiveMins[today] = (stats.productiveMins[today] ?? 0) + 30;
        stats.onTableWastedMins[today] = (stats.onTableWastedMins[today] ?? 0) + 15;
        stats.sessionsCompleted[today] = (stats.sessionsCompleted[today] ?? 0) + 1;
        await writeStats(testWorkspaceDir, stats);

        const currentSessionState = await readState(testWorkspaceDir);
        await writeState(testWorkspaceDir, { ...currentSessionState, mode: "idle", activeBlock: undefined });

        // Clear triggers
        const activeBeforeClear = await listActiveTriggers(testWorkspaceDir);
        assert.strictEqual(activeBeforeClear.length, 2);
        
        for (const t of activeBeforeClear) {
            await clearTrigger({
                workspaceDir: testWorkspaceDir,
                config: testConfig,
                triggerId: t.id,
                reason: "session ended early or completed"
            });
        }

        const activeAfterClear = await listActiveTriggers(testWorkspaceDir);
        assert.strictEqual(activeAfterClear.length, 0, "All session triggers should be cleared");

        const statsAfterEnd = await readStats(testWorkspaceDir);
        assert.strictEqual((statsAfterEnd.productiveMins[today] ?? 0) - (statsBeforeEnd.productiveMins[today] ?? 0), 30, "Productive minutes should increase by 30");
        assert.strictEqual((statsAfterEnd.onTableWastedMins[today] ?? 0) - (statsBeforeEnd.onTableWastedMins[today] ?? 0), 15, "Wasted minutes should increase by 15");
        assert.strictEqual((statsAfterEnd.sessionsCompleted[today] ?? 0) - (statsBeforeEnd.sessionsCompleted[today] ?? 0), 1, "Completed sessions count should increase by 1");

        console.log("✅ Scenario F Passed!");

        // ----------------------------------------------------
        // Scenario G & I: Declaring Off-Table Routine / Mindful Break Jitter
        // ----------------------------------------------------
        console.log("\n⏳ Testing Scenario G & I: Declaring Off-Table Routine & Jitter Calculation...");

        // Scenario F: Breakfast block duration = 30 minutes, 10% jitter = 3 minutes. Total delay = 33 minutes
        // Scenario H: Meditation block duration = 20 minutes, 8% jitter = 1.6 minutes. Total delay = 21.6 minutes -> rounds to 22
        // Since runtime.ts uses a random jitter of 5% - 10%, we'll test the output bounds of applyHiddenTimeBuffer
        
        // We'll calculate the bounds for 30 minutes and 20 minutes
        // applyHiddenTimeBuffer is not exported, but we can verify created triggers' scheduled delay
        const breakfastTrigger = await createAntirotTrigger({
            workspaceDir: testWorkspaceDir,
            config: testConfig,
            kind: "routine",
            scope: "daily",
            label: "Breakfast",
            reason: "Routine check: Breakfast",
            delayMins: 30,
            cronName: "antirot-routine-Breakfast",
            systemEvent: "Breakfast check"
        });

        // The scheduled delay must be delayMins * (1 + jitter) where jitter is 5% to 10%
        // Min scheduled delay: Math.round(30 * 1.05) = 32
        // Max scheduled delay: Math.round(30 * 1.10) = 33
        assert.ok(
            breakfastTrigger.trigger.scheduledDelayMins && 
            breakfastTrigger.trigger.scheduledDelayMins >= 32 && 
            breakfastTrigger.trigger.scheduledDelayMins <= 33,
            `Scheduled delay for Breakfast should be between 32 and 33 mins (jittered), got ${breakfastTrigger.trigger.scheduledDelayMins}`
        );

        const meditationTrigger = await createAntirotTrigger({
            workspaceDir: testWorkspaceDir,
            config: testConfig,
            kind: "routine",
            scope: "daily",
            label: "Meditation",
            reason: "Routine check: Meditation",
            delayMins: 20,
            cronName: "antirot-routine-Meditation",
            systemEvent: "Meditation check"
        });

        // Min scheduled delay: Math.round(20 * 1.05) = 21
        // Max scheduled delay: Math.round(20 * 1.10) = 22
        assert.ok(
            meditationTrigger.trigger.scheduledDelayMins && 
            meditationTrigger.trigger.scheduledDelayMins >= 21 && 
            meditationTrigger.trigger.scheduledDelayMins <= 22,
            `Scheduled delay for Meditation should be between 21 and 22 mins (jittered), got ${meditationTrigger.trigger.scheduledDelayMins}`
        );

        // Clear active triggers
        const activeRoutines = await listActiveTriggers(testWorkspaceDir);
        for (const t of activeRoutines) {
            await clearTrigger({
                workspaceDir: testWorkspaceDir,
                config: testConfig,
                triggerId: t.id,
                reason: "routine ended"
            });
        }

        console.log("✅ Scenario G & I Passed!");

        // ----------------------------------------------------
        // Scenario H: Low-Value Break Redirection
        // ----------------------------------------------------
        console.log("\n⏳ Testing Scenario H: Low-Value Break Redirection Targets...");
        
        // Ensure miscellaneous_todo.md exists and is populated
        const miscTodoText = await fs.readFile(path.join(testWorkspaceDir, "miscellaneous_todo.md"), "utf8");
        assert.ok(miscTodoText.includes("Miscellaneous Todo"), "Default miscellaneous_todo.md should be populated");
        assert.ok(miscTodoText.includes("Clear one tiny admin task"), "Should contain simple administrative redirections");
        
        console.log("✅ Scenario H Passed!");

        // ----------------------------------------------------
        // Scenario K & L: Drop-Dead Trigger & Overdue Nag Loop
        // ----------------------------------------------------
        console.log("\n⏳ Testing Scenario K & L: Trigger Rescheduling (Extension / Nag Loop)...");
        
        // 1. Add a routine trigger
        const initialRoutineTrigger = await createAntirotTrigger({
            workspaceDir: testWorkspaceDir,
            config: testConfig,
            kind: "routine",
            scope: "daily",
            label: "Shower",
            reason: "Routine check: Shower",
            delayMins: 15,
            cronName: "antirot-routine-Shower",
            systemEvent: "Shower check"
        });

        // 2. User asks for more time -> reschedule trigger
        const rescheduleResult = await rescheduleTrigger({
            workspaceDir: testWorkspaceDir,
            config: testConfig,
            triggerId: initialRoutineTrigger.trigger.id,
            delayMins: 10,
            reason: "User requested extension"
        });

        assert.strictEqual(rescheduleResult.oldTrigger?.status, "cleared", "Old trigger should be cleared");
        assert.strictEqual(rescheduleResult.newTrigger?.status, "active", "New trigger should be active");
        assert.strictEqual(rescheduleResult.newTrigger?.requestedDelayMins, 10, "New delay should be 10 minutes");
        
        // Clean up trigger
        if (rescheduleResult.newTrigger) {
            await clearTrigger({
                workspaceDir: testWorkspaceDir,
                config: testConfig,
                triggerId: rescheduleResult.newTrigger.id,
                reason: "routine ended"
            });
        }

        console.log("✅ Scenario K & L Passed!");

        // ----------------------------------------------------
        // Scenario M: Maximum Intervention (Loud Alarm Skill)
        // ----------------------------------------------------
        console.log("\n⏳ Testing Scenario M: Maximum Intervention (Loud Alarm)...");
        
        const statsBeforeAlarm = await readStats(testWorkspaceDir);
        
        // Simulate trigger_loud_alarm tool execution
        const alarmResult = await triggerAlarmCommand(testConfig);
        assert.strictEqual(alarmResult.ok, false, "Should fallback gracefully when alarmCommand is not set");
        assert.ok(alarmResult.message.includes("🔴 FALLBACK"), "Should contain fallback log prefix");

        const statsAfterAlarm = await readStats(testWorkspaceDir);
        statsAfterAlarm.loudAlarmsTriggered[today] = (statsAfterAlarm.loudAlarmsTriggered[today] ?? 0) + 1;
        await writeStats(testWorkspaceDir, statsAfterAlarm);

        const statsFinal = await readStats(testWorkspaceDir);
        assert.strictEqual((statsFinal.loudAlarmsTriggered[today] ?? 0) - (statsBeforeAlarm.loudAlarmsTriggered[today] ?? 0), 1, "Loud alarms count should increase by 1");

        console.log("✅ Scenario M Passed!");

        // ----------------------------------------------------
        // Scenario N: Pre-Sleep De-escalation
        // ----------------------------------------------------
        console.log("\n⏳ Testing Scenario N: Pre-Sleep De-escalation (Sleep Summary & Debt)...");
        
        const sleepSummaryText = await getSleepSummary(testWorkspaceDir);
        assert.ok(sleepSummaryText.includes("Sleep debt:"), "Sleep summary should report debt");
        assert.ok(sleepSummaryText.includes("Recommended sleep now:"), "Sleep summary should report recommendations");

        console.log("✅ Scenario N Passed!");

        console.log("\n🎉 All scenarios successfully verified programmatically!");

    } finally {
        // Cleanup test directory
        await cleanWorkspace();
    }
}

runTests().catch((err) => {
    console.error("🔴 Test Suite failed with error:", err);
    process.exit(1);
});
