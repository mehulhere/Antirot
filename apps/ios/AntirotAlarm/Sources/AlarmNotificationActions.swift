import Foundation
import UserNotifications

enum AlarmNotificationActions {
    static let categoryIdentifier = "ANTIROT_ALARM"

    static func register() {
        let stop = UNNotificationAction(
            identifier: "stop",
            title: "I'm awake",
            options: [.authenticationRequired]
        )
        let snooze = UNNotificationAction(
            identifier: "snooze",
            title: "Snooze",
            options: []
        )
        let moreTime = UNNotificationAction(
            identifier: "need_more_time",
            title: "Need more time",
            options: [.foreground]
        )
        let category = UNNotificationCategory(
            identifier: categoryIdentifier,
            actions: [stop, snooze, moreTime],
            intentIdentifiers: [],
            options: [.customDismissAction]
        )
        UNUserNotificationCenter.current().setNotificationCategories([category])
    }

    static func handle(response: UNNotificationResponse) async {
        let alarmId = response.notification.request.content.userInfo["alarmId"] as? String
        guard let alarmId else { return }

        let defaults = UserDefaults.standard
        let serverURL = defaults.string(forKey: "serverURL").flatMap(URL.init(string:))
        let apiToken: String
        do {
            apiToken = try SecureTokenStore().load()
        } catch {
            print("🔴 FALLBACK: alarm token read failed - Reason: \(error.localizedDescription) - Impact: backend may not receive this notification action")
            return
        }
        let deviceId = defaults.string(forKey: "deviceId") ?? "unknown-device"
        let client = APIClient(baseURL: serverURL, apiToken: apiToken)
        let action = response.actionIdentifier == UNNotificationDefaultActionIdentifier ? "ack" : response.actionIdentifier

        do {
            switch action {
            case "snooze":
                try await client.acknowledge(alarmId: alarmId, deviceId: deviceId, action: "snooze", minutes: 9)
            case "need_more_time":
                try await client.acknowledge(alarmId: alarmId, deviceId: deviceId, action: "snooze", minutes: 15)
            case "stop", "ack":
                try await client.acknowledge(alarmId: alarmId, deviceId: deviceId, action: "ack")
            default:
                try await client.acknowledge(alarmId: alarmId, deviceId: deviceId, action: "clear")
            }
            await AlarmActionReconciler.reconcile()
        } catch {
            print("🔴 FALLBACK: alarm callback failed - Reason: \(error.localizedDescription) - Impact: backend may not know the phone action succeeded")
        }
    }
}
