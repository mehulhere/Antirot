# iOS Warm Smoked-Glass Revamp Design

## Objective

Revamp the complete native iOS app with a warm smoked-glass visual language inspired by the supplied social-feed reference while preserving Antirot's strict coach identity, current navigation, backend contracts, runtime behavior, and accessibility.

## Approved Direction

Use warm graphite, smoked taupe, and muted bronze surfaces with layered translucency, hairline specular borders, compact editorial typography, and restrained depth. Antirot red remains the decisive action and active-state accent; amber, cyan, green, and danger red remain semantic colors only.

The UI must feel focused and premium rather than decorative. Glass is used to organize hierarchy and maintain context, not to obscure content. Text and controls remain readable over every background without relying on blur alone.

## Scope

The revamp covers:

- the app-wide design tokens and reusable surfaces;
- the bottom navigation shell;
- Coach/Home, including the coach stage, runtime controls, and chat sheet;
- Tasks, Stats, and Settings;
- supporting screens already reached from Settings or the primary tabs;
- the current-task widget where shared colors would otherwise look inconsistent;
- focused layout and design-token tests;
- a refreshed Imagegen-created coach-stage raster asset.

It does not change API payloads, authentication, task parsing, alarms, Screen Time behavior, chat behavior, navigation destinations, product copy, or runtime state transitions.

## Visual System

### Palette

- Canvas: near-black warm charcoal, approximately `#151311`.
- Deep canvas: `#0D0C0B` for contrast behind floating panes.
- Smoked surface: translucent graphite-taupe near `#3B3633`.
- Raised glass: warm gray near `#514A46` with low opacity over content.
- Primary text: warm white near `#F5F1EC`.
- Secondary text: stone gray near `#B6ADA6`.
- Muted text: warm gray near `#7C746F`.
- Primary accent: Antirot red `#E63946`.
- Semantic colors remain distinct and are not used as ornamental gradients.

### Material and Depth

- Reusable glass surfaces combine native SwiftUI material, a warm tint layer, a subtle top-left sheen, and a 0.5–0.8 pt light border.
- Major containers use 22–30 pt continuous corners; inner cards use 16–20 pt corners; controls use capsules where appropriate.
- Shadows are broad, dark, and low-opacity. Glows are limited to the active coach action or a live runtime state.
- Background ambience uses soft warm radial light and generated coach-stage imagery. It must not reduce contrast.

### Typography and Spacing

- Keep the system font and Dynamic Type support.
- Use rounded bold display type sparingly for screen titles and session metrics.
- Use compact semibold labels, uppercase kickers with tracking, and quieter metadata.
- Use an 8 pt spacing rhythm, with 20 pt standard horizontal screen padding.

## App Shell

Retain the four destinations: Coach, Tasks, Stats, and Settings. Replace the current evenly filled bar with a floating smoked-glass dock. Each destination keeps an icon and short label. The selected item receives a warm translucent capsule, white foreground, and a small red indicator; inactive items remain quiet but pass contrast requirements. Selection continues to use the existing spring animation and respects Reduce Motion.

## Coach/Home

The Coach screen remains immersive and full-screen. The generated stage art becomes a warm, editorial training-room portrait with dark negative space for UI, no text, and no embedded controls. A gradient and tint treatment ensures consistent contrast across device sizes.

The top header becomes a compact glass status island with the Coach title, connection state, and current runtime state. The primary action remains dominant and state-driven. Secondary actions use quiet glass capsules. The action cluster must remain above the chat sheet at every existing detent.

The chat sheet uses the shared smoked-glass material with a clear drag handle, improved message grouping, warm input surface, and explicit send/record states. Existing gestures, detents, voice playback, send behavior, and keyboard handling remain unchanged.

## Tasks

Tasks use an editorial board hierarchy:

- a compact header and summary strip;
- the active or highest-priority task presented as the strongest raised pane;
- remaining items as quieter glass rows with clear completion, urgency, and metadata;
- existing task actions and parsing preserved.

The design must remain scannable with long task names and large text sizes.

## Stats

Stats use a concise performance-report layout:

