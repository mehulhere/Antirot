# iOS Warm Smoked-Glass Revamp Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Revamp every primary Antirot iOS screen with the approved warm smoked-glass design while preserving existing navigation, state, and backend behavior.

**Architecture:** Keep the native SwiftUI feature views and environment-object data flow intact. Centralize the new visual language in `AntirotDesign.swift`, let the shell and feature screens compose those primitives, and use one new generated coach-stage raster asset behind accessibility-safe overlays.

**Tech Stack:** Swift 5.10, SwiftUI, UIKit bridge already used by the chat sheet, XCTest, XcodeGen, built-in Imagegen.

## Global Constraints

- Deployment target remains iOS 17.0 and target family remains iPhone.
- Do not change API payloads, authentication, runtime state transitions, alarms, Screen Time, or chat behavior.
- Use native SwiftUI; add no third-party UI framework.
- Preserve Dynamic Type, VoiceOver reading order, safe areas, keyboard behavior, and minimum 44×44 pt targets.
- Respect Reduce Motion for newly changed animation and ambient effects.
- Keep Antirot red sparse and decisive; semantic colors communicate state only.
- Do not overwrite existing generated assets; add a new asset name.

---

### Task 1: Lock the warm design tokens and layout invariants

**Files:**
- Modify: `apps/ios/AntirotAlarmTests/Sources/CinematicLayoutTests.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/AntirotDesign.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/MainTabView.swift`

**Interfaces:**
- Produces: `AntirotPaletteValues`, updated `AntirotCinematicMetrics`, `SmokedGlassModifier`, `smokedGlass(cornerRadius:tint:shadow:)`, and updated `AppBottomBarMetrics`.
- Consumes: existing `Color.ar*` aliases and `AppScreen` destinations.

- [ ] **Step 1: Write failing invariant tests**

Add assertions that the palette exposes warm canvas values, card/pill radii are 22/28, the bottom dock reserves enough coach clearance, and all four destinations remain present:

```swift
func testSmokedGlassUsesApprovedWarmPalette() {
    XCTAssertEqual(AntirotPaletteValues.backgroundRed, 0.082, accuracy: 0.001)
    XCTAssertGreaterThan(AntirotPaletteValues.surfaceRed, AntirotPaletteValues.surfaceBlue)
}

func testSmokedGlassUsesGenerousContinuousCorners() {
    XCTAssertEqual(AntirotCinematicMetrics.cardRadius, 22, accuracy: 0.1)
    XCTAssertEqual(AntirotCinematicMetrics.pillRadius, 28, accuracy: 0.1)
}
```

- [ ] **Step 2: Run the focused tests and confirm failure**

Run the generated-project test command if Xcode is available. Otherwise use `swiftc`/source inspection only as a temporary diagnostic and retain the failing-test intent for CI.

- [ ] **Step 3: Implement the shared system and dock**

Introduce numeric palette constants that tests can inspect, point the canonical colors at them, update backdrop/card/header/row primitives, and implement the selected dock capsule:

```swift
enum AntirotPaletteValues {
    static let backgroundRed = 0.082
    static let backgroundGreen = 0.075
    static let backgroundBlue = 0.067
}

extension View {
    func smokedGlass(
        cornerRadius: CGFloat = AntirotCinematicMetrics.cardRadius,
        tint: Color = .arGlassTint,
        shadow: Bool = true
    ) -> some View {
        modifier(SmokedGlassModifier(cornerRadius: cornerRadius, tint: tint, castsShadow: shadow))
    }
}
```

Use `@Environment(\.accessibilityReduceMotion)` in the app shell and replace the spring with an ease animation when motion is reduced.

- [ ] **Step 4: Run focused tests and source-format checks**

Expected: palette, radius, clearance, and tab-list tests pass; `git diff --check` reports no whitespace errors.

- [ ] **Step 5: Commit the shared visual system**

```bash
git add apps/ios/AntirotAlarm/Sources/AntirotDesign.swift apps/ios/AntirotAlarm/Sources/MainTabView.swift apps/ios/AntirotAlarmTests/Sources/CinematicLayoutTests.swift
git commit -m "feat: add ios smoked glass design system"
```

### Task 2: Generate and integrate the warm Coach stage

**Files:**
- Create: `apps/ios/AntirotAlarm/Resources/Assets.xcassets/AntirotCoachStageWarm.imageset/coach-stage-warm.png`
- Create: `apps/ios/AntirotAlarm/Resources/Assets.xcassets/AntirotCoachStageWarm.imageset/Contents.json`
- Modify: `apps/ios/AntirotAlarm/Sources/CoachStage.swift`
- Modify: `apps/ios/AntirotAlarmTests/Sources/CinematicLayoutTests.swift`

