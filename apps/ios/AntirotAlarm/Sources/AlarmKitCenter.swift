import Foundation
import SwiftUI

#if canImport(AlarmKit)
import ActivityKit
import AlarmKit
import AppIntents

@available(iOS 26.0, *)
struct AntirotAlarmMetadata: AlarmMetadata, Codable, Sendable {
    var antirotId: String
    var severity: String
}

@available(iOS 26.0, *)
struct StopAntirotAlarmIntent: LiveActivityIntent {
    static var title: LocalizedStringResource = "I'm awake"

    @Parameter(title: "Alarm ID")
    var alarmID: String

    init() {}

    init(alarmID: String) {
        self.alarmID = alarmID
    }

    func perform() async throws -> some IntentResult {
        let defaults = UserDefaults.standard
        let serverURL = defaults.string(forKey: "serverURL").flatMap(URL.init(string:))
        let apiToken = try SecureTokenStore().load()
        let deviceId = defaults.string(forKey: "deviceId") ?? "unknown-device"
        try await APIClient(baseURL: serverURL, apiToken: apiToken)
            .acknowledge(alarmId: alarmID, deviceId: deviceId, action: "ack")
        return .result()
    }
}

@available(iOS 26.0, *)
struct SnoozeAntirotAlarmIntent: LiveActivityIntent {
    static var title: LocalizedStringResource = "Need more time"

    @Parameter(title: "Alarm ID")
    var alarmID: String

    init() {}

    init(alarmID: String) {
        self.alarmID = alarmID
    }

    func perform() async throws -> some IntentResult {
        let defaults = UserDefaults.standard
        let serverURL = defaults.string(forKey: "serverURL").flatMap(URL.init(string:))
        let apiToken = try SecureTokenStore().load()
        let deviceId = defaults.string(forKey: "deviceId") ?? "unknown-device"
        try await APIClient(baseURL: serverURL, apiToken: apiToken)
            .acknowledge(alarmId: alarmID, deviceId: deviceId, action: "snooze", minutes: 9)
        return .result()
    }
}
#endif

enum AlarmKitCenter {
    static func authorizationLabel() -> String {
        #if canImport(AlarmKit)
        if #available(iOS 26.1, *) {
            return String(describing: AlarmManager.shared.authorizationState)
        }
        #endif
        return "unavailable"
    }

    static func requestAuthorization() async -> String {
        #if canImport(AlarmKit)
        if #available(iOS 26.0, *) {
            do {
                let state = try await AlarmManager.shared.requestAuthorization()
                return "AlarmKit \(String(describing: state))"
            } catch {
                return "AlarmKit authorization failed: \(error.localizedDescription)"
            }
        }
        #endif
        return "AlarmKit unavailable. Requires iOS 26 SDK/device support."
    }

    static func schedule(_ alarm: AlarmJob, soundName: String?) async throws -> Bool {
        #if canImport(AlarmKit)
        if #available(iOS 26.0, *) {
            let authorization = AlarmManager.shared.authorizationState
            guard authorization == .authorized else {
                return false
            }
            return try await scheduleAuthorized(alarm, soundName: soundName)
        }
        #endif
        return false
    }

    #if canImport(AlarmKit)
    @available(iOS 26.1, *)
    private static func scheduleAuthorized(_ alarm: AlarmJob, soundName: String?) async throws -> Bool {
        let alert = AlarmPresentation.Alert(
            title: LocalizedStringResource(stringLiteral: alarm.title),
            secondaryButton: AlarmButton(
                text: "Snooze",
                textColor: .white,
                systemImageName: "clock.arrow.circlepath"
            ),
            secondaryButtonBehavior: .custom
        )
        let presentation = AlarmPresentation(alert: alert, countdown: nil, paused: nil)
        let attributes = AlarmAttributes<AntirotAlarmMetadata>(
            presentation: presentation,
            metadata: AntirotAlarmMetadata(antirotId: alarm.id, severity: alarm.severity.rawValue),
            tintColor: alarm.severity == .normal ? .orange : .red
        )
        let id = UUID(uuidString: stableUuidString(alarm.id)) ?? UUID()
        let sound = soundName.map(AlertConfiguration.AlertSound.named) ?? .default
        let configuration = AlarmManager.AlarmConfiguration.alarm(
            schedule: .fixed(alarm.fireAt),
            attributes: attributes,
            stopIntent: StopAntirotAlarmIntent(alarmID: alarm.id),
            secondaryIntent: SnoozeAntirotAlarmIntent(alarmID: alarm.id),
            sound: sound
        )
        _ = try await AlarmManager.shared.schedule(id: id, configuration: configuration)
        return true
    }
    #endif

    private static func stableUuidString(_ value: String) -> String {
        let bytes = Array(value.utf8)
        var padded = Array(repeating: UInt8(0), count: 16)
        for (index, byte) in bytes.enumerated() {
            padded[index % 16] = padded[index % 16] &+ byte
        }
        return UUID(uuid: (
            padded[0], padded[1], padded[2], padded[3],
            padded[4], padded[5],
            padded[6], padded[7],
            padded[8], padded[9],
            padded[10], padded[11], padded[12], padded[13], padded[14], padded[15]
        )).uuidString
    }
}
