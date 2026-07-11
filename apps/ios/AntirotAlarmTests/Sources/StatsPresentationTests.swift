import CoreGraphics
import XCTest
@testable import Antirot

final class StatsPresentationTests: XCTestCase {
    func testZeroWorkShowsZeroProgress() {
        XCTAssertEqual(StatsPresentation.goalRatio(workMinutes: 0), 0, accuracy: 0.001)
    }

    func testGoalProgressIsClamped() {
        XCTAssertEqual(StatsPresentation.goalRatio(workMinutes: 120), 0.5, accuracy: 0.001)
        XCTAssertEqual(StatsPresentation.goalRatio(workMinutes: 400), 1, accuracy: 0.001)
    }
}
