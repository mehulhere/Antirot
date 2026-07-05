import CoreGraphics
import XCTest
@testable import Antirot

final class CinematicLayoutTests: XCTestCase {
    func testCoachChatFloatsAboveBottomNavigation() {
        XCTAssertGreaterThan(AppBottomBarMetrics.coachChatClearance, AppBottomBarMetrics.bottomPadding)
        XCTAssertEqual(AppBottomBarMetrics.horizontalPadding, 12, accuracy: 0.1)
        XCTAssertEqual(AppBottomBarMetrics.coachChatClearance, 92, accuracy: 0.1)
    }

    func testCinematicShellUsesCompactCornerRadii() {
        XCTAssertEqual(AntirotCinematicMetrics.cardRadius, 16, accuracy: 0.1)
        XCTAssertEqual(AntirotCinematicMetrics.pillRadius, 24, accuracy: 0.1)
    }

    func testReferenceBottomBarHasFourTabs() {
        XCTAssertEqual(AppScreen.allCases.map(\.title), ["Coach", "Tasks", "Stats", "Settings"])
    }

    func testCoachStageSitsBelowHeaderBand() {
        XCTAssertGreaterThanOrEqual(CoachStageLayoutMetrics.imageVerticalPositionFraction, 0.47)
    }

    func testCoachScreenDoesNotShowTopMenuShortcut() {
        XCTAssertFalse(AppChromeMetrics.showsCoachTopMenuShortcut)
    }
}
