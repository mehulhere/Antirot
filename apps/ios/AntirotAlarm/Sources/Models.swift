import Foundation

struct HealthResponse: Codable {
    var ok: Bool
    var service: String
}

struct AlarmJob: Codable, Identifiable, Equatable {
    enum Kind: Codable, Equatable {
        case normalWake
        case loudWake
        case routineOverdue
        case sessionOverdue
        case nonResponse
        case sessionAlarm
        case breakAlarm
        case wakeAlarm
        case idleAlarm
        case test
        case unknown(String)

        init(from decoder: Decoder) throws {
            let value = try decoder.singleValueContainer().decode(String.self)
            self = switch value {
            case "normal_wake": .normalWake
            case "loud_wake": .loudWake
            case "routine_overdue": .routineOverdue
            case "session_overdue": .sessionOverdue
            case "non_response": .nonResponse
            case "session_alarm": .sessionAlarm
            case "break_alarm": .breakAlarm
            case "wake_alarm": .wakeAlarm
            case "idle_alarm": .idleAlarm
            case "test": .test
            default: .unknown(value)
            }
        }

        func encode(to encoder: Encoder) throws {
            var container = encoder.singleValueContainer()
            try container.encode(rawValue)
        }

        var rawValue: String {
            switch self {
            case .normalWake: "normal_wake"
            case .loudWake: "loud_wake"
            case .routineOverdue: "routine_overdue"
            case .sessionOverdue: "session_overdue"
            case .nonResponse: "non_response"
            case .sessionAlarm: "session_alarm"
            case .breakAlarm: "break_alarm"
            case .wakeAlarm: "wake_alarm"
            case .idleAlarm: "idle_alarm"
            case .test: "test"
            case .unknown(let value): value
            }
        }
    }

    enum Severity: String, Codable {
        case normal
        case loud
        case urgent
    }

    var id: String
    var kind: Kind
    var seriesId: String
    var generation: Int
    var deliveryToken: String?
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
            seriesId: "local-test",
            generation: 1,
            deliveryToken: nil,
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

struct PendingAlarmsResponse: Codable {
    var alarms: [AlarmJob]
    var cancelledSeriesIds: [String]
    var cancelledAlarmIds: [String]
    var cancellations: [AlarmCancellationTombstone]
}

struct AlarmCancellationTombstone: Codable {
    var seriesId: String
    var localAlarmIds: [String]
}

struct AlarmActionResponse: Codable {
    var ok: Bool
    var alarmId: String
    var status: String
    var cancelledSeriesIds: [String]
    var replacementAlarm: AlarmJob?
}

struct ScheduledAlarmConfirmation: Codable {
    var alarmId: String
    var deliveryToken: String
    var localAlarmId: String
}

struct AlarmReconcileRequest: Codable {
    var deviceId: String
    var scheduled: [ScheduledAlarmConfirmation]
    var cancelledSeriesIds: [String]
}

struct AlarmReconcileResponse: Codable {
    var ok: Bool
    var scheduledCount: Int
    var cancellationCount: Int
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
    var deviceToken: String?
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
    var userId: String
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
    var requestId: String
}

struct ChatCoachResponse: Codable {
    var ok: Bool
    var reply: String
    var runtimeState: RuntimeStatePayload?
    /// Optional "LLM Emotion Contract" fields. Absent on backend builds that
    /// have not attached them yet, so both decode to nil gracefully and the
    /// coach falls back to a calm watching pose.
    var coachEmotion: String? = nil
    var voicePreface: String? = nil

    /// Resolved emotion used to drive the coach stage, with a safe fallback.
    var emotion: CoachEmotion { CoachEmotion.from(coachEmotion) }

