import Foundation

struct AlarmJob: Codable, Identifiable, Equatable {
    enum Kind: String, Codable {
        case normalWake = "normal_wake"
        case loudWake = "loud_wake"
        case routineOverdue = "routine_overdue"
        case sessionOverdue = "session_overdue"
        case nonResponse = "non_response"
        case test
    }

    enum Severity: String, Codable {
        case normal
        case loud
        case urgent
    }

    var id: String
    var kind: Kind
    var severity: Severity
    var title: String
    var message: String
    var fireAt: Date
    var hiddenBufferApplied: Bool
    var requiresAcknowledgement: Bool
    var expiresAt: Date?

    static func test(severity: Severity) -> AlarmJob {
        AlarmJob(
            id: "local-test-\(UUID().uuidString)",
            kind: .test,
            severity: severity,
            title: severity == .normal ? "Antirot test" : "Antirot loud test",
            message: severity == .normal ? "Normal alarm test. Wake up, champ." : "Loud test. Enough disappearing.",
            fireAt: Date().addingTimeInterval(5),
            hiddenBufferApplied: false,
            requiresAcknowledgement: true,
            expiresAt: Date().addingTimeInterval(300)
        )
    }
}

struct DeviceRegistrationRequest: Codable {
    var deviceId: String
    var platform: String
    var appVersion: String
    var notificationCapability: String
    var usageCapability: String
    var pushProvider: String?
    var pushToken: String?
}

struct DeviceRegistrationResponse: Codable {
    var ok: Bool
    var deviceId: String
    var message: String?
}

struct GoogleAuthRequest: Codable {
    var idToken: String
    var deviceId: String
    var platform: String
    var appVersion: String
    var notificationCapability: String
    var usageCapability: String
    var pushProvider: String?
    var pushToken: String?
}

struct GoogleAuthResponse: Codable {
    var ok: Bool
    var deviceId: String
    var deviceToken: String
    var email: String
    var name: String?
    var message: String
}

struct PairingClaimRequest: Codable {
    var code: String
    var deviceId: String
    var deviceName: String
    var platform: String
}

struct PairingClaimResponse: Codable {
    var ok: Bool
    var workspaceId: String
    var deviceId: String
    var message: String
}

struct AlarmActionRequest: Codable {
    var deviceId: String
    var action: String
    var at: Date
    var minutes: Int?
}

struct ChatCoachRequest: Codable {
    var message: String
}

struct ChatCoachResponse: Codable {
    var ok: Bool
    var reply: String
}

struct SpeechTranscriptionResponse: Codable {
    var ok: Bool
    var text: String
}

struct SpeechSynthesisRequest: Codable {
    var text: String
    var voiceId: String?
}

struct SpeechSynthesisResponse: Codable {
    var ok: Bool
    var audioBase64: String
    var contentType: String

    var audioData: Data? {
        Data(base64Encoded: audioBase64)
    }
}

struct CoachMessage: Identifiable, Equatable {
    enum Role: Equatable {
        case user
        case coach
        case system
    }

    let id = UUID()
    var role: Role
    var text: String
    var createdAt: Date = Date()
}

struct CoachQuickAction: Identifiable, Equatable {
    var id: String
    var title: String
    var systemImage: String
    var message: String
    var fillsDraft: Bool = false

    static let primary: [CoachQuickAction] = [
        CoachQuickAction(
            id: "start_working",
            title: "Start Working",
            systemImage: "play.fill",
            message: "I am starting a focused work session now. Help me choose the exact next task and timer."
        ),
        CoachQuickAction(
            id: "done",
            title: "Done",
            systemImage: "checkmark",
            message: "Done. I finished the current work block. Log it and tell me the next move."
        ),
        CoachQuickAction(
            id: "need_break",
            title: "Need Break",
            systemImage: "pause.fill",
            message: "I need a break. Keep it honest and short unless I justify more."
        ),
        CoachQuickAction(
            id: "log_work",
            title: "Log Work",
            systemImage: "square.and.pencil",
            message: "Log work: I worked on ",
            fillsDraft: true
        ),
        CoachQuickAction(
            id: "good_night",
            title: "Good Night",
            systemImage: "moon.fill",
            message: "Good night. Start sleep and distill today's memory."
        ),
        CoachQuickAction(
            id: "wake_up",
            title: "Awake",
            systemImage: "sun.max.fill",
            message: "I am awake. Log wake and decide the first move."
        )
    ]
}
