import SwiftUI

struct PlanView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var coach: CoachViewModel
    @State private var reviewText = ""
    @State private var isReviewing = false

    private var client: APIClient {
        APIClient(baseURL: settings.baseURL, apiToken: settings.apiToken)
    }

    private let routineItems: [(String, String)] = [
        ("Work Blocks", "timer"),
        ("Gym", "figure.strengthtraining.traditional"),
        ("Relationship", "heart.fill"),
        ("Sleep", "bed.double.fill"),
        ("Vacation", "beach.umbrella.fill")
    ]

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            AntirotSectionHeader(title: "Plan", icon: "list.bullet")

            // State actions
            if !visibleStateActions.isEmpty {
                VStack(spacing: 2) {
                    ForEach(visibleStateActions) { action in
                        actionRow(action)
                    }
                }
                .minimalCard(cornerRadius: 12, padding: 0)
            }

            // Routine
            AntirotSectionHeader(title: "Routine")

            VStack(spacing: 0) {
                ForEach(Array(routineItems.enumerated()), id: \.offset) { index, item in
                    HStack(spacing: 12) {
                        Image(systemName: item.1)
                            .font(.subheadline)
                            .foregroundStyle(.arTextMuted)
                            .frame(width: 24)
                        Text(item.0)
                            .font(.subheadline)
                            .foregroundStyle(.arTextPrimary)
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
            .minimalCard(cornerRadius: 12, padding: 0)

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
            .minimalCard(cornerRadius: 12, padding: 0)

            if !reviewText.isEmpty {
                Text(reviewText)
                    .font(.subheadline)
                    .foregroundStyle(.arTextSecondary)
                    .minimalCard(cornerRadius: 12, padding: 14)
            }
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
}

#Preview {
    PlanView()
        .padding(.horizontal, 24)
        .background(Color.arBg)
        .environmentObject(SettingsStore())
        .environmentObject(CoachViewModel())
}
