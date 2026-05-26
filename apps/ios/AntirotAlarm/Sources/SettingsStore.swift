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

    @Published var statusMessage: String = "Not registered"

    private let defaults: UserDefaults

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        self.serverURL = defaults.string(forKey: Keys.serverURL) ?? ""
        self.apiToken = defaults.string(forKey: Keys.apiToken) ?? ""
        self.deviceId = defaults.string(forKey: Keys.deviceId) ?? UUID().uuidString
        self.registered = defaults.bool(forKey: Keys.registered)
    }

    var baseURL: URL? {
        URL(string: serverURL.trimmingCharacters(in: .whitespacesAndNewlines))
    }

    private enum Keys {
        static let serverURL = "serverURL"
        static let apiToken = "apiToken"
        static let deviceId = "deviceId"
        static let registered = "registered"
    }
}
