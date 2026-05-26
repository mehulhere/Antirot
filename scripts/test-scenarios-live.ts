import fs from "node:fs/promises";
import path from "node:path";
import assert from "node:assert";
import { fileURLToPath } from "node:url";

// Local imports for storage/helpers & strategy selection test
import {
    readState,
    readStats,
    readStrategyPerformance,
    writeStrategyPerformance,
    ensureWorkspace
} from "../dist/storage.js";
import { selectDailyStrategies } from "../dist/index.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const testWorkspaceDir = path.resolve(__dirname, "../test-workspace");
const GATEWAY_URL = "http://127.0.0.1:18789/tools/invoke";

interface LiveToolResponse {
    ok: boolean;
    result: {
        content: Array<{ text: string }>;
    };
}

interface AntirotTrigger {
    id: string;
    kind: string;
    status: string;
    label?: string;
    requestedDelayMins?: number;
    scheduledDelayMins?: number;
}

// Helper to clean up the test workspace
async function cleanWorkspace() {
    try {
        await fs.rm(testWorkspaceDir, { recursive: true, force: true });
    } catch {
        // Ignore
    }
}

// Helper to invoke a tool via the live OpenClaw Gateway HTTP endpoint
async function invokeLiveTool(tool: string, args: Record<string, unknown> = {}): Promise<LiveToolResponse> {
    const response = await fetch(GATEWAY_URL, {
        method: "POST",
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify({
            tool,
            args
        })
    });
    if (!response.ok) {
        throw new Error(`HTTP Error ${response.status}: ${await response.text()}`);
    }
    return await response.json() as LiveToolResponse;
}

