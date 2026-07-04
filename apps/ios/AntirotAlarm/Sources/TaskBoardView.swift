import Foundation
import SwiftUI

struct TaskBoardView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var coach: CoachViewModel

    @State private var liveTasks: [TaskBoardItem] = []
    @State private var pendingTasks: [TaskBoardItem] = []
    @State private var doneTasks: [TaskBoardItem] = []
    @State private var statusText = "Loading tasks..."
    @State private var isLoading = false

    private var client: APIClient {
        APIClient(baseURL: settings.baseURL, apiToken: settings.apiToken, userId: settings.userId)
    }

    var body: some View {
        ScrollView(.vertical, showsIndicators: true) {
            VStack(alignment: .leading, spacing: 18) {
                VStack(alignment: .leading, spacing: 6) {
                    Text("Tasks")
                        .font(.largeTitle.weight(.bold))
                        .foregroundStyle(.arTextPrimary)
                    Text(statusText)
                        .font(.caption)
                        .foregroundStyle(.arTextSecondary)
                }
                .padding(.top, 86)

                taskSection(title: "Live", icon: "bolt.fill", items: liveTasks, emptyText: "No live work block.")
                taskSection(title: "Pending", icon: "circle", items: pendingTasks, emptyText: "No pending tasks.")
                taskSection(title: "Done", icon: "checkmark.circle.fill", items: doneTasks, emptyText: "No completed tasks found today.")
            }
            .padding(.horizontal, 24)
            .padding(.bottom, 36)
        }
        .background(Color.arBg.ignoresSafeArea())
        .refreshable {
            await loadTasks()
        }
        .task {
            await loadTasks()
        }
        .onChange(of: coach.runtimeState) {
            Task { await loadTasks() }
        }
    }

    private func taskSection(title: String, icon: String, items: [TaskBoardItem], emptyText: String) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            AntirotSectionHeader(title: title, icon: icon)

            VStack(spacing: 0) {
                if items.isEmpty {
                    Text(emptyText)
                        .font(.subheadline)
                        .foregroundStyle(.arTextMuted)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.horizontal, 14)
                        .padding(.vertical, 14)
                } else {
                    ForEach(Array(items.enumerated()), id: \.element.id) { index, item in
                        taskRow(item)
                        if index < items.count - 1 {
                            SectionDivider()
                                .padding(.leading, 48)
                        }
                    }
                }
            }
            .minimalCard(cornerRadius: 14, padding: 0)
        }
    }

    private func taskRow(_ item: TaskBoardItem) -> some View {
        HStack(alignment: .top, spacing: 12) {
            Image(systemName: item.systemImage)
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(item.tint)
                .frame(width: 24)

            VStack(alignment: .leading, spacing: 4) {
                Text(item.title)
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(.arTextPrimary)
                    .fixedSize(horizontal: false, vertical: true)

                if let detail = item.detail, !detail.isEmpty {
                    Text(detail)
                        .font(.caption)
                        .foregroundStyle(.arTextMuted)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }

            Spacer(minLength: 8)
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 12)
    }

    @MainActor
    private func loadTasks() async {
        isLoading = true
        defer { isLoading = false }

        do {
            async let tasksResponse = client.fetchMemory(key: "tasks")
            async let workLogResponse = client.fetchMemory(key: TaskBoardParser.todayWorkLogKey())
            async let stateResponse = client.fetchRuntimeState(deviceId: settings.deviceId)

            let tasks = try await tasksResponse
            let workLog = try? await workLogResponse
            let state = try await stateResponse

            let parsed = TaskBoardParser.parse(
                tasksMarkdown: tasks.content,
                workLogMarkdown: workLog?.content ?? "",
                runtimeState: state.runtimeState?.state ?? coach.runtimeState,
                runtimeMetadata: state.runtimeState?.metadata ?? coach.runtimeMetadata
            )

            liveTasks = parsed.live
            pendingTasks = parsed.pending
            doneTasks = parsed.done
            statusText = "Live \(liveTasks.count) | Pending \(pendingTasks.count) | Done \(doneTasks.count)"
        } catch {
            let parsed = TaskBoardParser.parse(
                tasksMarkdown: "",
                workLogMarkdown: "",
                runtimeState: coach.runtimeState,
                runtimeMetadata: coach.runtimeMetadata
            )
            liveTasks = parsed.live
            pendingTasks = parsed.pending
            doneTasks = parsed.done
            statusText = "Task load failed: \(error.localizedDescription)"
        }
    }
}

struct TaskBoardItem: Identifiable, Equatable {
    enum Status {
        case live
        case pending
        case done
    }

    var id: String
    var title: String
    var detail: String?
    var status: Status

    var systemImage: String {
        switch status {
        case .live:
            return "bolt.fill"
        case .pending:
            return "circle"
        case .done:
            return "checkmark.circle.fill"
        }
    }

    var tint: Color {
        switch status {
        case .live:
            return .arAccent
        case .pending:
            return .arTextSecondary
        case .done:
            return .arSuccess
        }
    }
}

