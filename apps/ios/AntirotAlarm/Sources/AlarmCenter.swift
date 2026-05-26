import Foundation
import UserNotifications

@MainActor
final class AlarmCenter: ObservableObject {
    @Published var notificationStatus: UNAuthorizationStatus = .notDetermined
    @Published var scheduledAlarms: [AlarmJob] = []
    @Published var lastMessage: String = "No alarms scheduled"

    private var settings: SettingsStore?

    func configure(settings: SettingsStore) async {
        self.settings = settings
        await refreshAuthorizationStatus()
    }

    func requestNotificationPermission() async {
        do {
            let granted = try await UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .badge, .sound])
            await refreshAuthorizationStatus()
            lastMessage = granted ? "Notification permission granted" : "Notification permission denied"
        } catch {
            lastMessage = "Notification permission failed: \(error.localizedDescription)"
        }
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
                usageCapability: await ScreenTimeCenter.currentCapability()
            ))
            settings.registered = response.ok
            settings.statusMessage = response.message ?? "Registered as \(response.deviceId)"
            lastMessage = settings.statusMessage
        } catch {
            settings.statusMessage = "Registration failed: \(error.localizedDescription)"
            lastMessage = settings.statusMessage
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
        } catch {
            lastMessage = "Poll failed: \(error.localizedDescription)"
        }
    }

    func scheduleTestAlarm(severity: AlarmJob.Severity) async {
        do {
            try await schedule(.test(severity: severity))
        } catch {
            lastMessage = "Test alarm failed: \(error.localizedDescription)"
        }
    }

    func schedule(_ alarm: AlarmJob) async throws {
        let content = UNMutableNotificationContent()
        content.title = alarm.title
        content.body = alarm.message
        content.sound = alarm.severity == .normal ? .default : .defaultCritical
        content.categoryIdentifier = AlarmNotificationActions.categoryIdentifier
        content.userInfo = ["alarmId": alarm.id]

        let delay = max(1, alarm.fireAt.timeIntervalSinceNow)
        let trigger = UNTimeIntervalNotificationTrigger(timeInterval: delay, repeats: false)
        let request = UNNotificationRequest(identifier: alarm.id, content: content, trigger: trigger)
        try await UNUserNotificationCenter.current().add(request)
        scheduledAlarms.append(alarm)
        lastMessage = "Alarm scheduled"
    }

    func refreshAuthorizationStatus() async {
        let settings = await UNUserNotificationCenter.current().notificationSettings()
        notificationStatus = settings.authorizationStatus
    }

    private var notificationCapability: String {
        switch notificationStatus {
        case .authorized, .provisional, .ephemeral:
            "notification"
        default:
            "none"
        }
    }
}
