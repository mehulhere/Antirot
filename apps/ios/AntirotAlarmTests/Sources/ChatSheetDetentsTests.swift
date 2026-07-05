import CoreGraphics
import XCTest
@testable import Antirot

final class ChatSheetDetentsTests: XCTestCase {
    func testCollapsedSheetUsesCompactPreviewHeight() {
        XCTAssertEqual(ChatSheetDetents.collapsedHeight, 72, accuracy: 0.1)
    }

    func testUpwardSwipeOpensDirectlyToFull() {
        let availableHeight: CGFloat = 800

        let full = ChatSheetDetents.nextExpandedHeight(
            from: ChatSheetDetents.collapsedHeight,
            availableHeight: availableHeight
        )

        XCTAssertEqual(full, 768, accuracy: 0.1)
    }

    func testDownwardSwipeCollapsesDirectly() {
        let availableHeight: CGFloat = 800

        let collapsed = ChatSheetDetents.nextCollapsedHeight(
            from: ChatSheetDetents.fullHeight(availableHeight: availableHeight),
            availableHeight: availableHeight
        )

        XCTAssertEqual(collapsed, ChatSheetDetents.collapsedHeight, accuracy: 0.1)
    }

    func testNoHalfDetentExists() {
        let availableHeight: CGFloat = 800

        XCTAssertEqual(
            ChatSheetDetents.heights(availableHeight: availableHeight),
            [
                ChatSheetDetents.collapsedHeight,
                ChatSheetDetents.fullHeight(availableHeight: availableHeight)
            ]
        )
    }

    func testCollapsedDetectionAllowsSmallFingerJitterNearRestingHeight() {
        XCTAssertTrue(ChatSheetDetents.isCollapsed(ChatSheetDetents.collapsedHeight))
        XCTAssertTrue(ChatSheetDetents.isCollapsed(ChatSheetDetents.collapsedHeight + 14))
        XCTAssertFalse(ChatSheetDetents.isCollapsed(ChatSheetDetents.collapsedHeight + 15))
    }

    func testCollapsedContentOnlyShowsWhenRestingCollapsedAndNotDraggingUp() {
        XCTAssertTrue(
            ChatSheetDetents.showsCollapsedContent(
                committedHeight: ChatSheetDetents.collapsedHeight,
                isDragging: false,
                dragBeganCollapsed: false
            )
        )
        XCTAssertFalse(
            ChatSheetDetents.showsCollapsedContent(
                committedHeight: 700,
                isDragging: false,
                dragBeganCollapsed: true
            )
        )
    }

    func testCollapsedContentFreezesToDragStartingState() {
        XCTAssertTrue(
            ChatSheetDetents.showsCollapsedContent(
                committedHeight: 700,
                isDragging: true,
                dragBeganCollapsed: true
            )
        )
        XCTAssertFalse(
            ChatSheetDetents.showsCollapsedContent(
                committedHeight: ChatSheetDetents.collapsedHeight,
                isDragging: true,
                dragBeganCollapsed: false
            )
        )
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
            572,
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

    func testUIKitPanEndSnapsUsingTranslationAndVelocity() {
        let availableHeight: CGFloat = 800

        XCTAssertEqual(
            ChatSheetDetents.finalHeight(
                from: ChatSheetDetents.collapsedHeight,
                translationY: -40,
                velocityY: 0,
                availableHeight: availableHeight
            ),
            ChatSheetDetents.fullHeight(availableHeight: availableHeight),
            accuracy: 0.1
        )
        XCTAssertEqual(
            ChatSheetDetents.finalHeight(
                from: ChatSheetDetents.fullHeight(availableHeight: availableHeight),
                translationY: 0,
                velocityY: 120,
                availableHeight: availableHeight
            ),
            ChatSheetDetents.collapsedHeight,
            accuracy: 0.1
        )
    }

    func testSmallUpwardReleaseStillFinishesFullSize() {
        let availableHeight: CGFloat = 800

        XCTAssertEqual(
            ChatSheetDetents.finalHeight(
                from: ChatSheetDetents.collapsedHeight,
                translationY: -80,
                velocityY: 0,
                availableHeight: availableHeight
            ),
            ChatSheetDetents.fullHeight(availableHeight: availableHeight),
            accuracy: 0.1
        )
    }

    func testCoachMessageMarksLocalAudioAsPlayableVoiceMessage() {
        let audioURL = URL(fileURLWithPath: "/tmp/antirot-voice-test.m4a")
        let voiceMessage = CoachMessage(role: .user, text: "Voice message", audioFileURL: audioURL)
        let textMessage = CoachMessage(role: .user, text: "Typed message")

        XCTAssertTrue(voiceMessage.isPlayableVoiceMessage)
        XCTAssertFalse(textMessage.isPlayableVoiceMessage)
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
