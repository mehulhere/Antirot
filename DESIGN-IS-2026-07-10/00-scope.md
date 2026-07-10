# Antirot iOS Design Audit Scope

## Audited Product

The complete native iOS surface on branch `codex/ios-warm-smoked-glass`, including Coach, Tasks, Stats, Settings, sign-in, chat, supporting utility screens, and the current-task widget.

## Inputs

- User-provided screenshots of the current TestFlight build showing Coach, Tasks, Stats, and Settings.
- User-provided editorial references showing high-contrast dark landing pages, large typography, generous negative space, restrained navigation, cinematic orange imagery, monochrome cyber texture, and airy image-led composition.
- SwiftUI source under `apps/ios/AntirotAlarm/Sources/` and widget source under `apps/ios/AntirotCurrentTaskWidget/Sources/`.

## Primary User and Task

The primary user is a capable but distractible person who needs Antirot to identify the next meaningful action and make beginning or reporting work immediate. The primary task is to understand current state, take the next action, and communicate with the coach without navigating a dashboard.

## Constraints

- Native SwiftUI, iOS 17 deployment floor, iPhone portrait.
- Preserve backend contracts, authentication, alarms, Screen Time, chat, voice, task parsing, and runtime state transitions.
- Preserve truthful data presentation and accessibility, including Dynamic Type, VoiceOver, Reduce Motion, and 44-point targets.
- No third-party UI framework unless a concrete native limitation is found.
- The new system must reduce equal-weight cards and ornamental glass rather than reskinning the current hierarchy.

## Reference Interpretation

Use the references for principles, not literal web layouts: decisive scale contrast, editorial type pairing, negative space, one dominant image or action per screen, few containers, disciplined alignment, controlled texture, and a narrow accent palette. Avoid copying marketing navigation, hero copy, or desktop composition into the mobile app.
