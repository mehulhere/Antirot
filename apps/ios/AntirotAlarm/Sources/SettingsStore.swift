import Foundation

@MainActor
final class SettingsStore: ObservableObject {
    nonisolated static let defaultServerURL = "https://api.antirot.org"

    @Published var serverURL: String {
        didSet { defaults.set(serverURL, forKey: Keys.serverURL) }
    }

    @Published var apiToken: String {
        didSet { defaults.set(apiToken, forKey: Keys.apiToken) }
    }

    @Published var deviceId: String {
        didSet { defaults.set(deviceId, forKey: Keys.deviceId) }
    }

    @Published var userId: String {
        didSet { defaults.set(userId, forKey: Keys.userId) }
    }

    @Published var registered: Bool {
        didSet { defaults.set(registered, forKey: Keys.registered) }
    }

    @Published var alarmSoundName: String {
        didSet { defaults.set(alarmSoundName, forKey: Keys.alarmSoundName) }
    }

    @Published var alarmSoundMode: String {
        didSet { defaults.set(alarmSoundMode, forKey: Keys.alarmSoundMode) }
    }

    @Published var pushToken: String {
        didSet { defaults.set(pushToken, forKey: Keys.pushToken) }
    }

    @Published var onboardingName: String {
        didSet { defaults.set(onboardingName, forKey: Keys.onboardingName) }
    }

    @Published var onboardingNameSent: Bool {
        didSet { defaults.set(onboardingNameSent, forKey: Keys.onboardingNameSent) }
    }

    @Published var statusMessage: String = "Not registered"

    private let defaults: UserDefaults
    private var pushTokenObserver: NSObjectProtocol?

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        self.serverURL = Self.normalizedServerURL(defaults.string(forKey: Keys.serverURL))
        self.apiToken = defaults.string(forKey: Keys.apiToken) ?? ""
        self.deviceId = defaults.string(forKey: Keys.deviceId) ?? UUID().uuidString
        self.userId = defaults.string(forKey: Keys.userId) ?? "admin"
        self.registered = defaults.bool(forKey: Keys.registered)
        self.alarmSoundName = defaults.string(forKey: Keys.alarmSoundName) ?? ""
        self.alarmSoundMode = defaults.string(forKey: Keys.alarmSoundMode) ?? AlarmSoundMode.automatic.rawValue
        self.pushToken = PushTokenStore.currentToken(defaults: defaults)
        self.onboardingName = defaults.string(forKey: Keys.onboardingName) ?? ""
        self.onboardingNameSent = defaults.bool(forKey: Keys.onboardingNameSent)
            || !(defaults.string(forKey: Keys.onboardingName) ?? "").isEmpty
        self.pushTokenObserver = NotificationCenter.default.addObserver(
            forName: .antirotPushTokenDidChange,
            object: nil,
            queue: .main
        ) { [weak self] notification in
            let token = notification.userInfo?["token"] as? String ?? ""
            Task { @MainActor in
                self?.pushToken = token
            }
        }
    }

    deinit {
        if let pushTokenObserver {
            NotificationCenter.default.removeObserver(pushTokenObserver)
        }
    }

    var baseURL: URL? {
        URL(string: Self.normalizedServerURL(serverURL))
    }

    var effectiveServerURL: String {
        Self.normalizedServerURL(serverURL)
    }

    func resetBackendSession() {
        serverURL = Self.defaultServerURL
        apiToken = ""
        deviceId = UUID().uuidString
        userId = "admin"
        PushTokenStore.clear(defaults: defaults)
        pushToken = ""
        registered = false
        statusMessage = "Not registered"
        resetOnboardingNamePrompt()
    }

    func resetOnboardingNamePrompt() {
        onboardingName = ""
        onboardingNameSent = false
    }

    private static func normalizedServerURL(_ value: String?) -> String {
        let trimmed = value?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        guard !trimmed.isEmpty, let url = URL(string: trimmed), let host = url.host()?.lowercased() else {
            return defaultServerURL
        }
        if ["localhost", "127.0.0.1", "0.0.0.0", "::1"].contains(host) {
            return defaultServerURL
        }
        return trimmed
    }

    private enum Keys {
        static let serverURL = "serverURL"
        static let apiToken = "apiToken"
        static let deviceId = "deviceId"
        static let userId = "userId"
        static let registered = "registered"
        static let alarmSoundName = "alarmSoundName"
        static let alarmSoundMode = "alarmSoundMode"
        static let pushToken = "apnsDeviceToken"
        static let onboardingName = "antirot:onboardingName"
        static let onboardingNameSent = "antirot:onboardingNameSent"
    }
}

enum AlarmSoundMode: String, CaseIterable, Identifiable {
    case automatic
    case bundledNormal
    case bundledLoud
    case custom

    var id: String { rawValue }

    var label: String {
        switch self {
        case .automatic:
            return "Auto"
        case .bundledNormal:
            return "Normal"
        case .bundledLoud:
            return "Loud"
        case .custom:
            return "Custom"
        }
    }

    var detail: String {
        switch self {
        case .automatic:
            return "Normal alarms use normal sound. Loud/urgent alarms use loud sound."
        case .bundledNormal:
            return "Use the bundled normal sound for every alarm."
        case .bundledLoud:
            return "Use the bundled loud sound for every alarm."
        case .custom:
            return "Use your imported sound for every alarm."
        }
    }

    init(storedValue: String) {
        self = AlarmSoundMode(rawValue: storedValue) ?? .automatic
    }
}