- primary metrics in a balanced two-column grid where width allows;
- a wider narrative or trend pane for context;
- semantic color used only to communicate state;
- no decorative charting that implies data not already available.

Existing values and calculations are unchanged.

## Settings and Supporting Screens

Settings use grouped smoked-glass sections with clearer category headers, consistent control rows, and restrained destructive styling. Developer, diagnostics, memory, alarm, and account destinations remain available. Forms retain native behavior and keyboard accessibility. Supporting screens adopt the same backdrop, header, glass card, and row vocabulary so there is no visual break when navigating deeper.

## Generated Asset

Use the built-in Imagegen path to create one project-bound raster asset for the Coach stage. The supplied image is a style/composition reference only. The prompt must request:

- a vertical iPhone background;
- an austere coach in a shadowed training room or study;
- warm taupe, charcoal, muted bronze, and restrained red light;
- soft cinematic depth and subtle architectural panels;
- generous dark negative space in the upper-left and lower UI zones;
- no text, logos, interface elements, or watermark.

The generated output is inspected before use, copied into the existing asset catalog under a new non-destructive name, and referenced by `CoachStage` with contrast overlays.

## Architecture and Components

- `AntirotDesign.swift` owns the canonical warm palette, material recipes, backdrop, cards, headers, rows, and accessibility-aware motion helpers.
- `MainTabView.swift` owns the floating app dock and selection behavior.
- `HomeView.swift`, `GlassSheet.swift`, and `PrimaryActionButton.swift` compose Coach-specific surfaces from shared primitives.
- `TaskBoardView.swift`, `StatsView.swift`, and `SettingsView.swift` keep their existing feature logic and adopt the shared components.
- The asset catalog owns the new generated coach-stage image.
- Tests validate stable metrics, palette values exposed as testable constants where practical, and critical layout invariants without snapshotting nondeterministic pixels.

No third-party UI framework is required. Native SwiftUI materials and shapes are sufficient and minimize compatibility risk.

## Data Flow and Behavior Preservation

Views continue to consume the existing environment objects and models. The revamp does not introduce a new state container. User interactions continue to call the same actions, async tasks, API client methods, and navigation destinations. The visual layer may derive presentation-only labels and colors from existing state but must not reinterpret backend behavior.

## Error and Fallback Behavior

Existing connection, send, recording, and API error states remain visible. A material or image failure must degrade to the warm solid-color backdrop while preserving legibility and interaction. New runtime fallback logging, if required, must use the repository's explicit `🔴 FALLBACK` format. No fallback may silently remove a control or state indicator.

## Accessibility and Adaptation

- Preserve Dynamic Type and avoid fixed-height text containers where content can grow.
- Maintain at least 44×44 pt interactive targets.
- Provide sufficient contrast on glass with explicit tint/overlay layers.
- Respect Reduce Motion for selection, pulses, and ambient effects.
- Keep VoiceOver labels and logical reading order for navigation and controls.
- Support current iPhone targets, safe areas, keyboard presentation, and existing chat detents.

## Verification

- Add focused tests for changed layout constants and component behavior.
- Run the relevant iOS unit tests available in the project configuration.
- Run `npx eslint <changed-files>` only for changed JavaScript or TypeScript files; none are expected.
- Run `npx tsc --noEmit` only if TypeScript files change; none are expected.
- Generate the Xcode project and perform the strongest locally available iOS build/type check.
- Inspect the generated image and rendered SwiftUI previews or simulator screenshots where the environment supports them.
- Add one crisp manual verification line to `Done.md` covering all four tabs, chat detents, Dynamic Type, and Reduce Motion.

## Acceptance Criteria

- Every primary iOS screen visibly shares the approved warm smoked-glass system.
- The result is recognizably inspired by the reference without copying its social-feed content or layout literally.
- Antirot red remains purposeful and sparse.
- Existing app flows and backend integration compile without behavioral changes.
- The Coach screen uses the new Imagegen-created project asset.
- Navigation, long content, keyboard/chat interactions, and accessibility settings remain usable.
- Relevant tests and the strongest available build verification pass, or any environment-only limitation is reported precisely.
