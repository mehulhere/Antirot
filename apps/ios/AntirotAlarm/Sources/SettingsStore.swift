import Foundation

@MainActor
final class SettingsStore: ObservableObject {
    @Published var serverURL: String {
        didSet { defaults.set(serverURL, forKey: Keys.serverURL) }
    }

    @Published var apiToken: String {
        didSet { defaults.set(apiToken, forKey: Keys.apiToken) }
    }

    @Published var deviceId: String {
        didSet { defaults.set(deviceId, forKey: Keys.deviceId) }
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

    @Published var statusMessage: String = "Not registered"

    private let defaults: UserDefaults

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        self.serverURL = defaults.string(forKey: Keys.serverURL) ?? ""
        self.apiToken = defaults.string(forKey: Keys.apiToken) ?? ""
        self.deviceId = defaults.string(forKey: Keys.deviceId) ?? UUID().uuidString
        self.registered = defaults.bool(forKey: Keys.registered)
        self.alarmSoundName = defaults.string(forKey: Keys.alarmSoundName) ?? ""
        self.alarmSoundMode = defaults.string(forKey: Keys.alarmSoundMode) ?? AlarmSoundMode.automatic.rawValue
    }

    var baseURL: URL? {
        URL(string: serverURL.trimmingCharacters(in: .whitespacesAndNewlines))
    }

    private enum Keys {
        static let serverURL = "serverURL"
        static let apiToken = "apiToken"
        static let deviceId = "deviceId"
        static let registered = "registered"
        static let alarmSoundName = "alarmSoundName"
        static let alarmSoundMode = "alarmSoundMode"
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
            "Auto"
        case .bundledNormal:
            "Normal"
        case .bundledLoud:
            "Loud"
        case .custom:
            "Custom"
        }
    }

    var detail: String {
        switch self {
        case .automatic:
            "Normal alarms use normal sound. Loud/urgent alarms use loud sound."
        case .bundledNormal:
            "Use the bundled normal sound for every alarm."
        case .bundledLoud:
            "Use the bundled loud sound for every alarm."
        case .custom:
            "Use your imported sound for every alarm."
        }
    }

    init(storedValue: String) {
        self = AlarmSoundMode(rawValue: storedValue) ?? .automatic
    }
}
