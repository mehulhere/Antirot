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

    func testDownwardSwipesCollapseFromFullToHalfThenCollapsed() {
        let availableHeight: CGFloat = 800

        let half = ChatSheetDetents.nextCollapsedHeight(
            from: ChatSheetDetents.fullHeight(availableHeight: availableHeight),
            availableHeight: availableHeight
        )
        let collapsed = ChatSheetDetents.nextCollapsedHeight(
            from: half,
            availableHeight: availableHeight
        )

        XCTAssertEqual(half, 400, accuracy: 0.1)
        XCTAssertEqual(collapsed, ChatSheetDetents.collapsedHeight, accuracy: 0.1)
    }

    func testNearestDetentCanReturnFullHeight() {
        let availableHeight: CGFloat = 800

        XCTAssertEqual(
            ChatSheetDetents.nearestHeight(to: 730, availableHeight: availableHeight),
            768,
            accuracy: 0.1
        )
    }

    func testLiveDragHeightTracksFingerAndClampsToSheetBounds() {
        let availableHeight: CGFloat = 800

        XCTAssertEqual(
            ChatSheetDetents.liveHeight(
                from: ChatSheetDetents.collapsedHeight,
                translationY: -500,
                availableHeight: availableHeight
            ),
            618,
            accuracy: 0.1
        )
        XCTAssertEqual(
            ChatSheetDetents.liveHeight(
                from: 400,
                translationY: 900,
                availableHeight: availableHeight
            ),
            ChatSheetDetents.collapsedHeight,
            accuracy: 0.1
        )
        XCTAssertEqual(
            ChatSheetDetents.liveHeight(
                from: 400,
                translationY: -900,
                availableHeight: availableHeight
            ),
            ChatSheetDetents.fullHeight(availableHeight: availableHeight),
            accuracy: 0.1
        )
    }

    func testDragEndSnapsDownToCollapsedOrUpToFull() {
        let availableHeight: CGFloat = 800

        XCTAssertEqual(
            ChatSheetDetents.finalHeight(
                from: 400,
                predictedEndTranslationY: 1_600,
                availableHeight: availableHeight
            ),
            ChatSheetDetents.collapsedHeight,
            accuracy: 0.1
        )
        XCTAssertEqual(
            ChatSheetDetents.finalHeight(
                from: 400,
                predictedEndTranslationY: -1_600,
                availableHeight: availableHeight
            ),
            ChatSheetDetents.fullHeight(availableHeight: availableHeight),
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
