# iOS Editorial Operator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Antirot’s glass-card UI with a minimal black/ivory/burnt-orange editorial system centered on a realistic human Coach and truthful task/performance information.

**Architecture:** Preserve the native SwiftUI environment-object and backend data flow. Replace the shared visual primitives in `AntirotDesign.swift`, introduce a tiny shared navigation model for cross-tab routing, compose flat screen sections in each feature view, and integrate one new generated Coach-stage asset.

**Tech Stack:** Swift 5.10, SwiftUI, UIKit chat gesture bridge, XCTest, XcodeGen, built-in Imagegen, GitHub Actions TestFlight workflow.

## Global Constraints

- Deployment target remains iOS 17.0 and iPhone portrait.
- No third-party UI framework.
- Preserve API payloads, auth, alarms, Screen Time, voice, task parsing, chat callbacks, and runtime state transitions.
- Use semantic Dynamic Type styles and minimum 44×44 point controls.
- Primary interface colors are Ink `#080807`, Graphite `#171714`, Paper `#F0ECE2`, Ash `#8A877F`, Rule `#34322E`, and Signal Orange `#E45B2C`.
- Active primary UI uses no glass material, decorative glow, or corner radius above 16 points.
- Display only backend-recorded values or explicitly labeled estimates.
- Every visible chevron/button must perform its implied action.

---

### Task 1: Editorial tokens and app shell

**Files:**
- Modify: `apps/ios/AntirotAlarmTests/Sources/CinematicLayoutTests.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/AntirotDesign.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/MainTabView.swift`

**Interfaces:**
- Produces: `AntirotEditorialPalette`, `AntirotEditorialMetrics`, `EditorialScreen`, `EditorialSectionRule`, `EditorialKicker`, and `AppNavigationModel`.
- Consumes: existing `AppScreen` destinations and feature-view environment objects.

- [ ] Write failing tests asserting Ink RGB values, maximum active radius 16, bottom bar minimum target 44, content clearance, and four unchanged destinations.
- [ ] Verify RED with source checks showing editorial symbols are absent and glass radii remain above 16.
- [ ] Replace active palette/surface primitives with solid editorial tokens, semantic serif/sans/monospaced helpers, rules, and flat sections. Retain compatibility aliases only where required for untouched code.
- [ ] Replace floating navigation with a safe-area-inset flat bar and inject `AppNavigationModel` into all destination views.
- [ ] Verify GREEN using the test-source invariants, Swift tree-sitter parsing, and `git diff --check`.
- [ ] Commit with `feat: add ios editorial design system`.

### Task 2: Generate and rebuild the Coach experience

**Files:**
- Create: `apps/ios/AntirotAlarm/Resources/Assets.xcassets/AntirotCoachEditorial.imageset/coach-editorial.png`
- Create: `apps/ios/AntirotAlarm/Resources/Assets.xcassets/AntirotCoachEditorial.imageset/Contents.json`
- Modify: `apps/ios/AntirotAlarm/Sources/CoachStage.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/HomeView.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/PrimaryActionButton.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/GlassSheet.swift`
- Modify: `apps/ios/AntirotAlarmTests/Sources/CinematicLayoutTests.swift`
- Modify: `apps/ios/AntirotAlarmTests/Sources/ChatSheetDetentsTests.swift`

**Interfaces:**
- Produces: asset name `AntirotCoachEditorial`, honest connectivity label, flat dominant action, compact composer, and opaque expanded conversation.
- Consumes: unchanged `CoachStateActions`, mic/send/play callbacks, detent functions, and Coach environment state.

- [ ] Change the asset-name and chat target tests first; verify they fail against current names/presentation constants.
- [ ] Generate one portrait asset with built-in Imagegen from the approved brief, inspect it, copy it into the new imageset, and validate dimensions/JSON.
- [ ] Recompose Coach with unboxed status typography, one dominant rectangular action, plain secondary text actions, and one compact Graphite composer surface.
- [ ] Keep only visible composer/handle chat-opening affordances and add explicit accessibility open/collapse actions while preserving UIKit pan behavior.
- [ ] Make expanded chat opaque and editorial; preserve message order, audio playback, keyboard dismissal, send, mic, and status behavior.
- [ ] Parse changed Swift, validate asset metadata, and verify callbacks remain textually present.
- [ ] Commit with `feat: redesign ios coach editorially`.

