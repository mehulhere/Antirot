import CoreGraphics
import XCTest
@testable import Antirot

final class ChatSheetDetentsTests: XCTestCase {
    func testUpwardSwipesAdvanceFromCollapsedToHalfThenFull() {
        let availableHeight: CGFloat = 800

        let half = ChatSheetDetents.nextExpandedHeight(
            from: ChatSheetDetents.collapsedHeight,
            availableHeight: availableHeight
        )
        let full = ChatSheetDetents.nextExpandedHeight(
            from: half,
            availableHeight: availableHeight
        )

        XCTAssertEqual(half, 400, accuracy: 0.1)
        XCTAssertEqual(full, 768, accuracy: 0.1)
    }

    func testNearestDetentCanReturnFullHeight() {
        let availableHeight: CGFloat = 800

        XCTAssertEqual(
            ChatSheetDetents.nearestHeight(to: 730, availableHeight: availableHeight),
            768,
            accuracy: 0.1
        )
    }

    func testRoutineParserDoesNotInventDefaultCategories() {
        let content = """
        # Routine

        ## Personalized Categories
        - None yet. Add only recurring categories the user actually mentions.
        """

        XCTAssertTrue(RoutinePlanItem.parseMarkdown(content).isEmpty)
        XCTAssertTrue(RoutinePlanItem.defaultItems.isEmpty)
    }
}
