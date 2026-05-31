import Foundation

enum PushTokenStore {
    private static let key = "apnsDeviceToken"

    static func currentToken(defaults: UserDefaults = .standard) -> String {
        defaults.string(forKey: key) ?? ""
    }

    static func save(_ token: String, defaults: UserDefaults = .standard) {
        defaults.set(token, forKey: key)
        NotificationCenter.default.post(
            name: .antirotPushTokenDidChange,
            object: nil,
            userInfo: ["token": token]
        )
    }

    static func clear(defaults: UserDefaults = .standard) {
        defaults.removeObject(forKey: key)
        NotificationCenter.default.post(
            name: .antirotPushTokenDidChange,
            object: nil,
            userInfo: ["token": ""]
        )
    }
}

extension Notification.Name {
    static let antirotPushTokenDidChange = Notification.Name("antirotPushTokenDidChange")
}
