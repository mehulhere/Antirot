import Foundation

struct DiagnosticMemorySnapshot: Equatable {
    var key: String
    var previous: String?
    var content: String?
    var error: String?
}

struct DiagnosticsReporter {
    static let memoryKeys = [
        "personality",
        "user_profile",
        "durable",
        "longterm",
        "shortterm",
        "behavior",
        "tasks",
        "routine",
        "sleep",
        "achievements",
        "miscellaneous_todo",
        "work"
    ]

    static func lastExchanges(from messages: [CoachMessage], limit: Int = 3) -> [(user: String, coach: String)] {
        var exchanges: [(user: String, coach: String)] = []
        var pendingUser: String?

        for message in messages {
            switch message.role {
            case .user:
                pendingUser = message.text
            case .coach:
                if let user = pendingUser {
                    exchanges.append((user: user, coach: message.text))
                    pendingUser = nil
                }
            case .system:
                continue
            }
        }

        return Array(exchanges.suffix(limit))
    }

    static func changedMemoryRows(_ snapshots: [DiagnosticMemorySnapshot]) -> [DiagnosticMemorySnapshot] {
        snapshots.filter { snapshot in
            if snapshot.error != nil {
                return true
            }
            guard let previous = snapshot.previous, let content = snapshot.content else {
                return false
            }
            return previous != content
        }
    }

    static func summarizeMemoryDiff(previous: String, content: String) -> String {
        let previousLines = previous.split(separator: "\n", omittingEmptySubsequences: false)
        let contentLines = content.split(separator: "\n", omittingEmptySubsequences: false)
        let previousSet = Set(previousLines.map(String.init))
        let contentSet = Set(contentLines.map(String.init))
        let added = contentSet.subtracting(previousSet).count
        let removed = previousSet.subtracting(contentSet).count
        return "+\(added) / -\(removed) lines, \(content.count) chars"
    }

    static func buildMarkdown(
        messages: [CoachMessage],
        events: [ReportEventPayload],
        memorySnapshots: [DiagnosticMemorySnapshot],
        runtimeState: String,
        statusText: String,
        deviceId: String,
        now: Date = Date()
    ) -> String {
        let exchanges = lastExchanges(from: messages)
        let windowStart = events.map(\.at).min() ?? now
        let changedMemory = changedMemoryRows(memorySnapshots)

        let exchangeLines = exchanges.enumerated().map { index, exchange in
            [
                "### Exchange \(index + 1)",
                "- User: \(normalize(exchange.user))",
                "- Coach: \(normalize(exchange.coach))"
            ].joined(separator: "\n")
        }.joined(separator: "\n\n")

        let eventLines = events.suffix(20).map { event in
            var line = "- \(iso(event.at)) [\(event.kind)] \(event.summary)"
            if let detail = event.detail, !detail.isEmpty {
                line += " -- \(normalize(detail))"
            }
            return line
        }.joined(separator: "\n")

        let memoryLines = changedMemory.map { row in
            if let error = row.error {
                return "### \(row.key).md\nLoad failed: \(normalize(error))"
            }
            let previous = row.previous ?? ""
            let content = row.content ?? ""
            return "### \(row.key).md\n\(summarizeMemoryDiff(previous: previous, content: content))"
        }.joined(separator: "\n\n")

        return [
            "# Antirot iOS Diagnostics",
            "",
            "Created: \(iso(now))",
            "Window: \(iso(windowStart)) -> \(iso(now))",
            "Device: \(deviceId)",
            "Runtime state: \(runtimeState)",
            "Status: \(statusText)",
            "",
            "## Last 3 Exchanges",
            exchangeLines.isEmpty ? "No complete user/coach exchanges in this app session." : exchangeLines,
            "",
            "## Recent App Events",
            eventLines.isEmpty ? "No recorded app events in this app session." : eventLines,
            "",
            "## Changed Markdown Files",
            memoryLines.isEmpty ? "No observed markdown file changes since the previous diagnostics snapshot." : memoryLines
        ].joined(separator: "\n")
    }

    static func reportEvents(from events: [ReportEventPayload], now: Date = Date()) -> [ReportEventPayload] {
        let cutoff = now.addingTimeInterval(-30 * 60)
        return events.filter { $0.at >= cutoff }
    }

    private static func normalize(_ value: String) -> String {
        value
            .replacingOccurrences(of: "\n", with: " ")
            .replacingOccurrences(of: "  ", with: " ")
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private static func iso(_ date: Date) -> String {
        ISO8601DateFormatter().string(from: date)
    }
}
