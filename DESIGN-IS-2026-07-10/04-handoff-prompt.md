# Redesign Planning Handoff

```text
/make-plan Redesign the complete Antirot native iOS experience. Current design failed audit at 10/30 with critical gaps in principles #3 aesthetic, #4 understandable, #5 unobtrusive, #6 honest, #7 long-lasting, #8 thorough, and #10 as little design as possible.

Verdict paragraph:
> The current Antirot iOS design scores 10/30 and requires a structural redesign because chrome dominates content, several controls misrepresent behavior or data, and the product lacks a clear editorial hierarchy.

Why redesign and not refine: the total is below the 20-point threshold and the load-bearing unobtrusive and minimal-design principles scored zero.

Preserve from current design:
- Native SwiftUI environment-object architecture and state/action wiring in HomeView.swift:29-88 and MainTabView.swift:23-44.
- Antirot red as a sparse semantic action accent from AntirotDesign.swift:45-48.
- The full-screen Coach artwork concept and generated-asset pipeline in CoachStage.swift:3-49.
- Four destinations—Coach, Tasks, Stats, Settings—from MainTabView.swift:48-70.

Discard:
- Nested equal-weight smoked-glass cards. Evidence: AntirotDesign.swift:254-327 and current screenshots. Caused failure on principles #3, #5, and #10.
- Persistent oversized glass headers and bottom chrome that occludes content. Evidence: MainTabView.swift:39-43,73-129 and Stats screenshot. Caused failure on principles #5 and #8.
- Duplicate/faux surfaces: Tasks overview plus summary plus quote, Stats noninteractive chevron rows, redundant chat-open affordances. Evidence: TaskBoardView.swift:25-32,75-85,187-245; StatsView.swift:236-262; GlassSheet.swift:180-258. Caused failure on principles #4 and #10.

Top moves:
1. Principle #5 — Unobtrusive: Replace the nested glass-card stack with one edge-to-edge editorial canvas per screen; use whitespace, rules, and type scale for grouping, reserving a single elevated surface for the current action or active conversation. Evidence: DESIGN-IS-2026-07-10/01-evidence.md#visual-evidence.
2. Principle #10 — As little design as possible: Remove duplicate summaries, the motivational quote card, faux Stats navigation rows, redundant chat-opening affordances, and ornamental status containers. Evidence: DESIGN-IS-2026-07-10/01-evidence.md#structural-evidence.
3. Principle #4 — Understandable: Make every chevron/button perform its visible promise, give icon-only actions labels, derive connectivity from real state, and rewrite vague/internal labels in plain task language. Evidence: DESIGN-IS-2026-07-10/01-evidence.md#copy-and-honesty-evidence.
4. Principle #6 — Honest: Render zero as zero, distinguish estimated from recorded time, remove invented task durations and fixed goals, and show explicit loading/error/offline states. Evidence: DESIGN-IS-2026-07-10/01-evidence.md#copy-and-honesty-evidence.
5. Principle #3 — Aesthetic: Establish a strict 4/8/12/20/32 spacing scale, a small semantic type system with one editorial display face, three surface levels maximum, and a narrow black/ivory/burnt-orange palette inspired by the references. Evidence: DESIGN-IS-2026-07-10/01-evidence.md#visual-evidence.

Redesign principles in priority order:
1. Principle #2 — Useful: the next meaningful action is the dominant element and reachable in one tap.
2. Principle #4 — Understandable: every visible affordance predicts its behavior without explanation.
3. Principle #10 — As little design as possible: every screen has one primary story, no duplicate summaries, and at most one elevated content surface.

Deliverables:
- New information architecture, not derived from the current card stack.
- New primary flow with low-fidelity labeled comparisons to current.
- Exact screen-by-screen composition for Coach, Tasks, Stats, Settings, sign-in, chat, supporting screens, and widget.
- Token specification: spacing, type, color, rules, imagery, motion, accessibility, loading/error/empty/success/disabled/focus states.
- Migration path and cutover criteria for removing the old presentation while preserving behavior.
- Imagegen brief for any replacement stage or texture asset.

Anti-patterns:
- Porting the old card structure under new colors.
- Adding decorative glass, gradients, or texture without information purpose.
- Copying desktop marketing-page navigation or hero copy into the mobile app.
- Keeping both designs behind a flag indefinitely.
- Treating preserved state/action architecture as permission to preserve the current hierarchy.
```
