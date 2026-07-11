import CoreGraphics
import XCTest
@testable import Antirot

final class CinematicLayoutTests: XCTestCase {
    func testCoachChatFloatsAboveBottomNavigation() {
        XCTAssertGreaterThan(AppBottomBarMetrics.coachChatClearance, AppBottomBarMetrics.bottomPadding)
        XCTAssertEqual(AppBottomBarMetrics.horizontalPadding, 0, accuracy: 0.1)
        XCTAssertEqual(AppBottomBarMetrics.coachChatClearance, 76, accuracy: 0.1)
        XCTAssertEqual(AppBottomBarMetrics.barHeight, 64, accuracy: 0.1)
        XCTAssertGreaterThanOrEqual(AppBottomBarMetrics.minimumHitTarget, 44)
        XCTAssertFalse(AppBottomBarMetrics.usesFullScreenHitTestOverlay)
    }

    func testEditorialSystemUsesApprovedInkPalette() {
        XCTAssertEqual(AntirotEditorialPalette.inkRed, 8.0 / 255.0, accuracy: 0.001)
        XCTAssertEqual(AntirotEditorialPalette.inkGreen, 8.0 / 255.0, accuracy: 0.001)
        XCTAssertEqual(AntirotEditorialPalette.inkBlue, 7.0 / 255.0, accuracy: 0.001)
        XCTAssertEqual(AntirotEditorialPalette.signalOrangeRed, 228.0 / 255.0, accuracy: 0.001)
    }

    func testEditorialSystemCapsActiveCornerRadius() {
        XCTAssertEqual(AntirotEditorialMetrics.maximumRadius, 16, accuracy: 0.1)
        XCTAssertLessThanOrEqual(AntirotCinematicMetrics.cardRadius, 16)
        XCTAssertLessThanOrEqual(AntirotCinematicMetrics.pillRadius, 16)
    }

    func testReferenceBottomBarHasFourTabs() {
        XCTAssertEqual(AppScreen.allCases.map(\.title), ["Coach", "Tasks", "Stats", "Settings"])
    }

    func testCoachStageSitsBelowHeaderBand() {
        XCTAssertGreaterThanOrEqual(CoachStageLayoutMetrics.imageVerticalPositionFraction, 0.47)
    }

    func testCoachStageUsesEditorialGeneratedBackgroundAsset() {
        XCTAssertEqual(CoachStageLayoutMetrics.backgroundAssetName, "AntirotCoachEditorial")
    }

    func testCoachHeaderDoesNotLeaveLargeTopGap() {
        XCTAssertLessThanOrEqual(HomeLayoutMetrics.headerTopPadding, 40)
    }

    func testCoachScreenDoesNotShowTopMenuShortcut() {
        XCTAssertFalse(AppChromeMetrics.showsCoachTopMenuShortcut)
    }
}