struct TaskBoardSnapshot: Equatable {
    var live: [TaskBoardItem]
    var pending: [TaskBoardItem]
    var done: [TaskBoardItem]
}

enum TaskBoardParser {
    static func parse(
        tasksMarkdown: String,
        workLogMarkdown: String,
        runtimeState: String,
        runtimeMetadata: String?
    ) -> TaskBoardSnapshot {
        var pending: [TaskBoardItem] = []
        var done: [TaskBoardItem] = []

        for (index, line) in tasksMarkdown.split(separator: "\n").enumerated() {
            guard let parsed = parseTaskLine(String(line), index: index) else { continue }
            switch parsed.status {
            case .pending:
                pending.append(parsed)
            case .done:
                done.append(parsed)
            case .live:
                break
            }
        }

        let live = liveTask(runtimeState: runtimeState, runtimeMetadata: runtimeMetadata)
        done.append(contentsOf: completedWorkLogItems(workLogMarkdown))

        return TaskBoardSnapshot(
            live: live.map { [$0] } ?? [],
            pending: pending,
            done: Array(done.prefix(30))
        )
    }

    static func todayWorkLogKey(now: Date = Date()) -> String {
        let formatter = DateFormatter()
        formatter.calendar = Calendar(identifier: .gregorian)
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = TimeZone(secondsFromGMT: 0)
        formatter.dateFormat = "yyyy_MM_dd"
        return "work_log_\(formatter.string(from: now))"
    }

    private static func parseTaskLine(_ line: String, index: Int) -> TaskBoardItem? {
        let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.hasPrefix("- [") || trimmed.hasPrefix("* [") else { return nil }
        guard let closeIndex = trimmed.firstIndex(of: "]") else { return nil }
        let markerStart = trimmed.index(trimmed.startIndex, offsetBy: 3)
        let marker = String(trimmed[markerStart..<closeIndex]).trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        let rawTitle = String(trimmed[trimmed.index(after: closeIndex)...])
            .trimmingCharacters(in: .whitespacesAndNewlines)
        let title = cleanTaskTitle(rawTitle)
        guard !title.isEmpty else { return nil }

        let status: TaskBoardItem.Status = marker == "x" ? .done : .pending
        return TaskBoardItem(
            id: "tasks-\(index)-\(status)-\(title)",
            title: title,
            detail: status == .done ? "Marked done in tasks.md" : nil,
            status: status
        )
    }

    private static func liveTask(runtimeState: String, runtimeMetadata: String?) -> TaskBoardItem? {
        guard runtimeState.lowercased() == "working" else { return nil }
        let metadata = runtimeMetadata
            .flatMap { $0.data(using: .utf8) }
            .flatMap { try? JSONSerialization.jsonObject(with: $0) as? [String: Any] } ?? [:]
        let rawTitle = metadata["task_id"] as? String
        let title = cleanTaskTitle(rawTitle ?? "Current work block")
        let estimated = metadata["estimated_minutes"] as? Int
        let detail = estimated.map { "Estimated \($0) minutes" }

        return TaskBoardItem(
            id: "live-\(title)",
            title: title,
            detail: detail,
            status: .live
        )
    }

    private static func completedWorkLogItems(_ workLogMarkdown: String) -> [TaskBoardItem] {
        var items: [TaskBoardItem] = []
        var lastStartedTask: String?

        for (index, line) in workLogMarkdown.split(separator: "\n").enumerated() {
            let text = String(line)
            if let started = parseWorkLogTask(text) {
                lastStartedTask = started
            } else if text.contains("session_end:") {
                let title = lastStartedTask ?? "Completed work session"
                items.append(TaskBoardItem(
                    id: "worklog-\(index)-\(title)",
                    title: title,
                    detail: cleanWorkLogDetail(text),
                    status: .done
                ))
                lastStartedTask = nil
            }
        }

        return items.reversed()
    }

    private static func parseWorkLogTask(_ line: String) -> String? {
        guard let range = line.range(of: "session_start:") else { return nil }
        let remainder = line[range.upperBound...]
        let beforeEstimate = remainder.components(separatedBy: " (estimated ").first ?? String(remainder)
        return cleanTaskTitle(beforeEstimate)
    }

    private static func cleanWorkLogDetail(_ line: String) -> String {
        line
            .replacingOccurrences(of: "- session_end:", with: "")
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private static func cleanTaskTitle(_ value: String) -> String {
        var title = value.trimmingCharacters(in: .whitespacesAndNewlines)
        if title.hasPrefix("-") {
            title = String(title.dropFirst()).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if let hoursRange = title.range(of: #"^\d+(\.\d+)?h\s*-\s*"#, options: .regularExpression) {
            title.removeSubrange(hoursRange)
        }
        if let minutesRange = title.range(of: #"^\d+\s*(min|mins|minutes)\s*-\s*"#, options: .regularExpression) {
            title.removeSubrange(minutesRange)
        }
        return title.trimmingCharacters(in: .whitespacesAndNewlines)
    }
}

#Preview {
    TaskBoardView()
        .environmentObject(SettingsStore())
        .environmentObject(CoachViewModel())
}