    enum CodingKeys: String, CodingKey {
        case ok
        case reply
        case runtimeState
        case coachEmotion = "coach_emotion"
        case voicePreface = "voice_preface"
    }
}

struct RuntimeStateResponse: Codable {
    var ok: Bool?
    var runtimeState: RuntimeStatePayload?
}

struct OnboardingProfileRequest: Codable {
    var name: String
    var timezone: String
}

struct OnboardingProfileResponse: Codable {
    var ok: Bool
    var name: String
    var timezone: String
    var reply: String
}

struct MemoryResponse: Codable {
    var ok: Bool
    var key: String
    var content: String
    var updatedAt: Date
}

struct CreateMemorySnapshotRequest: Codable {
    var deviceId: String?
    var title: String?
    var reason: String?
}

struct MemorySnapshotSummary: Codable, Identifiable, Equatable {
    var id: String
    var deviceId: String?
    var title: String
    var reason: String
    var memoryKeys: [String]
    var runtimeState: RuntimeStateSnapshotPayload?
    var createdAt: Date
}

struct RuntimeStateSnapshotPayload: Codable, Equatable {
    var state: String?
    var enteredAt: Date?
    var sourceTool: String?
}

struct CreateMemorySnapshotResponse: Codable {
    var ok: Bool
    var snapshot: MemorySnapshotSummary
    var retainedCount: Int
    var retentionLimit: Int
}

struct ListMemorySnapshotsResponse: Codable {
    var ok: Bool
    var snapshots: [MemorySnapshotSummary]
    var retentionLimit: Int
}

struct RestoreMemorySnapshotRequest: Codable {
    var restoreRuntimeState: Bool?
}

struct RestoreMemorySnapshotResponse: Codable {
    var ok: Bool
    var snapshot: MemorySnapshotSummary
    var restoredMemoryKeys: [String]
    var restoredRuntimeState: Bool
}

struct ReportEventPayload: Codable, Equatable {
    var at: Date
    var kind: String
    var summary: String
    var detail: String?
}

struct CreateReportRequest: Codable {
    var deviceId: String?
    var title: String
    var windowStart: Date
    var windowEnd: Date
    var reportMarkdown: String
    var events: [ReportEventPayload]
}

struct CreateReportResponse: Codable {
    var ok: Bool
    var reportId: String
    var savedAt: Date
}

struct RuntimeStatePayload: Codable {
    var state: String?
    var sourceTool: String?
    var metadata: String?
}

struct StatsPeriodResponse: Codable, Equatable {
    var label: String
    var workMinutes: Int
    var idleMinutes: Int
    var unproductiveDeskMinutes: Int
    var sessionsCompleted: Int
    var tasksDone: Int
}

struct StatsResponse: Codable, Equatable {
    var ok: Bool
    var generatedAt: Date
    var today: StatsPeriodResponse
    var week: StatsPeriodResponse
    var month: StatsPeriodResponse
    var checkedTasksTotal: Int
    var note: String
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
    var audioFileURL: URL?

    var isPlayableVoiceMessage: Bool {
        audioFileURL != nil
    }
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
            title: "I am ready to work",
            systemImage: "play.fill",
            message: "I am ready to work. Start the task we just picked."
        ),
        CoachQuickAction(
            id: "done",
            title: "Done",
            systemImage: "checkmark",
            message: "Done. I finished the current task. Ask me how much of it was actually productive before closing it."
        ),
        CoachQuickAction(
            id: "need_break",
            title: "I need a real break",
            systemImage: "pause.fill",
            message: "I need a real break. Help me choose the minimum honest break."
        ),
        CoachQuickAction(
            id: "wake_up",
            title: "I am awake",
            systemImage: "sun.max.fill",
            message: "I am awake. Log it and tell me the first specific move."
        ),
        CoachQuickAction(
            id: "movie_break",
            title: "Movie break check",
            systemImage: "film.fill",
            message: "I want a 2 hour movie break because I deserve it. Please please."
        )
    ]

    static func primary(for runtimeState: String, at date: Date = Date()) -> [CoachQuickAction] {
        let byId = Dictionary(uniqueKeysWithValues: primary.map { ($0.id, $0) })
        let ids: [String]
        switch runtimeState.lowercased() {
        case "onboarding":
            ids = []
        case "idle":
            ids = ["start_working"]
        case "working":
            ids = ["done"]
        case "break", "sleeping", "vacation", "unknown":
            ids = []
        default:
            ids = []
        }
        return ids.compactMap { id in
            byId[id]
        }
    }
}
