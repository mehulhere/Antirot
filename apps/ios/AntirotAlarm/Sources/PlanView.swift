import SwiftUI

struct PlanView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var coach: CoachViewModel
    @State private var reviewText = ""
    @State private var isReviewing = false

    private var client: APIClient {
        APIClient(baseURL: settings.baseURL, apiToken: settings.apiToken)
    }

    private let routineItems: [(String, String, String)] = [
        ("Work Blocks", "Deep work sessions, completion, and recovery checks.", "timer"),
        ("Gym", "Fixed daily body maintenance, not backlog work.", "figure.strengthtraining.traditional"),
        ("Relationship", "Protected time for girlfriend and real human presence.", "heart.fill"),
        ("Sleep", "Good night, wake logging, and nightly distillation.", "bed.double.fill"),
        ("Vacation", "No alarm pressure when vacation mode is explicit.", "beach.umbrella.fill")
    ]

    var body: some View {
        ZStack {
            Color.antirotBg.ignoresSafeArea()

            ScrollView(.vertical, showsIndicators: false) {
                VStack(alignment: .leading, spacing: 22) {
                    Text("Plan")
                        .font(.title.bold())
                        .foregroundStyle(.antirotTextPrimary)

                    stateActions
                    routineSection
                    reviewSection
                }
                .padding(.horizontal, 20)
                .padding(.top, 16)
                .padding(.bottom, 92)
            }
        }
    }

    private var stateActions: some View {
        let actions = visibleStateActions

        return Group {
            if !actions.isEmpty {
                VStack(alignment: .leading, spacing: 12) {
                    AntirotSectionHeader(title: "State Actions", icon: "switch.2")

                    LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], spacing: 10) {
                        ForEach(actions) { action in
                            planButton(action.title, action.systemImage, action.message)
                        }
                    }
                }
            }
        }
    }

    private var visibleStateActions: [CoachQuickAction] {
        switch coach.runtimeState.lowercased() {
        case "idle":
            return [
                CoachQuickAction(
                    id: "plan_start_work",
                    title: "Start Work",
                    systemImage: "play.fill",
                    message: "I am ready to work. Start the task we just picked."
                )
            ]
        case "working":
            return [
                CoachQuickAction(
                    id: "plan_done",
                    title: "Done",
                    systemImage: "checkmark",
                    message: "Done. I finished the current task. Ask me how much of it was actually productive before closing it."
                )
            ]
        default:
            return []
        }
    }

    private var routineSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            AntirotSectionHeader(title: "Routine", icon: "calendar")

            ForEach(Array(routineItems.enumerated()), id: \.offset) { _, item in
                HStack(spacing: 12) {
                    Image(systemName: item.2)
                        .font(.headline)
                        .foregroundStyle(.antirotGold)
                        .frame(width: 28)

                    VStack(alignment: .leading, spacing: 3) {
                        Text(item.0)
                            .font(.headline)
                            .foregroundStyle(.antirotTextPrimary)
                        Text(item.1)
                            .font(.caption)
                            .foregroundStyle(.antirotTextMuted)
                    }

                    Spacer()
                }
                .layeredCard(cornerRadius: 16, padding: 14)
            }
        }
    }

    private var reviewSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            AntirotSectionHeader(title: "Review", icon: "doc.text.magnifyingglass")

            Button {
                Task { await requestDailyReview() }
            } label: {
                HStack {
                    Image(systemName: isReviewing ? "hourglass" : "sparkles")
                    Text(isReviewing ? "Reviewing today" : "Ask coach for today's review")
                    Spacer()
                    Image(systemName: "chevron.right")
                        .font(.caption.weight(.bold))
                }
                .foregroundStyle(.antirotTextPrimary)
            }
            .buttonStyle(.plain)
            .disabled(isReviewing)
            .layeredCard(cornerRadius: 16, padding: 16)

            if !reviewText.isEmpty {
                Text(reviewText)
                    .font(.subheadline)
                    .foregroundStyle(.antirotTextSecondary)
                    .layeredCard(cornerRadius: 16, padding: 16)
            }
        }
    }

    private func planButton(_ title: String, _ icon: String, _ message: String) -> some View {
        Button {
            Task { await coach.send(message, client: client) }
        } label: {
            VStack(spacing: 9) {
                Image(systemName: icon)
                    .font(.title3.weight(.semibold))
                Text(title)
                    .font(.caption.weight(.semibold))
                    .lineLimit(1)
            }
            .foregroundStyle(.antirotTextPrimary)
            .frame(maxWidth: .infinity, minHeight: 72)
        }
        .buttonStyle(.plain)
        .layeredCard(cornerRadius: 16, padding: 12)
    }

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
        .environmentObject(SettingsStore())
        .environmentObject(CoachViewModel())
}
