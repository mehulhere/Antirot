import SwiftUI

// MARK: - Coach Emotion

/// The expressive states the stylized coach can take on screen.
/// The backend may return `coach_emotion` with a coach reply; the app maps
/// that string to this enum and drives the `CoachStage` animation.
enum CoachEmotion: String, CaseIterable, Codable {
    case watching
    case checkingClock = "checking_clock"
    case thinking
    case focused
    case strict
    case impatient
    case approving
    case disappointed
    case celebrating
    case silentWaiting = "silent_waiting"

    // Fallback labels mentioned in the PRD, accepted from the backend even
    // though they are not primary animation states.
    case neutralWatch = "neutral_watch"
    case thinkingDone = "thinking_done"

    /// Decode a backend emotion string defensively. Unknown/absent values
    /// fall back to a calm watching pose instead of crashing or showing nothing.
    static func from(_ raw: String?) -> CoachEmotion {
        guard let raw, let value = CoachEmotion(rawValue: raw) else { return .watching }
        switch value {
        case .neutralWatch: return .watching
        case .thinkingDone: return .thinking
        default: return value
        }
    }

    /// Whether this emotion should trigger the energetic start feedback.
    var isCelebratory: Bool {
        self == .celebrating || self == .approving
    }

    /// Quiet, monochrome-friendly accent used for the coach halo and gaze.
    /// Stays within the Antirot palette: a single muted red accent plus
    /// restrained neutrals, so the screen never turns into a dashboard.
    var accentColor: Color {
        switch self {
        case .watching, .checkingClock, .silentWaiting:
            return .arTextSecondary
        case .thinking:
            return .arAccentDim
        case .focused, .strict:
            return .arAccent
        case .impatient:
            return .arWarning
        case .approving, .celebrating:
            return .arSuccess
        case .disappointed:
            return .arTextMuted
        case .neutralWatch, .thinkingDone:
            return .arTextSecondary
        }
    }

    /// A one-line coach status shown in the collapsed chat sheet when there
    /// is no fresh reply to display. Keeps the coach "present" at all times.
    var ambientOneLiner: String {
        switch self {
        case .watching: return "Watching."
        case .checkingClock: return "Checking the clock."
        case .thinking: return "Thinking."
        case .focused: return "Focused on you."
        case .strict: return "Not impressed. Move."
        case .impatient: return "I'm waiting."
        case .approving: return "Good. Keep it."
        case .disappointed: return "That's not it."
        case .celebrating: return "That's the work."
        case .silentWaiting: return "…"
        case .neutralWatch, .thinkingDone: return "Watching."
        }
    }
}

// MARK: - Emotion Contract

/// Optional payload a coach reply may carry, per the PRD "LLM Emotion Contract".
/// Both fields are optional so the app degrades gracefully when the backend
/// has not attached them yet.
struct CoachEmotionPayload: Equatable {
    var emotion: CoachEmotion
    var voicePreface: String?

    static let none = CoachEmotionPayload(emotion: .watching, voicePreface: nil)
}
