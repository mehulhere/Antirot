import XCTest
@testable import Antirot

final class APIClientTests: XCTestCase {
    func testEndpointURLJoinsLeadingSlashPath() throws {
        let url = try APIClient.endpointURL(
            baseURL: URL(string: "https://api.antirot.org")!,
            path: "/v1/auth/google"
        )

        XCTAssertEqual(url.absoluteString, "https://api.antirot.org/v1/auth/google")
    }

    func testEndpointURLJoinsBasePathAndRoute() throws {
        let url = try APIClient.endpointURL(
            baseURL: URL(string: "https://api.antirot.org/api")!,
            path: "/v1/health"
        )

        XCTAssertEqual(url.absoluteString, "https://api.antirot.org/api/v1/health")
    }

    func testTransportErrorIncludesActionableDetails() {
        let error = APIClient.APIError.transportFailed(
            url: "https://api.antirot.org/v1/auth/google",
            underlying: "Could not connect to the server. | NSURLErrorDomain -1004 | URLError -1004"
        )

        XCTAssertEqual(error.shortMessage, "Backend network check failed")
        XCTAssertTrue(error.localizedDescription.contains("before an HTTP response"))
        XCTAssertTrue(error.localizedDescription.contains("NSURLErrorDomain -1004"))
        XCTAssertTrue(error.recoverySuggestion?.contains("/v1/health") == true)
    }
}
