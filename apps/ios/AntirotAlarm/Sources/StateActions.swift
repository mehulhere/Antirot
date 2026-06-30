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
                secondary: []
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
                secondary: []
            )
        case "sleeping":
            return Set(
                primary: CoachStateButton(
                    id: "awake",
                    title: "Awake",
                    systemImage: "sun.max.fill",
                    message: "I am awake. Log it and tell me the first specific move."
                ),
                secondary: []
            )
        case "onboarding":
            return Set(
                primary: CoachStateButton(
                    id: "begin",
                    title: "Talk",
                    systemImage: "bubble.left.fill",
                    message: "Let's begin. Walk me through today and what I'm getting done."
                ),
                secondary: []
            )
        case "unknown":
            return Set(
                primary: CoachStateButton(
                    id: "talk",
                    title: "Talk",
                    systemImage: "bubble.left.fill",
                    message: "Let's talk. Help me figure out the next honest move."
                ),
                secondary: []
            )
        case "offline":
            return Set(
                primary: CoachStateButton(
                    id: "reconnect",
                    title: "Reconnect",
                    systemImage: "arrow.clockwise",
                    message: "Reconnect and resync my current state."
                ),
                secondary: []
            )
        default:
            return Set(
                primary: CoachStateButton(
                    id: "reconnect",
                    title: "Reconnect",
                    systemImage: "arrow.clockwise",
                    message: "Reconnect and resync my current state."
                ),
                secondary: []
            )
        }
    }
}