**Interfaces:**
- Produces: asset name `AntirotCoachStageWarm` through `CoachStageLayoutMetrics.backgroundAssetName`.
- Consumes: existing `CoachEmotion`, `isThinking`, and SwiftUI `Image` asset loading.

- [ ] **Step 1: Change the asset-name test first**

```swift
func testCoachStageUsesWarmGeneratedBackgroundAsset() {
    XCTAssertEqual(CoachStageLayoutMetrics.backgroundAssetName, "AntirotCoachStageWarm")
}
```

- [ ] **Step 2: Confirm the focused test fails against the old asset name**

Expected: the test reports `AntirotCoachStage` instead of `AntirotCoachStageWarm`.

- [ ] **Step 3: Generate and inspect the image**

Use built-in Imagegen with the approved spec: vertical iPhone coach-stage background, austere coach, warm shadowed training room, taupe/charcoal/bronze palette, restrained red practical light, architectural panels, dark upper-left and lower negative space, no text, logo, UI, or watermark. Inspect the output before copying it into the asset catalog.

- [ ] **Step 4: Integrate the new asset and contrast overlays**

Set `backgroundAssetName` to `AntirotCoachStageWarm`, retain `scaledToFill`, and add warm top/bottom gradients that do not intercept hit testing. Create a valid universal 1x imageset JSON.

- [ ] **Step 5: Run the asset test and validate catalog metadata**

Run `plutil -lint` or `python -m json.tool` on `Contents.json`; expected valid JSON and passing asset-name test.

- [ ] **Step 6: Commit the Coach stage**

```bash
git add apps/ios/AntirotAlarm/Sources/CoachStage.swift apps/ios/AntirotAlarm/Resources/Assets.xcassets/AntirotCoachStageWarm.imageset apps/ios/AntirotAlarmTests/Sources/CinematicLayoutTests.swift
git commit -m "feat: add warm coach stage artwork"
```

### Task 3: Revamp Coach controls and chat sheet

**Files:**
- Modify: `apps/ios/AntirotAlarm/Sources/HomeView.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/PrimaryActionButton.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/GlassSheet.swift`
- Modify: `apps/ios/AntirotAlarmTests/Sources/ChatSheetDetentsTests.swift`

**Interfaces:**
- Consumes: `CoachStateActions.actions(for:)`, existing `GlassSheet` callbacks, existing detent functions.
- Produces: compact Coach status island, smoked action controls, and smoked chat presentation without callback or detent signature changes.

- [ ] **Step 1: Add failing presentation-invariant tests**

Assert the collapsed height, full fraction, two-detent behavior, and 44 pt minimum control dimension remain stable. Add a testable `ChatSheetMetrics.minimumControlSize` constant equal to 44.

- [ ] **Step 2: Confirm the new constant test fails**

Expected: compile failure because `ChatSheetMetrics` is not defined.

- [ ] **Step 3: Implement the Coach UI changes**

Replace the loose top header with a compact smoked-glass island, keep the action stack above every chat detent, restyle primary/secondary actions, and use `smokedGlass` for collapsed and expanded chat. Keep all send, record, playback, keyboard, UIKit pan, and snap code intact.

- [ ] **Step 4: Respect Reduce Motion**

Use `accessibilityReduceMotion` to disable repeating mic/state pulses and replace newly touched spring transitions with short ease animations.

- [ ] **Step 5: Run chat and cinematic layout tests**

Expected: detent behavior, asset, clearance, and target-size tests pass.

- [ ] **Step 6: Commit the Coach surface**

```bash
git add apps/ios/AntirotAlarm/Sources/HomeView.swift apps/ios/AntirotAlarm/Sources/PrimaryActionButton.swift apps/ios/AntirotAlarm/Sources/GlassSheet.swift apps/ios/AntirotAlarmTests/Sources/ChatSheetDetentsTests.swift
git commit -m "feat: revamp ios coach experience"
```

### Task 4: Revamp Tasks and Stats

**Files:**
- Modify: `apps/ios/AntirotAlarm/Sources/TaskBoardView.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/StatsView.swift`
- Test: `apps/ios/AntirotAlarmTests/Sources/TaskBoardParserTests.swift`

**Interfaces:**
- Consumes: existing task arrays, `scopedItems`, parser output, stats responses, and action closures.
- Produces: editorial task hierarchy and performance-report composition using shared glass primitives.

- [ ] **Step 1: Run parser tests before visual edits**

Expected: current parser tests pass, establishing that subsequent changes remain presentation-only.

- [ ] **Step 2: Implement the Tasks hierarchy**

Keep scope selection and action wiring. Present the first scoped item as the strongest pane, remaining items as quiet rows, retain the focus summary, and replace decorative quote emphasis with restrained editorial treatment. Long titles must use flexible vertical sizing.

