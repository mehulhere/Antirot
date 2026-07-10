import SwiftUI

struct PlanView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var coach: CoachViewModel
    @State private var reviewText = ""
    @State private var isReviewing = false
    @State private var routineItems = RoutinePlanItem.defaultItems

    private var client: APIClient {
        APIClient(baseURL: settings.baseURL, apiToken: settings.apiToken, userId: settings.userId)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            CinematicKicker(title: "Plan", icon: "list.bullet", tint: .arAccent)

            // State actions
            if !visibleStateActions.isEmpty {
                VStack(spacing: 2) {
                    ForEach(visibleStateActions) { action in
                        actionRow(action)
                    }
                }
                .minimalCard(cornerRadius: 20, padding: 0)
            }

            if !routineItems.isEmpty {
                CinematicKicker(title: "Routine", icon: "repeat", tint: .arAmber)

                VStack(spacing: 0) {
                    ForEach(Array(routineItems.enumerated()), id: \.offset) { index, item in
                        HStack(alignment: .top, spacing: 12) {
                            Image(systemName: item.systemImage)
                                .font(.subheadline)
                                .foregroundStyle(.arTextMuted)
                                .frame(width: 24)
                            VStack(alignment: .leading, spacing: 3) {
                                Text(item.title)
                                    .font(.subheadline)
                                    .foregroundStyle(.arTextPrimary)
                                if !item.description.isEmpty {
                                    Text(item.description)
                                        .font(.caption)
                                        .foregroundStyle(.arTextMuted)
                                        .lineLimit(2)
                                }
                            }
                            Spacer()
                        }
                        .padding(.horizontal, 14)
                        .padding(.vertical, 10)

                        if index < routineItems.count - 1 {
                            SectionDivider()
                                .padding(.leading, 50)
                        }
                    }
                }
                .minimalCard(cornerRadius: 20, padding: 0)
            }

            // Review
            Button {
                Task { await requestDailyReview() }
            } label: {
                HStack(spacing: 10) {
                    Image(systemName: isReviewing ? "hourglass" : "sparkles")
                        .font(.subheadline)
                        .foregroundStyle(.arTextSecondary)
                    Text(isReviewing ? "Reviewing..." : "Request daily review")
                        .font(.subheadline)
                        .foregroundStyle(.arTextPrimary)
                    Spacer()
                    Image(systemName: "chevron.right")
                        .font(.caption2)
                        .foregroundStyle(.arTextMuted)
                }
                .padding(.horizontal, 14)
                .padding(.vertical, 12)
            }
            .buttonStyle(.plain)
            .disabled(isReviewing)
            .minimalCard(cornerRadius: 20, padding: 0)

            if !reviewText.isEmpty {
                Text(reviewText)
                    .font(.subheadline)
                    .foregroundStyle(.arTextSecondary)
                    .minimalCard(cornerRadius: 20, padding: 14)
            }
        }
        .task {
            await loadRoutine()
        }
        .onChange(of: settings.apiToken) {
            Task { await loadRoutine() }
        }
    }

    // MARK: - State Actions

    private var visibleStateActions: [CoachQuickAction] {
        CoachQuickAction.primary(for: coach.runtimeState)
    }

    private func actionRow(_ action: CoachQuickAction) -> some View {
        Button {
            Task { await coach.send(action.message, client: client) }
        } label: {
            HStack(spacing: 10) {
                Image(systemName: action.systemImage)
                    .font(.subheadline)
                    .foregroundStyle(.arTextSecondary)
                    .frame(width: 24)
                Text(action.title)
                    .font(.subheadline)
                    .foregroundStyle(.arTextPrimary)
                Spacer()
                Image(systemName: "chevron.right")
                    .font(.caption2)
                    .foregroundStyle(.arTextMuted)
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 12)
        }
        .buttonStyle(.plain)
    }

    // MARK: - Review

    private func requestDailyReview() async {
        isReviewing = true
        defer { isReviewing = false }

        do {
            let response = try await client.chat(
                message: "Review today: summarize what is done, what is pending, and the next non-negotiable move."
            )
            reviewText = response.reply
        } catch {
            reviewText = error.localizedDescription
        }
    }

    @MainActor
    private func loadRoutine() async {
        guard !settings.apiToken.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            routineItems = RoutinePlanItem.defaultItems
            return
        }

        do {
            let response = try await client.fetchMemory(key: "routine")
            let parsed = RoutinePlanItem.parseMarkdown(response.content)
            routineItems = parsed.isEmpty ? RoutinePlanItem.defaultItems : parsed
        } catch {
            routineItems = RoutinePlanItem.defaultItems
        }
    }
}

struct RoutinePlanItem: Equatable {
    var title: String
    var description: String
    var systemImage: String

    static let defaultItems: [RoutinePlanItem] = []

    static func parseMarkdown(_ content: String) -> [RoutinePlanItem] {
        var activeSection: String?
        var sawRoutineSections = false
        let items = content
            .split(separator: "\n")
            .compactMap { rawLine -> RoutinePlanItem? in
                let line = String(rawLine).trimmingCharacters(in: .whitespacesAndNewlines)
                if line.hasPrefix("## ") {
                    activeSection = String(line.dropFirst(3))
                    if activeSection == "Personalized Categories" {
                        sawRoutineSections = true
                    }
                    return nil
                }

                if sawRoutineSections {
                    guard visibleSection(activeSection) else {
                        return nil
                    }
                } else if hiddenLegacySection(activeSection) {
                    return nil
                }

                return parseLine(line)
            }
        return items
    }

    private static func visibleSection(_ section: String?) -> Bool {
        section == "Personalized Categories"
            || section == "Fixed Daily Allocations"
    }

    private static func hiddenLegacySection(_ section: String?) -> Bool {
        section == "Rules" || section == "Source"
    }

    private static func parseLine(_ line: String) -> RoutinePlanItem? {
        let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.hasPrefix("- ") else { return nil }
        guard !trimmed.contains("None yet") else { return nil }
        guard !trimmed.hasPrefix("- Last updated from:") else { return nil }
        let body = String(trimmed.dropFirst(2))
        let parts = body.split(separator: ":", maxSplits: 1).map(String.init)
        let title = parts[0].trimmingCharacters(in: .whitespacesAndNewlines)
        guard !title.isEmpty else { return nil }
        let description = parts.count > 1
            ? parts[1].trimmingCharacters(in: .whitespacesAndNewlines)
            : ""
        return RoutinePlanItem(
            title: title,
            description: description,
            systemImage: icon(for: title)
        )
    }

    private static func icon(for title: String) -> String {
        let lower = title.lowercased()
        if lower.contains("work") { return "timer" }
        if lower.contains("sleep") { return "bed.double.fill" }
        if lower.contains("vacation") || lower.contains("off") { return "beach.umbrella.fill" }
        if lower.contains("gym") || lower.contains("fitness") || lower.contains("workout") { return "figure.strengthtraining.traditional" }
        if lower.contains("relationship") || lower.contains("girlfriend") || lower.contains("family") { return "heart.fill" }
        if lower.contains("study") || lower.contains("class") || lower.contains("learn") { return "book.closed.fill" }
        if lower.contains("commute") || lower.contains("travel") { return "tram.fill" }
        if lower.contains("meal") || lower.contains("food") { return "fork.knife" }
        return "circle.grid.2x2.fill"
    }
}

#Preview {
    PlanView()
        .padding(.horizontal, 24)
        .background(Color.arBg)
        .environmentObject(SettingsStore())
        .environmentObject(CoachViewModel())
}
