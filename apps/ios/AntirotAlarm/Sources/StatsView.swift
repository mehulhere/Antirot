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
        ScrollView(.vertical, showsIndicators: true) {
            VStack(alignment: .leading, spacing: 18) {
                VStack(alignment: .leading, spacing: 6) {
                    Text("Stats")
                        .font(.largeTitle.weight(.bold))
                        .foregroundStyle(.arTextPrimary)
                    Text(statusText)
                        .font(.caption)
                        .foregroundStyle(.arTextSecondary)
                }
                .padding(.top, 86)

                if let stats {
                    todayGrid(stats.today)

                    periodSection(title: "Daily", period: stats.today)
                    periodSection(title: "Weekly", period: stats.week)
                    periodSection(title: "Monthly", period: stats.month)

                    Button {
                        Task { await summarizeToday() }
                    } label: {
                        HStack(spacing: 10) {
                            Image(systemName: isSummarizing ? "hourglass" : "sparkles")
                                .font(.subheadline)
                            Text(isSummarizing ? "Summarizing..." : "Summarize today")
                                .font(.subheadline.weight(.semibold))
                            Spacer()
                            Image(systemName: "chevron.right")
                                .font(.caption2)
                        }
                        .foregroundStyle(.arTextPrimary)
                        .padding(.horizontal, 14)
                        .padding(.vertical, 13)
                    }
                    .buttonStyle(.plain)
                    .disabled(isSummarizing)
                    .minimalCard(cornerRadius: 14, padding: 0)

                    if !summaryText.isEmpty {
                        Text(summaryText)
                            .font(.subheadline)
                            .foregroundStyle(.arTextSecondary)
                            .fixedSize(horizontal: false, vertical: true)
                            .minimalCard(cornerRadius: 14, padding: 14)
                    }

                    Text(stats.note)
                        .font(.caption2)
                        .foregroundStyle(.arTextMuted)
                        .fixedSize(horizontal: false, vertical: true)
                        .padding(.top, 4)
                } else {
                    Text("Stats unavailable.")
                        .font(.subheadline)
                        .foregroundStyle(.arTextMuted)
                        .minimalCard(cornerRadius: 14, padding: 14)
                }
            }
            .padding(.horizontal, 24)
            .padding(.bottom, 36)
        }
        .background(Color.arBg.ignoresSafeArea())
        .refreshable {
            await loadStats()
        }
        .task {
            await loadStats()
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
            statTile(title: "Work", value: formatMinutes(today.workMinutes), icon: "timer")
            statTile(title: "Idle", value: formatMinutes(today.idleMinutes), icon: "pause.circle")
            statTile(title: "Unproductive desk", value: formatMinutes(today.unproductiveDeskMinutes), icon: "exclamationmark.triangle")
            statTile(title: "Tasks done", value: "\(today.tasksDone)", icon: "checkmark.circle")
        }
    }

    private func statTile(title: String, value: String, icon: String) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Image(systemName: icon)
                    .font(.caption.weight(.bold))
                    .foregroundStyle(.arAccent)
                Spacer()
            }
            Text(value)
                .font(.title3.weight(.bold))
                .foregroundStyle(.arTextPrimary)
                .lineLimit(1)
                .minimumScaleFactor(0.75)
            Text(title)
                .font(.caption2.weight(.semibold))
                .foregroundStyle(.arTextMuted)
                .lineLimit(2)
                .fixedSize(horizontal: false, vertical: true)
        }
        .frame(maxWidth: .infinity, minHeight: 104, alignment: .topLeading)
        .minimalCard(cornerRadius: 14, padding: 14)
    }

    private func periodSection(title: String, period: StatsPeriodResponse) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            AntirotSectionHeader(title: title, icon: "chart.bar")
            VStack(spacing: 0) {
                statRow(label: "Work time", value: formatMinutes(period.workMinutes))
                SectionDivider()
                statRow(label: "Idle time", value: formatMinutes(period.idleMinutes))
                SectionDivider()
                statRow(label: "Unproductive desk time", value: formatMinutes(period.unproductiveDeskMinutes))
                SectionDivider()
                statRow(label: "Tasks done", value: "\(period.tasksDone)")
            }
            .minimalCard(cornerRadius: 14, padding: 0)
        }
    }

    private func statRow(label: String, value: String) -> some View {
        HStack {
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
