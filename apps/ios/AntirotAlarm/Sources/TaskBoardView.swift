import Foundation
import SwiftUI

struct TaskBoardView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var coach: CoachViewModel
    @EnvironmentObject private var navigation: AppNavigationModel
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    @State private var liveTasks: [TaskBoardItem] = []
    @State private var pendingTasks: [TaskBoardItem] = []
    @State private var doneTasks: [TaskBoardItem] = []
    @State private var statusText = "Loading tasks..."
    @State private var isLoading = false
    @State private var selectedScope: TaskScope = .inProgress

    private var client: APIClient {
        APIClient(baseURL: settings.baseURL, apiToken: settings.apiToken)
    }

    var body: some View {
        CinematicScreen(
            title: "Tasks",
            subtitle: "Execute. No drift.",
            icon: "line.3.horizontal.decrease"
        ) {
            taskScopePicker
            taskOverviewStrip
            priorityTask
            taskList
            addTaskButton
        }
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

    private var taskScopePicker: some View {
        HStack(spacing: 20) {
            ForEach(TaskScope.allCases, id: \.self) { scope in
                Button {
                    withAnimation(reduceMotion ? .easeOut(duration: 0.14) : .spring(response: 0.22, dampingFraction: 0.86)) {
                        selectedScope = scope
                    }
                } label: {
                    Text(scope.title)
                        .font(.system(size: 13, weight: .semibold, design: .monospaced))
                        .textCase(.uppercase)
                        .foregroundStyle(selectedScope == scope ? .arTextPrimary : .arTextSecondary)
                        .padding(.vertical, 10)
                        .overlay(alignment: .bottom) {
                            Rectangle()
                                .fill(selectedScope == scope ? Color.arAccent : Color.clear)
                                .frame(height: 2)
                        }
                }
                .buttonStyle(.plain)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .overlay(alignment: .bottom) {
            Rectangle().fill(Color.arBorder).frame(height: 1)
        }
    }

    private var taskOverviewStrip: some View {
        HStack(spacing: 18) {
            taskOverviewMetric(value: "\(liveTasks.count)", label: "Live", tint: .arAccent)
            taskOverviewMetric(value: "\(pendingTasks.count)", label: "Pending", tint: .arAmber)
            taskOverviewMetric(value: "\(doneTasks.count)", label: "Done", tint: .arSuccess)
            Spacer(minLength: 0)
        }
        .padding(.vertical, 4)
        .accessibilityElement(children: .combine)
    }

    private func taskOverviewMetric(value: String, label: String, tint: Color) -> some View {
        HStack(spacing: 5) {
            Text(value).foregroundStyle(tint)
            Text(label.uppercased()).foregroundStyle(.arTextSecondary)
        }
        .font(.system(size: 12, weight: .semibold, design: .monospaced))
        .tracking(0.6)
    }

    @ViewBuilder
    private var priorityTask: some View {
        if let item = scopedItems.first {
            VStack(alignment: .leading, spacing: 12) {
                Text(statusTitle(for: item).uppercased())
                    .font(.system(size: 11, weight: .bold, design: .monospaced))
                    .tracking(1.4)
                    .foregroundStyle(item.tint)

                Text(item.title)
                    .font(.system(.title, design: .serif, weight: .semibold))
                    .foregroundStyle(.arTextPrimary)
                    .fixedSize(horizontal: false, vertical: true)

                if let detail = item.detail, !detail.isEmpty {
                    Text(detail)
                        .font(.subheadline)
                        .foregroundStyle(.arTextSecondary)
                        .fixedSize(horizontal: false, vertical: true)
                }

                if let duration = TaskBoardPresentation.durationText(for: item) {
                    Text(duration)
                        .font(.system(size: 12, weight: .semibold, design: .monospaced))
                        .foregroundStyle(.arTextSecondary)
                }
            }
            .padding(.leading, 16)
            .padding(.vertical, 8)
            .overlay(alignment: .leading) {
                Rectangle().fill(Color.arAccent).frame(width: 3)
            }
        }
    }

    @ViewBuilder
    private var taskList: some View {
        let items = Array(scopedItems.dropFirst())

        if scopedItems.isEmpty {
            taskListSurface(items: [], emptyMessage: emptyText)
        } else if !items.isEmpty {
            taskListSurface(items: items, emptyMessage: nil)
        }
    }

    private func taskListSurface(items: [TaskBoardItem], emptyMessage: String?) -> some View {
        VStack(spacing: 0) {
            HStack {
                Text(selectedScope.title.uppercased())
                    .font(.system(size: 13, weight: .semibold, design: .monospaced))
                    .foregroundStyle(.arTextPrimary)
                Spacer()
                if totalFocusMinutes > 0 {
                    Text("\(focusMinutesText) logged")
                        .font(.system(size: 11, weight: .medium, design: .monospaced))
                        .foregroundStyle(.arAccent)
                }
            }
            .padding(.vertical, 12)
            .overlay(alignment: .bottom) {
                Rectangle().fill(Color.arBorder).frame(height: 1)
            }

            if let emptyMessage {
                Text(emptyMessage)
                    .font(.system(size: 20, weight: .regular, design: .serif))
                    .foregroundStyle(.arTextMuted)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.vertical, 28)
            } else {
                ForEach(Array(items.enumerated()), id: \.element.id) { index, item in
                    referenceTaskRow(item)
                    if index < items.count - 1 {
                        Rectangle().fill(Color.arBorder).frame(height: 1)
                    }
                }
            }
        }
    }

    private var addTaskButton: some View {
        Button {
            coach.draft = "Add a new task."
            navigation.selectedScreen = .coach
        } label: {
            Label("Add task with Coach", systemImage: "plus")
                .font(.system(size: 14, weight: .semibold, design: .monospaced))
                .textCase(.uppercase)
                .foregroundStyle(.arDeepBg)
                .frame(maxWidth: .infinity, minHeight: 52)
                .background(Color.arAccent, in: RoundedRectangle(cornerRadius: 4))
        }
        .buttonStyle(.plain)
    }

    private func referenceTaskRow(_ item: TaskBoardItem) -> some View {
        HStack(alignment: .center, spacing: 12) {
            Image(systemName: item.systemImage)
                .font(.title3.weight(.semibold))
                .foregroundStyle(item.tint)
                .frame(width: 28)

            VStack(alignment: .leading, spacing: 4) {
                Text(item.title)
                    .font(.body.weight(.semibold))
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

            if let duration = TaskBoardPresentation.durationText(for: item) {
                Text(duration)
                    .font(.system(size: 12, weight: .semibold, design: .monospaced))
                    .foregroundStyle(.arTextSecondary)
            }

        }
        .padding(.vertical, 14)
    }

    private var scopedItems: [TaskBoardItem] {
        switch selectedScope {
        case .inProgress: return liveTasks
        case .pending: return pendingTasks
        case .done: return doneTasks
        }
    }

    private var emptyText: String {
        switch selectedScope {
        case .inProgress: return "Nothing in progress."
        case .pending: return "No pending tasks."
        case .done: return "No completed tasks."
        }
    }

    private var totalFocusMinutes: Int {
        TaskBoardPresentation.totalRecordedMinutes(items: doneTasks)
    }

    private var focusMinutesText: String {
        let hours = totalFocusMinutes / 60
        let minutes = totalFocusMinutes % 60
        if hours == 0 { return "\(minutes)m" }
        return "\(hours)h \(minutes)m"
    }

    private func statusTitle(for item: TaskBoardItem) -> String {
        switch item.status {
        case .live: return "Live"
        case .pending: return "Pending"
        case .done: return "Done"
        }
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
    enum Status: Equatable {
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

enum TaskBoardPresentation {
    static func durationText(for item: TaskBoardItem) -> String? {
        if let minutes = recordedMinutes(from: item.detail) {
            return "\(minutes)m recorded"
        }
        if let minutes = estimatedMinutes(from: item.detail) {
            return "Estimated \(minutes)m"
        }
        return item.status == .done ? "Done" : nil
    }

    static func totalRecordedMinutes(items: [TaskBoardItem]) -> Int {
        items.compactMap { recordedMinutes(from: $0.detail) }.reduce(0, +)
    }

    private static func estimatedMinutes(from detail: String?) -> Int? {
        firstInteger(in: detail, pattern: #"Estimated\s+(\d+)\s+minutes"#)
    }

    private static func recordedMinutes(from detail: String?) -> Int? {
        firstInteger(in: detail, pattern: #"(\d+)\s+actual\s+mins"#)
    }

    private static func firstInteger(in detail: String?, pattern: String) -> Int? {
        guard let detail,
              let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]),
              let match = regex.firstMatch(
                in: detail,
                range: NSRange(detail.startIndex..., in: detail)
              ),
              let range = Range(match.range(at: 1), in: detail) else {
            return nil
        }
        return Int(detail[range])
    }
}

struct TaskBoardSnapshot: Equatable {
    var live: [TaskBoardItem]
    var pending: [TaskBoardItem]
    var done: [TaskBoardItem]
}

private enum TaskScope: CaseIterable {
    case inProgress
    case pending
    case done

    var title: String {
        switch self {
        case .inProgress: return "In progress"
        case .pending: return "Pending"
        case .done: return "Done"
        }
    }
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
        .environmentObject(AppNavigationModel())
}
