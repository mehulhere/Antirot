import XCTest
@testable import Antirot

final class SettingsStoreTests: XCTestCase {
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