// Main test runner
async function runLiveTests() {
    console.log("🚀 Starting Live Antirot Scenarios Test Suite against OpenClaw Gateway...\n");

    try {
        // Setup initial clean environment
        await cleanWorkspace();
        await fs.mkdir(testWorkspaceDir, { recursive: true });
        await ensureWorkspace(testWorkspaceDir);

        // ----------------------------------------------------
        // Scenario A: The Midnight Task Planning Loop (00:00 – 06:00)
        // ----------------------------------------------------
        console.log("⏳ Testing Scenario A: Midnight Planning Loop & Linear Plan (Live)...");
        
        // 1. Check vacation=False in stats config
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

        // 3. Request linear plan slice through the live tool
        const planResponse = await invokeLiveTool("get_linear_plan", { remaining_hours: 4.0 });
        assert.ok(planResponse.ok, "get_linear_plan invoke should succeed");
        
        const planText = planResponse.result.content[0].text;
        console.log(`📋 Plan response text:\n${planText}`);
        
        assert.ok(planText.includes("Plan slice (3.5h of 4h):"), "Should show correct runway math header");
        assert.ok(planText.includes("1.5h - fixing Auth API endpoints"), "Should contain first uncompleted task");
        assert.ok(planText.includes("2h - database indexing configuration"), "Should contain second uncompleted task");
        assert.ok(!planText.includes("write README updates"), "Should omit completed task");
        assert.ok(!planText.includes("implement push notifications"), "Should omit task exceeding budget");

        console.log("✅ Scenario A Passed!");

        // ----------------------------------------------------
        // Scenario B: The Evening Strategy Shift
        // ----------------------------------------------------
        console.log("\n⏳ Testing Scenario B: Evening Strategy Shift (RL Probability Math)...");
        
        // Populate strategy performance history locally
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

        // Verify the RL daily strategy selection function locally
        const stateWithStrategies = await selectDailyStrategies(testWorkspaceDir, initialState);
        
        assert.strictEqual(stateWithStrategies.currentStrategies.length, 3, "Should select exactly 3 strategies");
        assert.ok(stateWithStrategies.currentStrategies.includes("five_minute_useful_diversion"), "Should contain top strategy");
        assert.ok(stateWithStrategies.currentStrategies.includes("strict_deadline_pressure"), "Should contain second top strategy");
        
        console.log(`🧠 Selected daily strategies: ${stateWithStrategies.currentStrategies.join(", ")}`);
        console.log("✅ Scenario B Passed!");

        // ----------------------------------------------------
        // Scenario C: Morning Initialization
        // ----------------------------------------------------
        console.log("\n⏳ Testing Scenario C: Morning Initialization (Live Sleep & Wake)...");
        
        // Start sleep mode via live tool
        const sleepResponse = await invokeLiveTool("start_sleep", { tiredness_level: 2 });
        assert.ok(sleepResponse.ok, "start_sleep invoke should succeed");
        
        const liveStateSleep = await readState(testWorkspaceDir);
        assert.strictEqual(liveStateSleep.mode, "sleeping", "Gateway should transition to sleeping mode");
        assert.ok(liveStateSleep.activeBlock, "Active block should be scheduled for sleep");
        
        // Read sleep triggers
        const triggersPath = path.join(testWorkspaceDir, ".antirot", "triggers.json");
        const triggersText = await fs.readFile(triggersPath, "utf8");
        const triggersData = JSON.parse(triggersText) as { triggers: AntirotTrigger[] };
        
        const sleepTriggers = triggersData.triggers.filter((t) => t.status === "active");
        assert.strictEqual(sleepTriggers.length, 2, "Should have scheduled normal and loud alarm triggers");
        assert.ok(sleepTriggers.some((t) => t.kind === "sleep_normal_alarm"), "Should have sleep_normal_alarm");
        assert.ok(sleepTriggers.some((t) => t.kind === "sleep_loud_alarm"), "Should have sleep_loud_alarm");

        // Complete sleep via live tool
        const wakeResponse = await invokeLiveTool("log_wake", {
            still_tired: false,
            sleep_quality: 4,
            notes: "Felt rested"
        });
        assert.ok(wakeResponse.ok, "log_wake invoke should succeed");
        
        const liveStateWake = await readState(testWorkspaceDir);
        assert.strictEqual(liveStateWake.mode, "idle", "Gateway should transition back to idle");
        assert.strictEqual(liveStateWake.activeBlock, undefined, "Active block should be cleared");
        
        const triggersWakeText = await fs.readFile(triggersPath, "utf8");
        const triggersWakeData = JSON.parse(triggersWakeText) as { triggers: AntirotTrigger[] };
        const activeWakeTriggers = triggersWakeData.triggers.filter((t) => t.status === "active");
        assert.strictEqual(activeWakeTriggers.length, 0, "Alarms should be cleared from registry");

        console.log("✅ Scenario C Passed!");

        // ----------------------------------------------------
        // Scenario D & I: Starting a Deep-Work Session & Two-Hour Alignment
        // ----------------------------------------------------
        console.log("\n⏳ Testing Scenario D & I: Starting Deep-Work & Two-Hour Alignment check (Live)...");
        
        const targetDuration = 45;
        const taskId = "auth-endpoints";
        
        const sessionResponse = await invokeLiveTool("start_session", {
            task_id: taskId,
            target_duration: targetDuration
        });
        assert.ok(sessionResponse.ok, "start_session invoke should succeed");
        
        const liveStateSession = await readState(testWorkspaceDir);
        assert.strictEqual(liveStateSession.mode, "working", "Gateway mode should be working");
        assert.ok(liveStateSession.activeBlock, "Active block should be present");
        assert.strictEqual(liveStateSession.activeBlock.name, taskId);
        
        const triggersSessionText = await fs.readFile(triggersPath, "utf8");
        const triggersSessionData = JSON.parse(triggersSessionText) as { triggers: AntirotTrigger[] };
        const sessionActiveTriggers = triggersSessionData.triggers.filter((t) => t.status === "active");
        
        assert.strictEqual(sessionActiveTriggers.length, 2, "Should register session end and alignment check triggers");
        
        const registeredSession = sessionActiveTriggers.find((t) => t.kind === "session");
        const registeredAlignment = sessionActiveTriggers.find((t) => t.kind === "alignment_check");
        assert.ok(registeredSession, "Session trigger should be active");
        assert.ok(registeredAlignment, "Alignment check trigger should be active");
        assert.strictEqual(registeredAlignment.requestedDelayMins, 120, "Alignment check should be exactly 120 mins");

        console.log("✅ Scenario D & I Passed!");

        // ----------------------------------------------------
        // Scenario E: Closing a Deep-Work Session
        // ----------------------------------------------------
        console.log("\n⏳ Testing Scenario E: Closing Deep-Work Session (Live)...");
        
        const today = new Date().toISOString().slice(0, 10);
        const statsBeforeEnd = await readStats(testWorkspaceDir);

        const endResponse = await invokeLiveTool("end_session", {
            productive_mins: 30,
            on_table_wasted_mins: 15,
            output_summary: "Worked on auth API endpoints and tested logic"
        });
        assert.ok(endResponse.ok, "end_session invoke should succeed");
        
        const liveStateEnd = await readState(testWorkspaceDir);
        assert.strictEqual(liveStateEnd.mode, "idle", "Mode should be idle");
        assert.strictEqual(liveStateEnd.activeBlock, undefined, "Active block should be cleared");
        
        const triggersEndText = await fs.readFile(triggersPath, "utf8");
        const triggersEndData = JSON.parse(triggersEndText) as { triggers: AntirotTrigger[] };
        const endActiveTriggers = triggersEndData.triggers.filter((t) => t.status === "active");
        assert.strictEqual(endActiveTriggers.length, 0, "Session triggers should be cleared");
        
        const statsAfterEnd = await readStats(testWorkspaceDir);
        assert.strictEqual((statsAfterEnd.productiveMins[today] ?? 0) - (statsBeforeEnd.productiveMins[today] ?? 0), 30, "Productive minutes should increase by 30");
        assert.strictEqual((statsAfterEnd.onTableWastedMins[today] ?? 0) - (statsBeforeEnd.onTableWastedMins[today] ?? 0), 15, "Wasted minutes should increase by 15");
        assert.strictEqual((statsAfterEnd.sessionsCompleted[today] ?? 0) - (statsBeforeEnd.sessionsCompleted[today] ?? 0), 1, "Completed sessions count should increase by 1");

        console.log("✅ Scenario E Passed!");

        // ----------------------------------------------------
        // Scenario F & H: Declaring Off-Table Routine / Mindful Break Jitter
        // ----------------------------------------------------
        console.log("\n⏳ Testing Scenario F & H: Declaring Off-Table Routine & Jitter (Live)...");
        
        const breakfastResp = await invokeLiveTool("start_routine", {
            routine_name: "Breakfast",
            duration_mins: 30
        });
        assert.ok(breakfastResp.ok, "start_routine breakfast should succeed");
        
        const meditationResp = await invokeLiveTool("start_routine", {
            routine_name: "Meditation",
            duration_mins: 20
        });
        assert.ok(meditationResp.ok, "start_routine meditation should succeed");
        
        const triggersRoutineText = await fs.readFile(triggersPath, "utf8");
        const triggersRoutineData = JSON.parse(triggersRoutineText) as { triggers: AntirotTrigger[] };
        const activeRoutines = triggersRoutineData.triggers.filter((t) => t.status === "active");
        
        assert.strictEqual(activeRoutines.length, 2, "Two routine triggers should be active");
        
        const breakfastTrigger = activeRoutines.find((t) => t.label === "Breakfast");
        const meditationTrigger = activeRoutines.find((t) => t.label === "Meditation");
        
        assert.ok(breakfastTrigger, "Breakfast trigger should exist");
        assert.ok(meditationTrigger, "Meditation trigger should exist");
        
        // Scheduled delay is jittered by 5% to 10%
        assert.ok(
            breakfastTrigger.scheduledDelayMins >= 32 && breakfastTrigger.scheduledDelayMins <= 33,
            `Breakfast scheduled delay should be 32-33 mins, got ${breakfastTrigger.scheduledDelayMins}`
        );
        
        assert.ok(
            meditationTrigger.scheduledDelayMins >= 21 && meditationTrigger.scheduledDelayMins <= 22,
            `Meditation scheduled delay should be 21-22 mins, got ${meditationTrigger.scheduledDelayMins}`
        );
        
        // Clear them
        for (const t of activeRoutines) {
            const clearResp = await invokeLiveTool("clear_active_trigger", {
                trigger_id: t.id,
                reason: "routine ended"
            });
            assert.ok(clearResp.ok, "clear_active_trigger should succeed");
        }
        
        const triggersClearedText = await fs.readFile(triggersPath, "utf8");
        const triggersClearedData = JSON.parse(triggersClearedText) as { triggers: AntirotTrigger[] };
        const activeAfterClear = triggersClearedData.triggers.filter((t) => t.status === "active");
        assert.strictEqual(activeAfterClear.length, 0, "All triggers should be cleared");

        console.log("✅ Scenario F & H Passed!");

        // ----------------------------------------------------
        // Scenario G: Low-Value Break Redirection
        // ----------------------------------------------------
        console.log("\n⏳ Testing Scenario G: Low-Value Break Redirection Targets...");
        
        const miscTodoText = await fs.readFile(path.join(testWorkspaceDir, "miscellaneous_todo.md"), "utf8");
        assert.ok(miscTodoText.includes("Miscellaneous Todo"), "Default miscellaneous_todo.md should be populated");
        assert.ok(miscTodoText.includes("Clear one tiny admin task"), "Should contain administrative redirections");
        
        console.log("✅ Scenario G Passed!");

        // ----------------------------------------------------
        // Scenario J & K: Drop-Dead Trigger & Overdue Nag Loop
        // ----------------------------------------------------
        console.log("\n⏳ Testing Scenario J & K: Trigger Rescheduling (Live)...");
        
        const showerResp = await invokeLiveTool("start_routine", {
            routine_name: "Shower",
            duration_mins: 15
        });
        assert.ok(showerResp.ok);
        
        const triggersShowerText = await fs.readFile(triggersPath, "utf8");
        const triggersShowerData = JSON.parse(triggersShowerText) as { triggers: AntirotTrigger[] };
        const showerTrigger = triggersShowerData.triggers.find((t) => t.label === "Shower" && t.status === "active");
        assert.ok(showerTrigger, "Shower trigger should be active");
        
        const rescheduleResp = await invokeLiveTool("reschedule_trigger", {
            trigger_id: showerTrigger.id,
            delay_mins: 10,
            reason: "User requested extension"
        });
        assert.ok(rescheduleResp.ok, "reschedule_trigger should succeed");
        
        const triggersRescheduledText = await fs.readFile(triggersPath, "utf8");
        const triggersRescheduledData = JSON.parse(triggersRescheduledText) as { triggers: AntirotTrigger[] };
        
        const oldShowerTrigger = triggersRescheduledData.triggers.find((t) => t.id === showerTrigger.id);
        const newShowerTrigger = triggersRescheduledData.triggers.find((t) => t.label === "Shower" && t.status === "active");
        
        assert.ok(oldShowerTrigger, "Old trigger should exist in registry");
        assert.strictEqual(oldShowerTrigger.status, "rescheduled", "Old trigger should be rescheduled");
        assert.ok(newShowerTrigger, "New trigger should be active");
        assert.strictEqual(newShowerTrigger.requestedDelayMins, 10, "New requested delay should be 10 mins");
        
        // Clean up trigger
        const clearResp = await invokeLiveTool("clear_active_trigger", {
            trigger_id: newShowerTrigger.id,
            reason: "shower ended"
        });
        assert.ok(clearResp.ok);

        console.log("✅ Scenario J & K Passed!");

        // ----------------------------------------------------
        // Scenario L: Maximum Intervention (Loud Alarm Skill)
        // ----------------------------------------------------
        console.log("\n⏳ Testing Scenario L: Maximum Intervention (Loud Alarm Live)...");
        
        const statsBeforeAlarm = await readStats(testWorkspaceDir);
        
        const alarmResp = await invokeLiveTool("trigger_loud_alarm");
        assert.ok(alarmResp.ok, "trigger_loud_alarm should succeed");
        
        const alarmText = alarmResp.result.content[0].text;
        console.log(`🔊 Alarm output message: ${alarmText}`);
        assert.ok(alarmText.includes("🔴 FALLBACK"), "Should contain fallback indicator as alarmCommand is unset");
        
        const statsAfterAlarm = await readStats(testWorkspaceDir);
        assert.strictEqual((statsAfterAlarm.loudAlarmsTriggered[today] ?? 0) - (statsBeforeAlarm.loudAlarmsTriggered[today] ?? 0), 1, "Loud alarm count should increase by 1");

        console.log("✅ Scenario L Passed!");

        // ----------------------------------------------------
        // Scenario M: Pre-Sleep De-escalation
        // ----------------------------------------------------
        console.log("\n⏳ Testing Scenario M: Pre-Sleep De-escalation (Sleep Report Live)...");
        
        const reportResp = await invokeLiveTool("get_sleep_report");
        assert.ok(reportResp.ok, "get_sleep_report should succeed");
        
        const reportText = reportResp.result.content[0].text;
        console.log(`🛌 Sleep report:\n${reportText}`);
        assert.ok(reportText.includes("Sleep debt:"), "Report should include sleep debt");
        assert.ok(reportText.includes("Recommended sleep now:"), "Report should include recommendation");

        console.log("✅ Scenario M Passed!");

        console.log("\n🎉 All live gateway scenarios successfully verified!");

    } finally {
        // Cleanup test directory
        await cleanWorkspace();
    }
}

runLiveTests().catch((err) => {
    console.error("🔴 Live Test Suite failed with error:", err);
    process.exit(1);
});
