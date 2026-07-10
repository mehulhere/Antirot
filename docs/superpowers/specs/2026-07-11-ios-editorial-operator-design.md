# Antirot iOS Editorial Operator Redesign

## Objective

Replace the current warm smoked-glass UI with a disciplined editorial system that makes Antirot feel designed by an expert: decisive hierarchy, minimal chrome, truthful data, and one obvious next action per screen.

## Approved Identity

Antirot keeps a realistic human Coach. The Coach appears as a composed, demanding professional—not a mascot, celebrity, drill sergeant, or generic stock-photo executive. The person is embedded in a cinematic environment with strong black negative space and a restrained burnt-orange light field.

## Visual Concept: Editorial Operator

The references contribute five principles:

1. Large editorial typography establishes hierarchy before containers.
2. Negative space is an active structural element.
3. Cinematic imagery carries atmosphere without decorative UI effects.
4. Navigation is compact, rectangular, and subordinate.
5. One accent color creates focus; it does not decorate every component.

This is not a marketing-page port. Mobile controls remain native, accessible, and task-oriented.

## Design Tokens

### Color

- Ink: `#080807` — primary canvas.
- Graphite: `#171714` — the single raised conversation/action surface.
- Paper: `#F0ECE2` — primary text and light actions.
- Ash: `#8A877F` — secondary text.
- Rule: `#34322E` — separators and quiet borders.
- Signal Orange: `#E45B2C` — primary action, selected state, and true urgency.
- Success: `#48C774`; Warning: `#E8A33C`; Danger: `#E5484D` — semantic use only.

No translucent material, glass tint, decorative glow, or multicolor ornamental gradient appears in interface chrome.

### Typography

- Display: native system serif design for screen titles and hero metrics.
- Body: native system sans serif for instructions and controls.
- Operational metadata: native monospaced design, uppercase, tracked, used sparingly.
- Semantic Dynamic Type styles are required; fixed point sizes are avoided for user-facing text.
- Maximum hierarchy per screen: one display title, one hero value/action, one body tier, one metadata tier.

### Spacing and Shape

- Spacing scale: 4, 8, 12, 20, 32 points.
- Standard horizontal inset: 20 points.
- Section separation uses 1-point rules and whitespace.
- Radius scale: 0, 4, 12, 16 points. Large 22–30 point card radii are removed.
- Shadows are absent from standard surfaces. The Coach image may use photographic depth.

## App Shell

The app retains Coach, Tasks, Stats, and Settings. The large floating rounded dock is replaced by a flat bottom safe-area bar with a hairline top rule. Each destination uses a compact icon and label. Only the selected destination uses Paper text and a 2-point orange underline; inactive destinations use Ash.

Content uses safe-area inset rather than being covered by the dock. Selection animation is a short opacity/position change and respects Reduce Motion.

## Coach

- The Coach image is full-bleed and newly generated for this direction.
- The top-left contains a quiet monospaced status line derived from real connectivity/runtime state; it is not enclosed in a card.
- The current state or Coach prompt is the primary typographic statement.
- One dominant action sits near the bottom as a rectangular Paper or Signal Orange control. It uses the existing state-driven action mapping.
- Secondary actions are plain text buttons separated by vertical rules.
- The collapsed chat composer is the only raised UI surface on the screen: a compact Graphite bar with mic, concise prompt, and send/open affordance.
- Expanded chat becomes an opaque full-screen conversation surface with a strong typographic header, flat message rhythm, and no floating bubble wall.
- Redundant full-screen swipe/tap affordances are reduced to the visible composer and an accessible drag/tap handle.

## Tasks

- The screen begins with a large serif title and a plain underlined Today/Upcoming/Backlog switcher.
- Live/Pending/Done counts appear as one horizontal metadata line, not a card.
- The active or next task is the single highlighted orange/ivory block.
- Remaining tasks are flat rows separated by rules; long titles grow vertically.
- Recorded work and estimates are labeled distinctly. Invented 30/45/120-minute fallbacks are removed from displayed Focus totals.
- The motivational quote card is removed.
- “Add task” is a labeled action. It sets the draft and navigates to Coach rather than silently changing hidden state.

