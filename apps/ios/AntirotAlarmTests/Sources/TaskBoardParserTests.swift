import XCTest
@testable import Antirot

final class TaskBoardParserTests: XCTestCase {
    func testTaskPresentationNeverInventsDurations() {
        let pending = TaskBoardItem(id: "pending", title: "Write brief", detail: nil, status: .pending)
        let live = TaskBoardItem(id: "live", title: "Build", detail: "Estimated 35 minutes", status: .live)
        let done = TaskBoardItem(id: "done", title: "Ship", detail: nil, status: .done)

        XCTAssertNil(TaskBoardPresentation.durationText(for: pending))
        XCTAssertEqual(TaskBoardPresentation.durationText(for: live), "35m")
        XCTAssertEqual(TaskBoardPresentation.durationText(for: done), "Done")
    }

    func testFocusMinutesOnlyUseExplicitEstimates() {
        let items = [
            TaskBoardItem(id: "one", title: "One", detail: "Estimated 25 minutes", status: .live),
            TaskBoardItem(id: "two", title: "Two", detail: nil, status: .done)
        ]

        XCTAssertEqual(TaskBoardPresentation.totalFocusMinutes(items: items), 25)
    }

    func testParseSplitsPendingDoneAndLiveTasks() {
        let snapshot = TaskBoardParser.parse(
            tasksMarkdown: """
            # Task Pipeline
            - [ ] 2h - Finalize Antirot app
            - [x] Ship Google login
            """,
            workLogMarkdown: """
            # Work Log
            - session_start: Fix iOS state sync (estimated 15 mins) at 2026-07-05T10:00:00Z
            - session_end: 12 actual mins, productivity level 100% at 2026-07-05T10:12:00Z
            """,
            runtimeState: "working",
            runtimeMetadata: #"{"task_id":"Fix task board","estimated_minutes":25}"#
        )

        XCTAssertEqual(snapshot.live.map(\.title), ["Fix task board"])
        XCTAssertEqual(snapshot.live.first?.detail, "Estimated 25 minutes")
        XCTAssertEqual(snapshot.pending.map(\.title), ["Finalize Antirot app"])
        XCTAssertEqual(snapshot.done.map(\.title), ["Ship Google login", "Fix iOS state sync"])
    }

    func testNoLiveTaskOutsideWorkingState() {
        let snapshot = TaskBoardParser.parse(
            tasksMarkdown: "# Task Pipeline\n- [ ] Write tests\n",
            workLogMarkdown: "",
            runtimeState: "idle",
            runtimeMetadata: #"{"task_id":"Write tests","estimated_minutes":25}"#
        )

        XCTAssertTrue(snapshot.live.isEmpty)
        XCTAssertEqual(snapshot.pending.map(\.title), ["Write tests"])
    }

    func testTodayWorkLogKeyUsesUtcDate() {
        let date = Date(timeIntervalSince1970: 1_786_284_000)

        XCTAssertEqual(TaskBoardParser.todayWorkLogKey(now: date), "work_log_2026_08_09")
    }
}