### Task 3: Truthful Tasks and navigation

**Files:**
- Modify: `apps/ios/AntirotAlarm/Sources/TaskBoardView.swift`
- Modify: `apps/ios/AntirotAlarmTests/Sources/TaskBoardParserTests.swift`

**Interfaces:**
- Consumes: `AppNavigationModel`, `TaskBoardParser`, existing memory/state fetches, and Coach draft.
- Produces: flat task list, single highlighted task, truthful recorded/estimated labels, and functional Add task routing.

- [ ] Add tests for a presentation helper that returns `nil` when no recorded duration exists instead of inventing 30/45/120 minutes.
- [ ] Verify RED against current fallback duration behavior.
- [ ] Remove overview/summary duplication and quote card; implement title, underlined scope switcher, metadata line, one highlighted task, and ruled rows.
- [ ] Make Add task a labeled control that sets the draft and selects Coach through `AppNavigationModel`.
- [ ] Re-run parser/presentation invariants and confirm parser implementation is unchanged.
- [ ] Commit with `feat: simplify ios task execution`.

### Task 4: Performance report, Settings, supporting screens, and widget

**Files:**
- Modify: `apps/ios/AntirotAlarm/Sources/StatsView.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/SettingsView.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/LoginView.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/AlarmsView.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/PlanView.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/MemoryFilesView.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/MemorySnapshotsView.swift`
- Modify: `apps/ios/AntirotCurrentTaskWidget/Sources/AntirotCurrentTaskWidget.swift`

**Interfaces:**
- Consumes: existing fetch/actions/navigation links and widget timeline entry.
- Produces: truthful editorial Stats report and flat ruled supporting screens.

- [ ] Add a failing test asserting zero work maps to zero goal progress.
- [ ] Remove the 5% floor, faux Settings/Developer rows, grid-card layout, and ambiguous loading/unavailable overlap.
- [ ] Build a serif hero metric, factual time composition, and ruled secondary metrics using only response values.
- [ ] Convert Settings and utilities to flat labeled sections with explicit semantic permission text; preserve every action/link/sheet.
- [ ] Redesign sign-in and widget using the same solid palette and no glass/card stack.
- [ ] Parse all changed Swift and verify all existing behavior entry points remain.
- [ ] Commit with `feat: complete ios editorial redesign`.

### Task 5: Remove waste, verify, review, and deploy

**Files:**
- Remove only after reference verification: unreferenced legacy Coach assets under `apps/ios/AntirotAlarm/Resources/Assets.xcassets/`.
- Modify: `Done.md` using partial staging so unrelated user edits remain untouched.

**Interfaces:**
- Consumes: complete editorial redesign.
- Produces: verified branch and successful TestFlight upload.

- [ ] Search production and test sources for all legacy asset names; remove only assets with zero remaining references.
- [ ] Validate every asset `Contents.json`, run Swift tree-sitter parse on every changed Swift file, and run repository diff/behavior invariants.
- [ ] Request an independent read-only code/design review and fix all Critical/Important issues.
- [ ] Add one manual verification line to `Done.md` and stage only that hunk.
- [ ] Commit with `docs: record editorial ios verification`.
- [ ] Push `codex/ios-warm-smoked-glass` to GitHub and the VPS bare remote.
- [ ] Manually dispatch `deploy-ios-testflight.yml` against the branch, capture the new run, and monitor the exact upload step:

```bash
gh workflow run deploy-ios-testflight.yml --repo mehulhere/Antirot --ref codex/ios-warm-smoked-glass
run_id="$(gh run list --repo mehulhere/Antirot --workflow deploy-ios-testflight.yml --branch codex/ios-warm-smoked-glass --limit 1 --json databaseId --jq '.[0].databaseId')"
npm run check:testflight-upload -- --run-id "$run_id" --repo mehulhere/Antirot --json
```

Expected: JSON reports `"status": "succeeded"` and `"message": "Upload to TestFlight succeeded."`.
