# Antirot iOS Design Audit Evidence

## Structural Evidence

- Coach exposes 11 visible controls in its ordinary collapsed state, plus one invisible full-screen swipe surface; Tasks and Stats each expose nine targets; Settings exposes nine collapsed and fifteen when Developer is expanded. Sources: `MainTabView.swift:23-44,73-129`, `HomeView.swift:29-66,119-139`, `GlassSheet.swift:180-258`, `TaskBoardView.swift:20-73,220-245`, `StatsView.swift:20-75,95-124`, `SettingsView.swift:20-205`.
- Maximum primary-tree nesting is eight levels, reached in Settings and expanded chat. Sources: `MainTabView.swift:137-149`, `AntirotDesign.swift:260-269`, `SettingsView.swift:20-176`, `HomeView.swift:29-66`, `GlassSheet.swift:262-329`.
- Five repeated-purpose pattern groups recur: bottom navigation, three-way segmented controls, pull-to-refresh, glass title shells, and duplicate diagnostics copy actions. Sources: `MainTabView.swift:73-129`, `TaskBoardView.swift:44-73`, `StatsView.swift:69-124`, `AntirotDesign.swift:254-304`, `SettingsView.swift:166-176,231-237`.
- Tasks has six sequential top-level content blocks, including overlapping overview/summary information and a quote card. Stats contains two chevron rows that are not interactive. Sources: `TaskBoardView.swift:25-32,75-85,187-245`, `StatsView.swift:236-262`.
- Coach exposes several overlapping chat-opening affordances: full-screen swipe, drag/tap handle, prompt button, arrow button, and parent tap gesture. Sources: `HomeView.swift:42-45`, `GlassSheet.swift:180-258`.
- No demonstrably unused imports or dead component props were found, but `showFullError` appears never to become true. Source: `SettingsView.swift:10,213-217`.

## Visual Evidence

- Source spacing values span `[0,2,3,4,5,6,7,8,10,11,12,13,14,16,18,20,22,24,30,64,104,126]`, which is broader than a disciplined token scale. Sources: `AntirotDesign.swift:217-223,254-327` and primary-screen usages.
- Typography mixes fixed sizes `[13,15,16,17,19,22,28,29,30,32]` with semantic styles. Sources: `AntirotDesign.swift:282-371`, `HomeView.swift:96-103`, `StatsView.swift:134-143`, `TaskBoardView.swift:117-126`.
- The design source references 18 base UI colors before material/photo compositing. Source: `AntirotDesign.swift:23-64,88-108`.
- Estimated token contrast includes muted-on-elevated at 1.90:1, muted-on-surface at 2.60:1, and accent-on-surface at 2.86:1. Likely failures occur in Settings developer captions. Sources: `AntirotDesign.swift:34-64`, `SettingsView.swift:157-204,280-287`.
- Current screenshots show nearly identical corner radius, fill, and elevation applied to headers, segments, metrics, content, chat, and navigation. Surface differentiation therefore does little to communicate hierarchy.
- The Stats screenshot shows the bottom navigation covering the Developer row; the Tasks screenshot shows the floating add button competing for the narrow gap above navigation.
- Coach places a header, two secondary pills, a primary action, chat sheet, and bottom navigation over the portrait, leaving the artwork and state action in competition rather than figure/ground hierarchy.

## State and Accessibility Evidence

- Empty, error, success, and disabled states exist. Loading is inconsistent: Stats renders “Stats unavailable” while the initial request is still loading. Sources: `TaskBoardView.swift:140-149`, `StatsView.swift:8-12,61-67,268-297`, `GlassSheet.swift:300-312`, `PrimaryActionButton.swift:33-41`.
- Custom keyboard-focus visuals and a disabled-state visual token are not defined. Selection styling exists but is not keyboard focus. Sources: `GlassSheet.swift:340-360`, `MainTabView.swift:105-127`, `TaskBoardView.swift:44-72`.
- CoachStage is announced as “Antirot coach” before actionable controls instead of being hidden as decorative. Source: `CoachStage.swift:48`.
- The chat handle has a label/hint but no explicit accessibility action for open/collapse, while a gesture-only full-screen swipe also opens chat. Sources: `GlassSheet.swift:180-212`, `HomeView.swift:42-45`.
- Reduce Motion is respected by Coach, tab, segment, and recording animations. Sources: `HomeView.swift:18,100`, `MainTabView.swift:19,91-94`, `GlassSheet.swift:396-401`.

## Copy and Honesty Evidence

- “Backend connected” is unconditional despite failed/unknown runtime fetch states. Sources: `HomeView.swift:93-104`, `CoachViewModel.swift:69-90`.
- Stats floors the goal ring at 5%, so zero work renders as 5%. Source: `StatsView.swift:148-159,264-266`.
- Task focus time is inferred from arbitrary digits, assumes every done task is 45 minutes, and uses 30/45/120-minute fallbacks while presenting the result as Focus against a fixed four-hour goal. Source: `TaskBoardView.swift:151-216,310-345`.
- Stats Settings/Developer rows use chevrons but have no action. Source: `StatsView.swift:236-262`.
- The Tasks add button only fills a hidden coach draft and neither navigates nor opens chat; it is also icon-only. Source: `TaskBoardView.swift:231-243`.
- “Behavioral operating system,” “Execute. No drift,” “Measure what matters,” “Live,” “Check In,” and “Unproductive desk” are inflated, vague, or domain-dependent labels. Sources: `LoginView.swift:41`, `TaskBoardView.swift:20-25,75-97`, `StatsView.swift:19-24,197-205`, `StateActions.swift:32-54`.
- No commercial dark pattern was found. The coach’s belittling onboarding language is pressure-based rather than deceptive, but can create shame friction. Sources: `CoachViewModel.swift:6-10`, `HomeView.swift:264-273`.

## Weight and Attention Evidence

- Main-target raw resources total 5,544,013 bytes before build compression. Two coach-stage images account for roughly 2.92 MB; only `AntirotCoachStageWarm` is referenced in production Swift. Sources: `CoachStage.swift:3-20`, asset inventory under `apps/ios/AntirotAlarm/Resources/`.
- Authenticated Coach launch authors one or two initial requests depending on configuration order: pending alarms and runtime state, with possible conditional APNs registration. Sources: `HomeView.swift:70-75`, `AlarmCenter.swift:76-88`, `APIClient.swift:107-159`, `AntirotApp.swift:17-61`.
- The first frame does not await network, but settled data requires sequential request latency. Runtime TTI needs Instruments/XCTest measurement.
- One continuous idle pulse exists on the Coach status dot and is Reduce Motion-gated. Sources: `HomeView.swift:99-103`, `AntirotDesign.swift:419-436`.
- No automatic permission prompt appears on launch. A conditional onboarding-name modal can appear while state is still unknown or after refresh failure. Sources: `AntirotApp.swift:17-19`, `AlarmCenter.swift:27-44`, `HomeView.swift:79-87,246-261`.

## Known Gaps

- No macOS simulator, Accessibility Inspector, Instruments trace, archived IPA thinning report, or runtime contrast measurement was available.
- Current screenshots were user supplied and are treated as representative of the TestFlight branch, not independently timestamp-verified.
- Dynamic backend strings and message-dependent voice controls make exact runtime counts conditional.

