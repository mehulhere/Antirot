import Foundation
import UIKit
import UserNotifications

@MainActor
final class AlarmCenter: ObservableObject {
    @Published var notificationStatus: UNAuthorizationStatus = .notDetermined
    @Published var alarmKitStatus: String = "unknown"
    @Published var scheduledAlarms: [AlarmJob] = []
    @Published var lastMessage: String = "No alarms scheduled"
    @Published var lastErrorDetails: String?

    private var settings: SettingsStore?

    func configure(settings: SettingsStore) async {
        self.settings = settings
        await refreshAuthorizationStatus()
        alarmKitStatus = AlarmKitCenter.authorizationLabel()
        RemoteNotificationRegistrar.register()
    }

    func requestNotificationPermission() async {
        do {
            let granted = try await UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .badge, .sound])
            await refreshAuthorizationStatus()
            if granted {
                RemoteNotificationRegistrar.register()
            }
            lastMessage = granted ? "Notification permission granted" : "Notification permission denied"
        } catch {
            recordError("Notification permission failed", error)
        }
    }

    func requestAlarmKitPermission() async {
        alarmKitStatus = await AlarmKitCenter.requestAuthorization()
        lastMessage = alarmKitStatus
    }

    func registerDevice() async {
        guard let settings else { return }
        let client = APIClient(baseURL: settings.baseURL, apiToken: settings.apiToken)
        do {
            let response = try await client.registerDevice(DeviceRegistrationRequest(
                deviceId: settings.deviceId,
                platform: "ios",
                appVersion: Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "0.1.0",
                notificationCapability: notificationCapability,
                usageCapability: await ScreenTimeCenter.currentCapability(),
                pushProvider: settings.pushToken.isEmpty ? nil : "apns",
                pushToken: settings.pushToken.isEmpty ? nil : settings.pushToken
            ))
            settings.registered = response.ok
            settings.statusMessage = response.message ?? "Registered as \(response.deviceId)"
            lastMessage = settings.statusMessage
            lastErrorDetails = nil
        } catch {
            settings.statusMessage = "Registration failed"
            lastMessage = settings.statusMessage
            recordError("Registration failed", error)
        }
    }

    func pollPendingAlarms() async {
        guard let settings else { return }
        let client = APIClient(baseURL: settings.baseURL, apiToken: settings.apiToken)
        do {
            let alarms = try await client.fetchPendingAlarms(deviceId: settings.deviceId)
            for alarm in alarms {
                try await schedule(alarm)
            }
            lastMessage = alarms.isEmpty ? "No pending alarms" : "Scheduled \(alarms.count) alarm(s)"
            lastErrorDetails = nil
        } catch {
            recordError("Poll failed", error)
        }
    }

    func scheduleTestAlarm(severity: AlarmJob.Severity) async {
        do {
            try await schedule(.test(severity: severity))
        } catch {
            recordError("Test alarm failed", error)
        }
    }

    func schedule(_ alarm: AlarmJob) async throws {
        let soundChoice = alarmSoundChoice(for: alarm.severity)
        let soundName = soundChoice.name
        let scheduledWithAlarmKit = try await AlarmKitCenter.schedule(alarm, soundName: soundName)
        if scheduledWithAlarmKit {
            scheduledAlarms.append(alarm)
            let widgetUpdated = writeCurrentTaskSnapshot(for: alarm)
            alarmKitStatus = AlarmKitCenter.authorizationLabel()
            let soundMessage = "Real AlarmKit alarm scheduled with \(soundChoice.label)"
            lastMessage = widgetUpdated ? soundMessage : "\(soundMessage). Widget app-group storage unavailable."
            return
        }

        let content = UNMutableNotificationContent()
        content.title = alarm.title
        content.body = alarm.message
        content.sound = UNNotificationSound(named: UNNotificationSoundName(rawValue: soundName))
        content.categoryIdentifier = AlarmNotificationActions.categoryIdentifier
        content.userInfo = ["alarmId": alarm.id]

        let delay = max(1, alarm.fireAt.timeIntervalSinceNow)
        let trigger = UNTimeIntervalNotificationTrigger(timeInterval: delay, repeats: false)
        let request = UNNotificationRequest(identifier: alarm.id, content: content, trigger: trigger)
        try await UNUserNotificationCenter.current().add(request)
        scheduledAlarms.append(alarm)
        let widgetUpdated = writeCurrentTaskSnapshot(for: alarm)
        let scheduleMessage = "AlarmKit unavailable; scheduled notification fallback with \(soundChoice.label)"
        lastMessage = widgetUpdated ? scheduleMessage : "\(scheduleMessage). Widget app-group storage unavailable."
    }

    func refreshAuthorizationStatus() async {
        let settings = await UNUserNotificationCenter.current().notificationSettings()
        notificationStatus = settings.authorizationStatus
    }

    private var notificationCapability: String {
        switch notificationStatus {
        case .authorized, .provisional, .ephemeral:
            settings?.pushToken.isEmpty == false ? "remote_notification" : "notification"
        default:
            "none"
        }
    }

    private func alarmSoundChoice(for severity: AlarmJob.Severity) -> (name: String, label: String) {
        guard let settings else {
            return (bundledSoundName(for: severity), "bundled sound")
        }

        let mode = AlarmSoundMode(storedValue: settings.alarmSoundMode)
        switch mode {
        case .automatic:
            return (bundledSoundName(for: severity), "automatic bundled sound")
        case .bundledNormal:
            return ("antirot-normal.wav", "bundled normal sound")
        case .bundledLoud:
            return ("antirot-loud.wav", "bundled loud sound")
        case .custom:
            if let customName = settings.alarmSoundName.nilIfBlank {
                return (customName, "custom sound")
            }
            return (bundledSoundName(for: severity), "automatic bundled sound because no custom sound is imported")
        }
    }

    private func bundledSoundName(for severity: AlarmJob.Severity) -> String {
        switch severity {
        case .normal:
            "antirot-normal.wav"
        case .loud, .urgent:
            "antirot-loud.wav"
        }
    }

    private func writeCurrentTaskSnapshot(for alarm: AlarmJob) -> Bool {
        let subtitle = switch alarm.kind {
        case .normalWake, .loudWake:
            "Wake up. Day does not start by negotiating with the pillow."
        case .routineOverdue:
            "Routine window is over. Come back."
        case .sessionOverdue:
            alarm.message
        case .nonResponse:
            "You vanished. Fix that."
        case .test:
            "Test alarm scheduled. Nothing heroic yet."
        }
        return SharedTaskStore.write(CurrentTaskSnapshot(
            title: alarm.title,
            subtitle: subtitle,
            mode: alarm.kind.rawValue,
            dueAt: alarm.fireAt
        ))
    }

    private func recordError(_ summary: String, _ error: Error) {
        lastMessage = summary
        lastErrorDetails = [
            summary,
            error.localizedDescription,
            String(describing: error)
        ].joined(separator: "\n\n")
    }
}

private extension String {
    var nilIfBlank: String? {
        let trimmed = trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }
}
