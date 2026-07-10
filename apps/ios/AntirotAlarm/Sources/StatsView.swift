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
        APIClient(baseURL: settings.baseURL, apiToken: settings.apiToken, userId: settings.userId)
    }

    var body: some View {
        CinematicScreen(
            title: "Stats",
            subtitle: "Measure what matters.",
            icon: "waveform.path.ecg"
        ) {
            if let stats {
                periodPicker
                focusCard(period)
                metricGrid(period)
                settingsRows

                CinematicGlassCard(padding: 0, accent: .arAccent) {
                    CinematicActionRow(
                        title: isSummarizing ? "Summarizing..." : "Summarize today",
                        subtitle: "Ask the coach what actually moved.",
                        icon: isSummarizing ? "hourglass" : "sparkles",
                        tint: .arAccent
                    ) {
                        Task { await summarizeToday() }
                    }
                    .disabled(isSummarizing)
                    .padding(14)
                }

                if !summaryText.isEmpty {
                    CinematicGlassCard(padding: 16, accent: .arCyan) {
                        VStack(alignment: .leading, spacing: 10) {
                            CinematicKicker(title: "Coach Review", icon: "quote.bubble", tint: .arCyan)
                            Text(summaryText)
                                .font(.subheadline)
                                .foregroundStyle(.arTextSecondary)
                                .fixedSize(horizontal: false, vertical: true)
                        }
                    }
                }

                Text(stats.note)
                    .font(.caption2)
                    .foregroundStyle(.arTextMuted)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.top, 4)
            } else {
                CinematicGlassCard(padding: 16, accent: .arWarning) {
                    Text("Stats unavailable.")
                        .font(.subheadline)
                        .foregroundStyle(.arTextSecondary)
                }
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
        HStack(spacing: 4) {
            ForEach(StatsScope.allCases, id: \.self) { scope in
                Button {
                    withAnimation(reduceMotion ? .easeOut(duration: 0.14) : .spring(response: 0.22, dampingFraction: 0.86)) {
                        selectedPeriod = scope
                    }
                } label: {
                    Text(scope.title)
                        .font(.subheadline.weight(.bold))
                        .foregroundStyle(selectedPeriod == scope ? .arTextPrimary : .arTextSecondary)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 12)
                        .background(
                            Capsule(style: .continuous)
                                .fill(selectedPeriod == scope ? Color.white.opacity(0.10) : Color.clear)
                        )
                        .overlay(alignment: .bottom) {
                            Capsule(style: .continuous)
                                .fill(selectedPeriod == scope ? Color.arAccent : Color.clear)
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

    private func focusCard(_ today: StatsPeriodResponse) -> some View {
        CinematicGlassCard(padding: 18, accent: .arAccent) {
            VStack(alignment: .leading, spacing: 18) {
                Text(statusText)
                    .font(.caption2.weight(.semibold))
                    .foregroundStyle(.arTextMuted)
                HStack(alignment: .center, spacing: 16) {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("FOCUS TIME")
                            .font(.caption.weight(.bold))
                            .tracking(1.0)
                            .foregroundStyle(.arTextSecondary)
                        Text(formatMinutes(today.workMinutes))
                            .font(.system(size: 32, weight: .regular, design: .rounded))
                            .foregroundStyle(.arTextPrimary)
                        Text("/ 4h 0m goal")
                            .font(.subheadline.weight(.medium))
                            .foregroundStyle(.arTextSecondary)
                    }

                    Spacer(minLength: 10)

                    ZStack {
                        Circle()
                            .stroke(Color.white.opacity(0.08), lineWidth: 8)
                        Circle()
                            .trim(from: 0, to: goalRatio(today))
                            .stroke(Color.arAccent, style: StrokeStyle(lineWidth: 8, lineCap: .round))
                            .rotationEffect(.degrees(-90))
                        Text("\(Int(goalRatio(today) * 100))%")
                            .font(.title3.weight(.bold))
                            .foregroundStyle(.arTextPrimary)
                    }
                    .frame(width: 88, height: 88)
                }

                timeComposition(today)
            }
        }
    }

    private func metricGrid(_ period: StatsPeriodResponse) -> some View {
        ViewThatFits(in: .horizontal) {
            HStack(alignment: .top, spacing: 12) {
                metricTiles(period)
            }

            VStack(spacing: 12) {
                metricTiles(period)
            }
        }
    }

    @ViewBuilder
    private func metricTiles(_ period: StatsPeriodResponse) -> some View {
        CinematicMetricTile(
            title: "Check-ins",
            value: "\(period.sessionsCompleted)",
            icon: "checkmark",
            tint: .arSuccess
        )
        .frame(minWidth: 148)
        CinematicMetricTile(
            title: "Completed today",
            value: "\(period.tasksDone)",
            icon: "checkmark.circle.fill",
            tint: .arAccent
        )
        .frame(minWidth: 148)
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
                            Capsule().fill(Color.white.opacity(0.07))
                            Capsule()
                                .fill(row.tint)
                                .frame(width: proxy.size.width * CGFloat(row.minutes) / CGFloat(total))
                        }
                    }
                    .frame(height: 6)
                }
            }
        }
    }

    private var settingsRows: some View {
        CinematicGlassCard(padding: 0, accent: .arAccent) {
            VStack(spacing: 0) {
                statsLinkRow(title: "Settings", icon: "gearshape", tint: .arTextSecondary)
                SectionDivider()
                statsLinkRow(title: "Developer", icon: "chevron.left.forwardslash.chevron.right", tint: .arTextSecondary)
            }
        }
    }

    private func statsLinkRow(title: String, icon: String, tint: Color) -> some View {
        HStack(spacing: 12) {
            Image(systemName: icon)
                .font(.headline.weight(.semibold))
                .foregroundStyle(tint)
                .frame(width: 28)
            Text(title)
                .font(.headline.weight(.semibold))
                .foregroundStyle(.arTextPrimary)
            Spacer()
            Image(systemName: "chevron.right")
                .font(.caption.weight(.bold))
                .foregroundStyle(.arTextMuted)
        }
        .padding(.horizontal, 18)
        .padding(.vertical, 16)
    }

    private func goalRatio(_ period: StatsPeriodResponse) -> CGFloat {
        min(max(CGFloat(period.workMinutes) / 240.0, 0.05), 1.0)
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
