import XCTest
@testable import Antirot

final class DiagnosticsReporterTests: XCTestCase {
    func testLastExchangesReturnsOnlyLatestThreeUserCoachPairs() {
        let messages = [
            CoachMessage(role: .coach, text: "Intro"),
            CoachMessage(role: .user, text: "User 1"),
            CoachMessage(role: .coach, text: "Coach 1"),
            CoachMessage(role: .system, text: "System noise"),
            CoachMessage(role: .user, text: "User 2"),
            CoachMessage(role: .coach, text: "Coach 2"),
            CoachMessage(role: .user, text: "User 3"),
            CoachMessage(role: .coach, text: "Coach 3"),
            CoachMessage(role: .user, text: "User 4"),
            CoachMessage(role: .coach, text: "Coach 4")
        ]

        let exchanges = DiagnosticsReporter.lastExchanges(from: messages)

        XCTAssertEqual(exchanges.count, 3)
        XCTAssertEqual(exchanges[0].user, "User 2")
        XCTAssertEqual(exchanges[2].coach, "Coach 4")
    }

    func testChangedMemoryRowsSkipUnchangedAndUnobservedRows() {
        let rows = [
            DiagnosticMemorySnapshot(key: "tasks", previous: "a", content: "a", error: nil),
            DiagnosticMemorySnapshot(key: "routine", previous: "a", content: "b", error: nil),
            DiagnosticMemorySnapshot(key: "sleep", previous: nil, content: "fresh", error: nil),
            DiagnosticMemorySnapshot(key: "durable", previous: nil, content: nil, error: "failed")
        ]

        let changed = DiagnosticsReporter.changedMemoryRows(rows)

        XCTAssertEqual(changed.map(\.key), ["routine", "durable"])
    }

    func testReportMarkdownIncludesStateAndChangedFilesOnly() {
        let now = Date(timeIntervalSince1970: 1_700_000_000)
        let messages = [
            CoachMessage(role: .user, text: "Start"),
            CoachMessage(role: .coach, text: "Work.")
        ]
        let rows = [
            DiagnosticMemorySnapshot(key: "tasks", previous: "same", content: "same", error: nil),
            DiagnosticMemorySnapshot(key: "routine", previous: "old", content: "new", error: nil)
        ]

        let markdown = DiagnosticsReporter.buildMarkdown(
            messages: messages,
            events: [
                ReportEventPayload(at: now, kind: "state.changed", summary: "idle -> working", detail: nil)
            ],
            memorySnapshots: rows,
            runtimeState: "working",
            statusText: "Ready",
            deviceId: "device-1",
            now: now
        )

        XCTAssertTrue(markdown.contains("Runtime state: working"))
        XCTAssertTrue(markdown.contains("### routine.md"))
        XCTAssertFalse(markdown.contains("### tasks.md"))
        XCTAssertTrue(markdown.contains("idle -> working"))
    }
}
