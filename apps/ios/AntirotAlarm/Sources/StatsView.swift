import SwiftUI

struct StatsView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var coach: CoachViewModel

    @State private var stats: StatsResponse?
    @State private var statusText = "Loading stats..."
    @State private var summaryText = ""
    @State private var isLoading = false
    @State private var isSummarizing = false

    private var client: APIClient {
        APIClient(baseURL: settings.baseURL, apiToken: settings.apiToken, userId: settings.userId)
    }

    var body: some View {
        CinematicScreen(
            title: "Stats",
            subtitle: statusText,
            icon: "chart.bar.fill"
        ) {
            if let stats {
                todayHero(stats.today)
                todayGrid(stats.today)
                periodSection(title: "Daily", period: stats.today, tint: .arAccent)
                periodSection(title: "Weekly", period: stats.week, tint: .arCyan)
                periodSection(title: "Monthly", period: stats.month, tint: .arAmber)

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

    private func todayHero(_ today: StatsPeriodResponse) -> some View {
        CinematicGlassCard(padding: 18, accent: .arAccent) {
            HStack(alignment: .center, spacing: 16) {
                VStack(alignment: .leading, spacing: 8) {
                    CinematicKicker(title: "Today", icon: "target", tint: .arAccent)
                    Text(formatMinutes(today.workMinutes))
                        .font(.system(size: 42, weight: .bold, design: .rounded))
                        .foregroundStyle(.arTextPrimary)
                        .lineLimit(1)
                        .minimumScaleFactor(0.65)
                    Text("focused work")
                        .font(.subheadline.weight(.semibold))
                        .foregroundStyle(.arTextSecondary)
                }

                Spacer(minLength: 10)

                ZStack {
                    Circle()
                        .stroke(Color.white.opacity(0.08), lineWidth: 10)
                    Circle()
                        .trim(from: 0, to: focusRatio(today))
                        .stroke(Color.arAccent, style: StrokeStyle(lineWidth: 10, lineCap: .round))
                        .rotationEffect(.degrees(-90))
                    Text("\(today.tasksDone)")
                        .font(.system(size: 26, weight: .bold, design: .rounded))
                        .foregroundStyle(.arTextPrimary)
                }
                .frame(width: 88, height: 88)
                .accessibilityLabel("Tasks done today \(today.tasksDone)")
            }
        }
    }

    private func todayGrid(_ today: StatsPeriodResponse) -> some View {
        LazyVGrid(
            columns: [
                GridItem(.flexible(), spacing: 10),
                GridItem(.flexible(), spacing: 10)
            ],
            spacing: 10
        ) {
            CinematicMetricTile(title: "Work", value: formatMinutes(today.workMinutes), icon: "timer", tint: .arAccent)
            CinematicMetricTile(title: "Idle", value: formatMinutes(today.idleMinutes), icon: "pause.circle", tint: .arCyan)
            CinematicMetricTile(title: "Desk drift", value: formatMinutes(today.unproductiveDeskMinutes), icon: "exclamationmark.triangle", tint: .arWarning)
            CinematicMetricTile(title: "Done", value: "\(today.tasksDone)", icon: "checkmark.circle", tint: .arSuccess)
        }
    }

    private func periodSection(title: String, period: StatsPeriodResponse, tint: Color) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            CinematicKicker(title: title, icon: "chart.bar", tint: tint)
            VStack(spacing: 0) {
                statRow(label: "Work time", value: formatMinutes(period.workMinutes), tint: tint)
                SectionDivider()
                statRow(label: "Idle time", value: formatMinutes(period.idleMinutes), tint: .arCyan)
                SectionDivider()
                statRow(label: "Desk drift", value: formatMinutes(period.unproductiveDeskMinutes), tint: .arWarning)
                SectionDivider()
                statRow(label: "Tasks done", value: "\(period.tasksDone)", tint: .arSuccess)
            }
            .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: AntirotCinematicMetrics.cardRadius, style: .continuous))
            .background(Color.white.opacity(0.025), in: RoundedRectangle(cornerRadius: AntirotCinematicMetrics.cardRadius, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: AntirotCinematicMetrics.cardRadius, style: .continuous)
                    .stroke(Color.white.opacity(0.08), lineWidth: 0.6)
            )
        }
    }

    private func statRow(label: String, value: String, tint: Color) -> some View {
        HStack {
            Circle()
                .fill(tint)
                .frame(width: 7, height: 7)
            Text(label)
                .font(.subheadline)
                .foregroundStyle(.arTextSecondary)
            Spacer()
            Text(value)
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(.arTextPrimary)
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 12)
    }

    private func focusRatio(_ today: StatsPeriodResponse) -> CGFloat {
        let total = max(today.workMinutes + today.idleMinutes + today.unproductiveDeskMinutes, 1)
        return min(max(CGFloat(today.workMinutes) / CGFloat(total), 0.05), 1.0)
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

#Preview {
    StatsView()
        .environmentObject(SettingsStore())
        .environmentObject(CoachViewModel())
}
