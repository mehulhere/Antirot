import SwiftUI

struct StatsView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var coach: CoachViewModel
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    @State private var stats: StatsResponse?
    @State private var statusText = "Loading stats..."
    @State private var summaryText = ""
    @State private var isLoading = false
    @State private var isSummarizing = false
    @State private var selectedPeriod: StatsScope = .day

    private var client: APIClient {
        APIClient(baseURL: settings.baseURL, apiToken: settings.apiToken)
    }

    var body: some View {
        CinematicScreen(
            title: "Stats",
            subtitle: "Measure what matters.",
            icon: "waveform.path.ecg"
        ) {
            if let stats {
                periodPicker
                focusReport(period)
                metricStrip(period)

                Button {
                    Task { await summarizeToday() }
                } label: {
                    HStack {
                        Image(systemName: isSummarizing ? "hourglass" : "sparkles")
                        Text(isSummarizing ? "Summarizing..." : "Summarize today")
                        Spacer()
                        Image(systemName: "arrow.up.right")
                    }
                    .font(.system(size: 13, weight: .semibold, design: .monospaced))
                    .textCase(.uppercase)
                    .foregroundStyle(.arDeepBg)
                    .padding(.horizontal, 16)
                    .frame(maxWidth: .infinity, minHeight: 52)
                    .background(Color.arAccent, in: RoundedRectangle(cornerRadius: 4))
                }
                .buttonStyle(.plain)
                .disabled(isSummarizing)

                if !summaryText.isEmpty {
                    VStack(alignment: .leading, spacing: 10) {
                        Text("COACH REVIEW")
                            .font(.system(size: 11, weight: .bold, design: .monospaced))
                            .tracking(1.2)
                            .foregroundStyle(.arAccent)
                        Text(summaryText)
                            .font(.system(size: 18, design: .serif))
                            .foregroundStyle(.arTextPrimary)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    .padding(.leading, 16)
                    .overlay(alignment: .leading) {
                        Rectangle().fill(Color.arAccent).frame(width: 3)
                    }
                }

                Text(stats.note)
                    .font(.caption2)
                    .foregroundStyle(.arTextMuted)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.top, 4)
            } else {
                Text(isLoading ? "Loading stats..." : statusText)
                    .font(.system(size: 20, design: .serif))
                    .foregroundStyle(.arTextSecondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.vertical, 32)
            }
        }
        .refreshable {
            await loadStats()
        }
        .task {
            await loadStats()
        }
    }

    private var period: StatsPeriodResponse {
        guard let stats else {
            return StatsPeriodResponse(
                label: "day",
                workMinutes: 0,
                idleMinutes: 0,
                unproductiveDeskMinutes: 0,
                sessionsCompleted: 0,
                tasksDone: 0
            )
        }
        switch selectedPeriod {
        case .day: return stats.today
        case .week: return stats.week
        case .month: return stats.month
        }
    }

    private var periodPicker: some View {
        HStack(spacing: 22) {
            ForEach(StatsScope.allCases, id: \.self) { scope in
                Button {
                    withAnimation(reduceMotion ? .easeOut(duration: 0.14) : .spring(response: 0.22, dampingFraction: 0.86)) {
                        selectedPeriod = scope
                    }
                } label: {
                    Text(scope.title)
                        .font(.system(size: 13, weight: .semibold, design: .monospaced))
                        .textCase(.uppercase)
                        .foregroundStyle(selectedPeriod == scope ? .arTextPrimary : .arTextSecondary)
                        .padding(.vertical, 10)
                        .overlay(alignment: .bottom) {
                            Rectangle()
                                .fill(selectedPeriod == scope ? Color.arAccent : Color.clear)
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

    private func focusReport(_ period: StatsPeriodResponse) -> some View {
        VStack(alignment: .leading, spacing: 20) {
            Text(statusText.uppercased())
                .font(.system(size: 10, weight: .medium, design: .monospaced))
                .tracking(0.8)
                .foregroundStyle(.arTextMuted)
            HStack(alignment: .firstTextBaseline) {
                VStack(alignment: .leading, spacing: 6) {
                    Text("FOCUS TIME")
                        .font(.system(size: 11, weight: .bold, design: .monospaced))
                        .tracking(1.2)
                        .foregroundStyle(.arAccent)
                    Text(formatMinutes(period.workMinutes))
                        .font(.system(.largeTitle, design: .serif))
                        .foregroundStyle(.arTextPrimary)
                    Text("recorded focus")
                        .font(.subheadline)
                        .foregroundStyle(.arTextSecondary)
                }
            }

            timeComposition(period)
        }
        .padding(.vertical, 8)
    }

    private func metricStrip(_ period: StatsPeriodResponse) -> some View {
        HStack(spacing: 0) {
            reportMetric(value: period.sessionsCompleted, label: "Check-ins")
            Rectangle().fill(Color.arBorder).frame(width: 1, height: 54)
            reportMetric(value: period.tasksDone, label: "Completed")
        }
        .padding(.vertical, 14)
        .overlay(alignment: .top) { Rectangle().fill(Color.arBorder).frame(height: 1) }
        .overlay(alignment: .bottom) { Rectangle().fill(Color.arBorder).frame(height: 1) }
    }

    private func reportMetric(value: Int, label: String) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("\(value)")
                .font(.system(.title, design: .serif, weight: .semibold))
                .foregroundStyle(.arTextPrimary)
            Text(label.uppercased())
                .font(.system(size: 10, weight: .medium, design: .monospaced))
                .tracking(0.8)
                .foregroundStyle(.arTextSecondary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 12)
    }

    private func timeComposition(_ period: StatsPeriodResponse) -> some View {
        let rows = [
            StatsCompositionRow(label: "Work", minutes: period.workMinutes, tint: .arAccent),
            StatsCompositionRow(label: "Idle", minutes: period.idleMinutes, tint: .arTextSecondary),
            StatsCompositionRow(
                label: "Unproductive desk",
                minutes: period.unproductiveDeskMinutes,
                tint: .arAmber
            )
        ]
        let total = max(rows.map(\.minutes).reduce(0, +), 1)

        return VStack(spacing: 10) {
            ForEach(rows) { row in
                VStack(spacing: 5) {
                    HStack {
                        Text(row.label)
                            .font(.caption.weight(.semibold))
                            .foregroundStyle(.arTextSecondary)
                        Spacer()
                        Text(formatMinutes(row.minutes))
                            .font(.caption.weight(.bold))
                            .foregroundStyle(.arTextPrimary)
                    }

                    GeometryReader { proxy in
                        ZStack(alignment: .leading) {
                            Rectangle().fill(Color.arBorder)
                            Rectangle()
                                .fill(row.tint)
                                .frame(
                                    width: proxy.size.width * StatsPresentation.compositionRatio(
                                        minutes: row.minutes,
                                        total: total
                                    )
                                )
                        }
                    }
                    .frame(height: 6)
                }
            }
        }
    }

    @MainActor
    private func loadStats() async {
        isLoading = true
        defer { isLoading = false }

        do {
            stats = try await client.fetchStats()
            statusText = "Updated \(Date().formatted(date: .omitted, time: .shortened))"
        } catch {
            statusText = "Stats load failed: \(error.localizedDescription)"
        }
    }

    @MainActor
    private func summarizeToday() async {
        isSummarizing = true
        defer { isSummarizing = false }

        do {
            let response = try await client.chat(
                message: "Summarize what all was done today. Use the work log and task memory. Be concise, specific, and separate done work from unfinished work."
            )
            summaryText = response.reply
            if let nextState = response.runtimeState?.state, !nextState.isEmpty {
                coach.runtimeState = nextState
                coach.runtimeMetadata = response.runtimeState?.metadata
            }
        } catch {
            summaryText = error.localizedDescription
        }
    }

    private func formatMinutes(_ minutes: Int) -> String {
        if minutes < 60 {
            return "\(minutes)m"
        }
        let hours = minutes / 60
        let remainder = minutes % 60
        return remainder == 0 ? "\(hours)h" : "\(hours)h \(remainder)m"
    }
}

enum StatsPresentation {
    static func compositionRatio(minutes: Int, total: Int) -> CGFloat {
        guard total > 0 else { return 0 }
        return min(max(CGFloat(minutes) / CGFloat(total), 0), 1)
    }
}

private enum StatsScope: CaseIterable {
    case day
    case week
    case month

    var title: String {
        switch self {
        case .day: return "Day"
        case .week: return "Week"
        case .month: return "Month"
        }
    }
}

private struct StatsCompositionRow: Identifiable {
    let label: String
    let minutes: Int
    let tint: Color

    var id: String { label }
}

#Preview {
    StatsView()
        .environmentObject(SettingsStore())
        .environmentObject(CoachViewModel())
}
