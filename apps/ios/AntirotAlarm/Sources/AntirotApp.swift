import SwiftUI
import UserNotifications

@main
struct AntirotApp: App {
    @UIApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    @StateObject private var settings = SettingsStore()
    @StateObject private var alarmCenter = AlarmCenter()
    @StateObject private var coach = CoachViewModel()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(settings)
                .environmentObject(alarmCenter)
                .environmentObject(coach)
                .task {
                    await alarmCenter.configure(settings: settings)
                }
                .onOpenURL { url in
                    _ = GoogleAuthCenter.handle(url: url)
                }
        }
    }
}

final class AppDelegate: NSObject, UIApplicationDelegate, UNUserNotificationCenterDelegate {
    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil
    ) -> Bool {
        UNUserNotificationCenter.current().delegate = self
        AlarmNotificationActions.register()
        return true
    }

    func application(
        _ application: UIApplication,
        didRegisterForRemoteNotificationsWithDeviceToken deviceToken: Data
    ) {
        PushTokenStore.save(deviceToken.map { String(format: "%02x", $0) }.joined())
        Task { @MainActor in
            let settings = SettingsStore()
            guard !settings.apiToken.isEmpty else { return }

            let client = APIClient(baseURL: settings.baseURL, apiToken: settings.apiToken)
            do {
                let response = try await client.registerDevice(DeviceRegistrationRequest(
                    deviceId: settings.deviceId,
                    platform: "ios",
                    appVersion: Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "0.1.0",
                    notificationCapability: "remote_notification",
                    usageCapability: await ScreenTimeCenter.currentCapability(),
                    pushProvider: "apns",
                    pushToken: settings.pushToken.isEmpty ? nil : settings.pushToken
                ))
                settings.registered = response.ok
                settings.statusMessage = response.message ?? "Registered as \(response.deviceId)"
            } catch {
                print("🔴 FALLBACK: APNs token backend registration failed - Reason: \(error.localizedDescription) - Impact: backend wake pushes may not reach this device until next manual registration")
            }
        }
    }

    func application(
        _ application: UIApplication,
        didFailToRegisterForRemoteNotificationsWithError error: Error
    ) {
        print("🔴 FALLBACK: APNs registration failed - Reason: \(error.localizedDescription) - Impact: backend wake pushes cannot reach this device")
    }

    func application(
        _ application: UIApplication,
        didReceiveRemoteNotification userInfo: [AnyHashable: Any],
        fetchCompletionHandler completionHandler: @escaping (UIBackgroundFetchResult) -> Void
    ) {
        Task { @MainActor in
            let settings = SettingsStore()
            guard !settings.apiToken.isEmpty else {
                print("🔴 FALLBACK: APNs wake ignored - Reason: device is not signed in - Impact: alarm remains pending until login/poll")
                completionHandler(.noData)
                return
            }

            let alarmCenter = AlarmCenter()
            await alarmCenter.configure(settings: settings)
            await alarmCenter.pollPendingAlarms()
            completionHandler(alarmCenter.lastErrorDetails == nil ? .newData : .failed)
        }
    }

    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification
    ) async -> UNNotificationPresentationOptions {
        [.banner, .list, .sound]
    }

    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse
    ) async {
        await AlarmNotificationActions.handle(response: response)
    }
}