## Stats

- The screen reads like a performance report, not a dashboard grid.
- A large serif recorded-focus value is the hero.
- Zero renders as zero; no minimum ring percentage is applied.
- Work, Idle, and At desk/off task use backend-provided minutes only.
- Check-ins and completed tasks appear as secondary typographic figures separated by rules.
- Noninteractive Settings/Developer chevron rows are removed.
- Loading, unavailable, and error states are visually and semantically distinct.

## Settings and Supporting Screens

- Settings uses flat labeled sections with hairline rules and generous whitespace.
- Account, permissions, device/server, and developer disclosure retain existing behavior.
- Permission states use a text label plus semantic status; color is not the only indicator.
- Developer content is visually quiet and remains collapsed by default.
- Sign-in uses the same editorial identity with one strong action and the realistic Coach imagery or a derived crop.
- Memory, alarm, plan, diagnostics, and widget surfaces adopt the same tokens without adding card stacks.

## Generated Asset

Use built-in Imagegen to create a new project-bound portrait asset:

- realistic human performance coach;
- black architectural studio or night interior;
- burnt-orange illuminated sculptural wall or landscape-like light field;
- coach positioned in the lower-right/middle-right;
- large clean black negative space in the upper-left and lower interface zones;
- editorial photography, subtle grain, natural anatomy;
- no text, logos, UI, neon cyberpunk, glass panels, or watermark.

The asset receives a new non-destructive imageset name. Obsolete unreferenced Coach-stage variants are removed only after source and catalog validation.

## Honesty and Copy

- Connectivity copy is derived from actual request state.
- “Behavioral operating system” becomes “Accountability coach.”
- “Live” becomes “In progress.”
- “Measure what matters” becomes “Recorded focus and completed work.”
- Internal/backend jargon remains inside Developer sections.
- Any estimate is labeled “Estimated.” Any recorded value is labeled “Recorded.”
- No decorative quote is attributed to Antirot.

## Accessibility

- Dynamic Type uses semantic styles throughout.
- All controls are at least 44×44 points.
- Decorative Coach imagery is hidden from VoiceOver; meaningful state is read as text.
- Chat open/collapse has explicit accessibility actions.
- Contrast meets WCAG AA after removing material compositing.
- Reduce Motion and Reduce Transparency remain honored.
- Loading, error, empty, success, focus, and disabled states have distinct labels and visual treatment.

## Architecture

- `AntirotDesign.swift` becomes the editorial token/component source and removes glass modifiers from active use.
- `MainTabView.swift` owns the flat safe-area navigation and a small shared `AppNavigationModel` so Tasks can route to Coach.
- Feature views preserve current environment objects and async calls.
- `CoachStage.swift` owns the new generated asset and contrast crop.
- No third-party UI dependency is introduced.

## Verification

- Test-first layout/token invariants cover palette, radius ceiling, navigation height, minimum targets, truthful zero ratio, and asset name.
- Swift syntax is parsed locally.
- The macOS TestFlight action must generate the project, compile the app, compile tests, archive, export, and upload successfully.
- Manual TestFlight review checks all four tabs, sign-in, expanded chat, Dynamic Type, VoiceOver order, Reduce Motion, keyboard behavior, and small-screen content clearance.

## Acceptance Criteria

- No primary screen is composed as a stack of equal rounded cards.
- No navigation or action overlaps content.
- Each primary screen has one obvious visual story and one dominant action or metric.
- The realistic Coach remains central without competing chrome.
- Displayed data and affordances are truthful.
- The result visibly reflects the supplied editorial references through hierarchy, negative space, typography, and controlled imagery.

