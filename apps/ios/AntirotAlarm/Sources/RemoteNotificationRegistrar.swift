import UIKit

@MainActor
enum RemoteNotificationRegistrar {
    static func register() {
        UIApplication.shared.registerForRemoteNotifications()
    }
}
