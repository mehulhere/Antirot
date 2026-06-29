import XCTest
@testable import Antirot

final class SettingsStoreTests: XCTestCase {
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
}