- [ ] **Step 3: Implement the Stats report**

Keep all fetch and summarize methods. Restyle scope selection, metric grid, week bars, summary, and rows using warm surfaces and truthful existing values only.

- [ ] **Step 4: Re-run parser tests and diff checks**

Expected: parser behavior unchanged and `git diff --check` clean.

- [ ] **Step 5: Commit Tasks and Stats**

```bash
git add apps/ios/AntirotAlarm/Sources/TaskBoardView.swift apps/ios/AntirotAlarm/Sources/StatsView.swift
git commit -m "feat: revamp ios tasks and stats"
```

### Task 5: Revamp Settings, supporting screens, and widget

**Files:**
- Modify: `apps/ios/AntirotAlarm/Sources/SettingsView.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/AlarmsView.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/PlanView.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/MemoryFilesView.swift`
- Modify: `apps/ios/AntirotAlarm/Sources/MemorySnapshotsView.swift`
- Modify: `apps/ios/AntirotCurrentTaskWidget/Sources/AntirotCurrentTaskWidget.swift`

**Interfaces:**
- Consumes: existing NavigationLinks, bindings, permission actions, alarm actions, memory APIs, and widget entry.
- Produces: consistent smoked sections and warm widget palette without behavior changes.

- [ ] **Step 1: Capture the current supporting-screen entry points**

Use exact source search to verify every existing Settings navigation destination remains present before editing.

- [ ] **Step 2: Restyle Settings groups and controls**

Compose account, permissions, system information, and developer tools with `CinematicGlassCard`, shared rows, and warm form fields. Keep logout/destructive controls explicit and preserve all sheets/alerts.

- [ ] **Step 3: Apply shared presentation to supporting screens**

Replace legacy flat backgrounds/cards with `CinematicBackdrop`, `CinematicHeader`, `CinematicGlassCard`, or `smokedGlass` as appropriate. Do not change async methods or model transforms.

- [ ] **Step 4: Align the widget palette**

Update only the widget's background, foreground hierarchy, borders, and accent tint. Keep timeline behavior and task state unchanged.

- [ ] **Step 5: Verify entry-point preservation and diff quality**

Expected: source search still finds Memory Files, Memory Snapshots, permissions, diagnostics, logout, alarm, and plan actions; `git diff --check` is clean.

- [ ] **Step 6: Commit supporting surfaces**

```bash
git add apps/ios/AntirotAlarm/Sources/SettingsView.swift apps/ios/AntirotAlarm/Sources/AlarmsView.swift apps/ios/AntirotAlarm/Sources/PlanView.swift apps/ios/AntirotAlarm/Sources/MemoryFilesView.swift apps/ios/AntirotAlarm/Sources/MemorySnapshotsView.swift apps/ios/AntirotCurrentTaskWidget/Sources/AntirotCurrentTaskWidget.swift
git commit -m "feat: revamp ios supporting surfaces"
```

### Task 6: Full verification and manual handoff

**Files:**
- Modify: `Done.md`

**Interfaces:**
- Consumes: the complete revamped app.
- Produces: verification evidence and one crisp manual-check instruction.

- [ ] **Step 1: Generate the Xcode project**

Run:

```bash
cd apps/ios
xcodegen generate
```

Expected: project generation succeeds.

- [ ] **Step 2: Run the strongest locally available build and tests**

Prefer an unsigned simulator build/test destination if Xcode is available:

```bash
xcodebuild -project apps/ios/Antirot.xcodeproj -scheme Antirot -sdk iphonesimulator -destination 'platform=iOS Simulator,name=iPhone 16 Pro' CODE_SIGNING_ALLOWED=NO test
```

If the named simulator is unavailable, list installed destinations and use an available iPhone simulator. If Xcode is unavailable on the host, run source/config validation and report the limitation precisely.

- [ ] **Step 3: Validate assets and repository hygiene**

Run JSON validation for all changed `Contents.json` files, `git diff --check`, and inspect `git status --short` to ensure unrelated pre-existing changes were not staged or overwritten.

- [ ] **Step 4: Record manual verification**

Append one line to `Done.md`: verify Coach, Tasks, Stats, Settings, chat detents/keyboard, Dynamic Type, VoiceOver order, and Reduce Motion on an iPhone simulator or TestFlight device.

- [ ] **Step 5: Review the final diff against the design spec**

Confirm every acceptance criterion in `docs/superpowers/specs/2026-07-10-ios-warm-smoked-glass-design.md` is covered and no backend or TypeScript file was changed by this work.

- [ ] **Step 6: Commit verification documentation**

```bash
git add Done.md docs/superpowers/plans/2026-07-10-ios-warm-smoked-glass.md
git commit -m "docs: record ios revamp verification"
```
