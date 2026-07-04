import SwiftUI

// MARK: - State-Driven Action Buttons

/// A single circular or pill action that sends an explicit user message to
/// the coach. Buttons and chat share one backend path: a button is just a
/// short, honest user message routed through `/v1/chat`, so the coach keeps
/// ownership of state transitions and can challenge the intent.
struct CoachStateButton: Identifiable, Equatable {
    let id: String
    let title: String
    let systemImage: String
    let message: String
    /// True only for the Start action, which fires the celebratory particle burst.
    var triggersConfetti: Bool = false
}

/// Maps the current runtime state to exactly one dominant primary button and
/// an optional set of quiet secondary buttons, per the PRD state-action table.
///
/// This is intentionally separate from the legacy `CoachQuickAction.primary(for:)`
/// map so the existing action-button regression test stays green. The cinematic
/// home screen uses this structured primary/secondary model instead.
enum CoachStateActions {
    struct Set: Equatable {
        var primary: CoachStateButton
        var secondary: [CoachStateButton]
    }

    static func actions(for runtimeState: String) -> Set {
        switch runtimeState.lowercased() {
        case "idle":
            return Set(
                primary: CoachStateButton(
                    id: "start",
                    title: "Start",
                    systemImage: "play.fill",
                    message: "I am ready to work. Start the task we just picked.",
                    triggersConfetti: true
                ),
                secondary: [
                    CoachStateButton(
                        id: "plan",
                        title: "Plan",
                        systemImage: "list.bullet.clipboard",
                        message: "Help me choose the next concrete task before starting."
                    ),
                    CoachStateButton(
                        id: "checkin",
                        title: "Check In",
                        systemImage: "message",
                        message: "Quick check-in. Ask what I am doing and redirect me."
                    )
                ]
            )
        case "working":
            return Set(
                primary: CoachStateButton(
                    id: "done",
                    title: "Done",
                    systemImage: "checkmark",
                    message: "Done. I finished the current task. Ask me how much of it was actually productive before closing it."
                ),
                secondary: [
                    CoachStateButton(
                        id: "extend",
                        title: "Extend",
                        systemImage: "arrow.forward.circle",
                        message: "Extend this work block. I'm in flow and the task isn't done."
                    ),
                    CoachStateButton(
                        id: "break",
                        title: "Break",
                        systemImage: "pause.circle",
                        message: "I need a real break. Help me choose the minimum honest break."
                    )
                ]
            )
        case "break":
            return Set(
                primary: CoachStateButton(
                    id: "resume",
                    title: "Resume",
                    systemImage: "play.fill",
                    message: "I'm back. Resume the work block."
                ),
                secondary: [
                    CoachStateButton(
                        id: "extend_break",
                        title: "Extend",
                        systemImage: "plus.circle",
                        message: "I need a longer break. Challenge me, then help me set the smallest honest extension."
                    ),
                    CoachStateButton(
                        id: "end_break",
                        title: "End",
                        systemImage: "xmark.circle",
                        message: "End this break and help me decide the next move."
                    )
                ]
            )
        case "sleeping":
            return Set(
                primary: CoachStateButton(
                    id: "awake",
                    title: "Awake",
                    systemImage: "sun.max.fill",
                    message: "I am awake. Log it and tell me the first specific move."
                ),
                secondary: [
                    CoachStateButton(
                        id: "snooze",
                        title: "Snooze",
                        systemImage: "moon.zzz",
                        message: "I need more sleep. Ask for the honest reason and set the smallest wake extension."
                    ),
                    CoachStateButton(
                        id: "rough_sleep",
                        title: "Rough",
                        systemImage: "cloud.rain",
                        message: "I woke up rough. Adjust the first move without letting me drift."
                    )
                ]
            )
        case "onboarding":
            return Set(
                primary: CoachStateButton(
                    id: "begin",
                    title: "Begin",
                    systemImage: "bolt.fill",
                    message: "Begin onboarding. Ask me for the baseline you need, then keep me moving."
                ),
                secondary: [
                    CoachStateButton(
                        id: "voice",
                        title: "Voice",
                        systemImage: "mic",
                        message: "I want to answer onboarding by voice. Ask the next thing simply."
                    ),
                    CoachStateButton(
                        id: "skip",
                        title: "Skip",
                        systemImage: "forward",
                        message: "Skip the long setup and get me into one concrete task."
                    )
                ]
            )
        case "vacation":
            return Set(
                primary: CoachStateButton(
                    id: "return",
                    title: "Return",
                    systemImage: "arrow.uturn.backward",
                    message: "End vacation mode and help me re-enter with one concrete move."
                ),
                secondary: [
                    CoachStateButton(
                        id: "extend_vacation",
                        title: "Extend",
                        systemImage: "plus.circle",
                        message: "Extend vacation mode. Ask for the reason and re-entry plan."
                    ),
                    CoachStateButton(
                        id: "review",
                        title: "Review",
                        systemImage: "sparkles",
                        message: "Review my current plan and help me decide whether vacation should continue."
                    )
                ]
            )
        case "unknown":
            return Set(
                primary: CoachStateButton(
                    id: "reconnect",
                    title: "Refresh",
                    systemImage: "arrow.clockwise",
                    message: "Reconnect and resync my current state."
                ),
                secondary: [
                    CoachStateButton(
                        id: "start",
                        title: "Start",
                        systemImage: "play.fill",
                        message: "I am ready to work. Start the task we just picked.",
                        triggersConfetti: true
                    ),
                    CoachStateButton(
                        id: "checkin",
                        title: "Check In",
                        systemImage: "message",
                        message: "Quick check-in. Ask what I am doing and redirect me."
                    )
                ]
            )
        case "offline":
            return Set(
                primary: CoachStateButton(
                    id: "reconnect",
                    title: "Refresh",
                    systemImage: "arrow.clockwise",
                    message: "Reconnect and resync my current state."
                ),
                secondary: [
                    CoachStateButton(
                        id: "start",
                        title: "Start",
                        systemImage: "play.fill",
                        message: "I am ready to work. Start the task we just picked.",
                        triggersConfetti: true
                    ),
                    CoachStateButton(
                        id: "checkin",
                        title: "Check In",
                        systemImage: "message",
                        message: "Quick check-in. Ask what I am doing and redirect me."
                    )
                ]
            )
        default:
            return Set(
                primary: CoachStateButton(
                    id: "reconnect",
                    title: "Refresh",
                    systemImage: "arrow.clockwise",
                    message: "Reconnect and resync my current state."
                ),
                secondary: [
                    CoachStateButton(
                        id: "start",
                        title: "Start",
                        systemImage: "play.fill",
                        message: "I am ready to work. Start the task we just picked.",
                        triggersConfetti: true
                    ),
                    CoachStateButton(
                        id: "checkin",
                        title: "Check In",
                        systemImage: "message",
                        message: "Quick check-in. Ask what I am doing and redirect me."
                    )
                ]
            )
        }
    }
}
