import XCTest
@testable import Antirot

@MainActor
final class SettingsStoreTests: XCTestCase {
    final class MemoryTokenStore: SecureTokenStoring, @unchecked Sendable {
        var token = ""

        func load() throws -> String {
            token
        }

        func save(_ token: String) throws {
            self.token = token
        }

        func clear() throws {
            token = ""
        }
    }

    func testGeneratedDeviceIdPersistsAcrossStoreInstances() {
        let suiteName = "SettingsStoreTests.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defer { defaults.removePersistentDomain(forName: suiteName) }

        let firstStore = SettingsStore(defaults: defaults)
        let secondStore = SettingsStore(defaults: defaults)

        XCTAssertFalse(firstStore.deviceId.isEmpty)
        XCTAssertEqual(secondStore.deviceId, firstStore.deviceId)
    }

    func testNormalizedServerURLStripsEndpointPath() {
        XCTAssertEqual(
            SettingsStore.normalizedServerURL("https://api.antirot.org/v1/auth/google"),
            "https://api.antirot.org"
        )
    }

    func testNormalizedServerURLRejectsLocalhost() {
        XCTAssertEqual(
            SettingsStore.normalizedServerURL("http://localhost:3000"),
            SettingsStore.defaultServerURL
        )
    }

    func testNormalizedServerURLPreservesRemotePort() {
        XCTAssertEqual(
            SettingsStore.normalizedServerURL("https://api.antirot.org:8443/debug"),
            "https://api.antirot.org:8443"
        )
    }

    func testAutoSnapshotOnStopDefaultsOnAndPersists() {
        let suiteName = "SettingsStoreTests.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defer { defaults.removePersistentDomain(forName: suiteName) }

        let firstStore = SettingsStore(defaults: defaults)
        XCTAssertTrue(firstStore.autoSnapshotOnStop)

        firstStore.autoSnapshotOnStop = false
        let secondStore = SettingsStore(defaults: defaults)
        XCTAssertFalse(secondStore.autoSnapshotOnStop)
    }

    func testLegacyUserDefaultsTokenMigratesToSecureStore() {
        let suiteName = "SettingsStoreTests.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        let tokenStore = MemoryTokenStore()
        defaults.set("legacy-device-token", forKey: "apiToken")
        defer { defaults.removePersistentDomain(forName: suiteName) }

        let store = SettingsStore(defaults: defaults, tokenStore: tokenStore)

        XCTAssertEqual(store.apiToken, "legacy-device-token")
        XCTAssertEqual(tokenStore.token, "legacy-device-token")
        XCTAssertNil(defaults.string(forKey: "apiToken"))
    }

    func testApiTokenUpdatesAndLogoutUseSecureStore() {
        let suiteName = "SettingsStoreTests.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        let tokenStore = MemoryTokenStore()
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let store = SettingsStore(defaults: defaults, tokenStore: tokenStore)

        store.apiToken = "new-device-token"
        XCTAssertEqual(tokenStore.token, "new-device-token")
        XCTAssertNil(defaults.string(forKey: "apiToken"))

        store.resetBackendSession()
        XCTAssertEqual(tokenStore.token, "")
    }
}
