import Foundation
import SwiftUI

struct TaskBoardView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var coach: CoachViewModel
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    @State private var liveTasks: [TaskBoardItem] = []
    @State private var pendingTasks: [TaskBoardItem] = []
    @State private var doneTasks: [TaskBoardItem] = []
    @State private var statusText = "Loading tasks..."
    @State private var isLoading = false
    @State private var selectedScope: TaskScope = .today

    private var client: APIClient {
        APIClient(baseURL: settings.baseURL, apiToken: settings.apiToken, userId: settings.userId)
    }

    var body: some View {
        CinematicScreen(
            title: "Tasks",
            subtitle: "Execute. No drift.",
            icon: "line.3.horizontal.decrease"
        ) {
            taskScopePicker
            taskOverviewStrip
            priorityTaskCard
            taskListCard
            taskSummaryCard
            quoteCard
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
        HStack(spacing: 4) {
            ForEach(TaskScope.allCases, id: \.self) { scope in
                Button {
                    withAnimation(reduceMotion ? .easeOut(duration: 0.14) : .spring(response: 0.22, dampingFraction: 0.86)) {
                        selectedScope = scope
                    }
                } label: {
                    Text(scope.title)
                        .font(.subheadline.weight(.bold))
                        .foregroundStyle(selectedScope == scope ? .arTextPrimary : .arTextSecondary)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 12)
                        .background(
                            Capsule(style: .continuous)
                                .fill(selectedScope == scope ? Color.white.opacity(0.10) : Color.clear)
                        )
                        .overlay(alignment: .bottom) {
                            Capsule(style: .continuous)
                                .fill(selectedScope == scope ? Color.arAccent : Color.clear)
                                .frame(width: 18, height: 2)
                                .padding(.bottom, 4)
                        }
                }
                .buttonStyle(.plain)
            }
        }
        .padding(5)
        .smokedGlass(cornerRadius: AntirotCinematicMetrics.pillRadius, tint: .arSurface, shadow: false)
    }

    private var taskOverviewStrip: some View {
        HStack(spacing: 0) {
            taskOverviewMetric(value: "\(liveTasks.count)", label: "Live", tint: .arAccent)
            overviewDivider
            taskOverviewMetric(value: "\(pendingTasks.count)", label: "Pending", tint: .arAmber)
            overviewDivider
            taskOverviewMetric(value: "\(doneTasks.count)", label: "Done", tint: .arSuccess)
        }
        .padding(.vertical, 12)
        .smokedGlass(cornerRadius: 20, tint: .arSurface, shadow: false)
    }

    private func taskOverviewMetric(value: String, label: String, tint: Color) -> some View {
        VStack(spacing: 3) {
            Text(value)
                .font(.title3.weight(.bold))
                .foregroundStyle(tint)
            Text(label.uppercased())
                .font(.caption2.weight(.bold))
                .tracking(0.8)
                .foregroundStyle(.arTextSecondary)
        }
        .frame(maxWidth: .infinity)
    }

    private var overviewDivider: some View {
        Rectangle()
            .fill(Color.arBorder)
            .frame(width: 0.5, height: 34)
    }

    @ViewBuilder
    private var priorityTaskCard: some View {
        if let item = scopedItems.first {
            CinematicGlassCard(padding: 18, accent: item.tint) {
                VStack(alignment: .leading, spacing: 14) {
                    CinematicKicker(
                        title: statusTitle(for: item),
                        icon: item.systemImage,
                        tint: item.tint
                    )

                    Text(item.title)
                        .font(.title2.weight(.bold))
                        .fontDesign(.rounded)
                        .foregroundStyle(.arTextPrimary)
                        .fixedSize(horizontal: false, vertical: true)

                    if let detail = item.detail, !detail.isEmpty {
                        Text(detail)
                            .font(.subheadline)
                            .foregroundStyle(.arTextSecondary)
                            .fixedSize(horizontal: false, vertical: true)
                    }

                    HStack {
                        Label(durationText(for: item), systemImage: "timer")
                            .font(.caption.weight(.bold))
                            .foregroundStyle(.arTextSecondary)
                    }
                }
            }
        }
    }

    @ViewBuilder
    private var taskListCard: some View {
        let items = Array(scopedItems.dropFirst())

        if scopedItems.isEmpty {
            taskListSurface(items: [], emptyMessage: emptyText)
        } else if !items.isEmpty {
            taskListSurface(items: items, emptyMessage: nil)
        }
    }

    private func taskListSurface(items: [TaskBoardItem], emptyMessage: String?) -> some View {
        CinematicGlassCard(padding: 0, accent: .arAccent) {
            VStack(spacing: 0) {
                HStack {
                    Text(dayTitle)
                        .font(.subheadline.weight(.bold))
                        .foregroundStyle(.arTextPrimary)
                    Spacer()
                    Text("\(focusMinutesText) Focus")
                        .font(.caption.weight(.bold))
                        .foregroundStyle(.arAccent)
                }
                .padding(.horizontal, 14)
                .padding(.top, 14)
                .padding(.bottom, 8)

                if let emptyMessage {
                    Text(emptyMessage)
                        .font(.subheadline)
                        .foregroundStyle(.arTextMuted)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.horizontal, 14)
                        .padding(.vertical, 18)
                } else {
                    ForEach(Array(items.enumerated()), id: \.element.id) { index, item in
                        referenceTaskRow(item)
                        if index < items.count - 1 {
                            SectionDivider()
                                .padding(.leading, 56)
                        }
                    }
                }
            }
        }
    }

    private var taskSummaryCard: some View {
        CinematicGlassCard(padding: 16, accent: .arAccent) {
            HStack(spacing: 14) {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Today's Focus")
                        .font(.subheadline.weight(.bold))
                        .foregroundStyle(.arTextPrimary)
                    Text("\(focusMinutesText) / 4h 0m")
                        .font(.title3.weight(.semibold))
                        .foregroundStyle(.arTextPrimary)
                    ProgressView(value: min(Double(totalFocusMinutes) / 240.0, 1.0))
                        .tint(.arAccent)
                }

                Spacer(minLength: 8)

                ZStack {
                    Circle()
                        .stroke(Color.white.opacity(0.08), lineWidth: 7)
                    Circle()
                        .trim(from: 0, to: min(CGFloat(totalFocusMinutes) / 240.0, 1.0))
                        .stroke(Color.arAccent, style: StrokeStyle(lineWidth: 7, lineCap: .round))
                        .rotationEffect(.degrees(-90))
                    Text("\(Int(min(Double(totalFocusMinutes) / 240.0, 1.0) * 100))%")
                        .font(.headline.weight(.bold))
                        .foregroundStyle(.arTextPrimary)
                }
                .frame(width: 68, height: 68)
            }
        }
    }

    private var quoteCard: some View {
        CinematicGlassCard(padding: 16, accent: .arAccent) {
            VStack(alignment: .leading, spacing: 10) {
                Text("Discipline is the bridge between goals and results.")
                    .font(.system(size: 19, weight: .semibold, design: .rounded))
                    .foregroundStyle(.arTextPrimary)
                    .fixedSize(horizontal: false, vertical: true)
                Text("- Antirot")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.arTextSecondary)
            }
        }
        .overlay(alignment: .bottom) {
            Button {
                coach.draft = "Add a new task."
            } label: {
                Image(systemName: "plus")
                    .font(.title2.weight(.bold))
                    .foregroundStyle(.white)
                    .frame(width: 58, height: 58)
                    .background(Circle().fill(Color.arAccent))
                    .shadow(color: Color.arAccent.opacity(0.32), radius: 20, y: 8)
            }
            .buttonStyle(.plain)
            .offset(y: 28)
        }
        .padding(.bottom, 30)
    }

    private func referenceTaskRow(_ item: TaskBoardItem) -> some View {
        HStack(alignment: .center, spacing: 12) {
            Image(systemName: item.systemImage)
                .font(.title3.weight(.semibold))
                .foregroundStyle(item.tint)
                .frame(width: 28)

            VStack(alignment: .leading, spacing: 4) {
                Text(item.title)
                    .font(.system(size: 16, weight: .semibold, design: .rounded))
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

            Text(durationText(for: item))
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(.arTextSecondary)

            Image(systemName: "chevron.right")
                .font(.caption.weight(.bold))
                .foregroundStyle(.arTextMuted)
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 12)
    }

    private var scopedItems: [TaskBoardItem] {
        switch selectedScope {
        case .today:
            let base = liveTasks + Array(pendingTasks.prefix(4))
            return base.isEmpty ? pendingTasks : Array(base)
        case .upcoming:
            return Array(pendingTasks.dropFirst(min(4, pendingTasks.count)))
        case .backlog:
            return pendingTasks
        }
    }

    private var emptyText: String {
        switch selectedScope {
        case .today:
            return "No tasks for today."
        case .upcoming:
            return "No upcoming tasks."
        case .backlog:
            return "No backlog."
        }
    }

    private var dayTitle: String {
        Date().formatted(.dateTime.weekday(.wide).day().month(.wide))
    }

    private var totalFocusMinutes: Int {
        let liveEstimate = liveTasks.compactMap { estimatedMinutes(from: $0.detail) }.reduce(0, +)
        let doneEstimate = doneTasks.count * 45
        return max(liveEstimate + doneEstimate, liveTasks.isEmpty ? 0 : 30)
    }

    private var focusMinutesText: String {
        let hours = totalFocusMinutes / 60
        let minutes = totalFocusMinutes % 60
        if hours == 0 { return "\(minutes)m" }
        return "\(hours)h \(minutes)m"
    }

    private func durationText(for item: TaskBoardItem) -> String {
        if let minutes = estimatedMinutes(from: item.detail) {
            return "\(minutes)m"
        }
        switch item.status {
        case .live: return "120m"
        case .pending: return "45m"
        case .done: return "Done"
        }
    }

    private func statusTitle(for item: TaskBoardItem) -> String {
        switch item.status {
        case .live: return "Live"
        case .pending: return "Pending"
        case .done: return "Done"
        }
    }

    private func estimatedMinutes(from detail: String?) -> Int? {
        guard let detail else { return nil }
        let digits = detail.filter { $0.isNumber }
        return Int(digits)
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

private enum TaskScope: CaseIterable {
    case today
    case upcoming
    case backlog

    var title: String {
        switch self {
        case .today: return "Today"
        case .upcoming: return "Upcoming"
        case .backlog: return "Backlog"
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
}
