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

struct RuntimeStateResponse: Codable {
    var runtimeState: RuntimeStatePayload?
}

struct RuntimeStatePayload: Codable {
    var state: String?
    var sourceTool: String?
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
            title: "Ready Work",
            systemImage: "play.fill",
            message: "I am ready to work. Start the next serious work block."
        ),
        CoachQuickAction(
            id: "done",
            title: "Done",
            systemImage: "checkmark",
            message: "Done. I finished the current work block. Log it and tell me the next move."
        ),
        CoachQuickAction(
            id: "need_break",
            title: "Real Break",
            systemImage: "pause.fill",
            message: "I need a real break. Help me choose the minimum honest break."
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
            message: "Good night. Close today and prepare tomorrow."
        ),
        CoachQuickAction(
            id: "wake_up",
            title: "Awake",
            systemImage: "sun.max.fill",
            message: "I am awake. Log it and tell me the first concrete move."
        ),
        CoachQuickAction(
            id: "movie_break",
            title: "Movie Check",
            systemImage: "film.fill",
            message: "I want a 2 hour movie break because I deserve it. Please please."
        )
    ]

    static func primary(for runtimeState: String) -> [CoachQuickAction] {
        let byId = Dictionary(uniqueKeysWithValues: primary.map { ($0.id, $0) })
        let ids: [String]
        switch runtimeState.lowercased() {
        case "onboarding":
            ids = ["start_working", "good_night"]
        case "idle":
            ids = ["start_working", "need_break", "good_night", "movie_break"]
        case "working":
            ids = ["done", "need_break", "log_work"]
        case "break":
            ids = ["start_working", "good_night"]
        case "sleeping":
            ids = ["wake_up"]
        case "vacation", "unknown":
            ids = []
        default:
            ids = []
        }
        return ids.compactMap { byId[$0] }
    }
}
