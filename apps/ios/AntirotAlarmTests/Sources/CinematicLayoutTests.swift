import CoreGraphics
import XCTest
@testable import Antirot

final class CinematicLayoutTests: XCTestCase {
    func testCoachChatFloatsAboveBottomNavigation() {
        XCTAssertGreaterThan(AppBottomBarMetrics.coachChatClearance, AppBottomBarMetrics.bottomPadding)
        XCTAssertEqual(AppBottomBarMetrics.horizontalPadding, 20, accuracy: 0.1)
        XCTAssertEqual(AppBottomBarMetrics.coachChatClearance, 82, accuracy: 0.1)
    }

    func testCinematicShellUsesCompactCornerRadii() {
        XCTAssertEqual(AntirotCinematicMetrics.cardRadius, 20, accuracy: 0.1)
        XCTAssertEqual(AntirotCinematicMetrics.pillRadius, 22, accuracy: 0.1)
    }
}
