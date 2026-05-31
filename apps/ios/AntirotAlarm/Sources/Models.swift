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
}

struct GoogleAuthResponse: Codable {
    var ok: Bool
    var deviceId: String
    var deviceToken: String
    var email: String
    var name: String?
    var message: String
}

struct AlarmActionRequest: Codable {
    var deviceId: String
    var action: String
    var at: Date
    var minutes: Int?
}
