import CoreGraphics
import XCTest
@testable import Antirot

final class CinematicLayoutTests: XCTestCase {
    func testCoachChatFloatsAboveBottomNavigation() {
        XCTAssertGreaterThan(AppBottomBarMetrics.coachChatClearance, AppBottomBarMetrics.bottomPadding)
        XCTAssertEqual(AppBottomBarMetrics.horizontalPadding, 14, accuracy: 0.1)
        XCTAssertEqual(AppBottomBarMetrics.coachChatClearance, 104, accuracy: 0.1)
        XCTAssertGreaterThanOrEqual(AppBottomBarMetrics.minimumHitTarget, 44)
        XCTAssertFalse(AppBottomBarMetrics.usesFullScreenHitTestOverlay)
    }

    func testSmokedGlassUsesApprovedWarmPalette() {
        XCTAssertEqual(AntirotPaletteValues.backgroundRed, 0.082, accuracy: 0.001)
        XCTAssertEqual(AntirotPaletteValues.backgroundGreen, 0.075, accuracy: 0.001)
        XCTAssertEqual(AntirotPaletteValues.backgroundBlue, 0.067, accuracy: 0.001)
        XCTAssertGreaterThan(AntirotPaletteValues.surfaceRed, AntirotPaletteValues.surfaceBlue)
    }

    func testSmokedGlassUsesGenerousContinuousCorners() {
        XCTAssertEqual(AntirotCinematicMetrics.cardRadius, 22, accuracy: 0.1)
        XCTAssertEqual(AntirotCinematicMetrics.pillRadius, 28, accuracy: 0.1)
    }

    func testReferenceBottomBarHasFourTabs() {
        XCTAssertEqual(AppScreen.allCases.map(\.title), ["Coach", "Tasks", "Stats", "Settings"])
    }

    func testCoachStageSitsBelowHeaderBand() {
        XCTAssertGreaterThanOrEqual(CoachStageLayoutMetrics.imageVerticalPositionFraction, 0.47)
    }

    func testCoachStageUsesWarmGeneratedBackgroundAsset() {
        XCTAssertEqual(CoachStageLayoutMetrics.backgroundAssetName, "AntirotCoachStageWarm")
    }

    func testCoachHeaderDoesNotLeaveLargeTopGap() {
        XCTAssertLessThanOrEqual(HomeLayoutMetrics.headerTopPadding, 40)
    }

    func testCoachScreenDoesNotShowTopMenuShortcut() {
        XCTAssertFalse(AppChromeMetrics.showsCoachTopMenuShortcut)
    }
}
