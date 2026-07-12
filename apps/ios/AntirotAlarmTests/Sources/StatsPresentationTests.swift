import CoreGraphics
import XCTest
@testable import Antirot

final class StatsPresentationTests: XCTestCase {
    func testZeroCompositionShowsZeroProgress() {
        XCTAssertEqual(StatsPresentation.compositionRatio(minutes: 0, total: 0), 0, accuracy: 0.001)
    }

    func testCompositionProgressIsClamped() {
        XCTAssertEqual(StatsPresentation.compositionRatio(minutes: 30, total: 60), 0.5, accuracy: 0.001)
        XCTAssertEqual(StatsPresentation.compositionRatio(minutes: 90, total: 60), 1, accuracy: 0.001)
    }
}
