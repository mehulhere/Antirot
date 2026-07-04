import SwiftUI

enum AppBottomBarMetrics {
    static let horizontalPadding: CGFloat = 20
    static let bottomPadding: CGFloat = 10
    static let coachChatClearance: CGFloat = 82
}

struct MainTabView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @EnvironmentObject private var coach: CoachViewModel

    @State private var selectedScreen: AppScreen = .coach
    @State private var showControlSheet = false

    var body: some View {
        ZStack(alignment: .topTrailing) {
            Group {
                switch selectedScreen {
                case .coach:
                    HomeView()
                case .tasks:
                    TaskBoardView()
                case .stats:
                    StatsView()
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)

            appBar
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)
                .padding(.horizontal, AppBottomBarMetrics.horizontalPadding)
                .padding(.bottom, AppBottomBarMetrics.bottomPadding)
                .shadow(color: .black.opacity(0.40), radius: 20, y: 10)

            // Hidden menu — small, quiet glass icon, top-right. Keeps stats,
            // plan, alarms, and settings out of the primary coach experience.
            Button {
                showControlSheet = true
            } label: {
                Image(systemName: "line.3.horizontal")
                    .font(.system(size: 15, weight: .semibold))
                    .foregroundStyle(.arTextPrimary)
                    .frame(width: 44, height: 44)
                    .background(Circle().fill(.ultraThinMaterial))
                    .background(Circle().fill(Color.white.opacity(0.035)))
                    .overlay(Circle().stroke(Color.white.opacity(0.09), lineWidth: 0.6))
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Open controls")
            .padding(.trailing, 16)
            .padding(.top, 6)
        }
        .sheet(isPresented: $showControlSheet) {
            ControlSheetView()
                .environmentObject(settings)
                .environmentObject(alarmCenter)
                .environmentObject(coach)
        }
    }
}

private enum AppScreen {
    case coach
    case tasks
    case stats
}

private extension MainTabView {
    var appBar: some View {
        HStack(spacing: 6) {
            AppBarButton(
                title: "Coach",
                systemImage: "bolt.fill",
                isSelected: selectedScreen == .coach
            ) {
                select(.coach)
            }

            AppBarButton(
                title: "Tasks",
                systemImage: "checklist",
                isSelected: selectedScreen == .tasks
            ) {
                select(.tasks)
            }

            AppBarButton(
                title: "Stats",
                systemImage: "chart.bar.fill",
                isSelected: selectedScreen == .stats
            ) {
                select(.stats)
            }
        }
        .padding(6)
        .background(.ultraThinMaterial, in: Capsule(style: .continuous))
        .background(Color.black.opacity(0.20), in: Capsule(style: .continuous))
        .overlay(
            Capsule(style: .continuous)
                .stroke(Color.white.opacity(0.12), lineWidth: 0.6)
        )
    }

    func select(_ screen: AppScreen) {
        withAnimation(.spring(response: 0.24, dampingFraction: 0.86)) {
            selectedScreen = screen
        }
    }
}

private struct AppBarButton: View {
    let title: String
    let systemImage: String
    let isSelected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 6) {
                Image(systemName: systemImage)
                    .font(.caption.weight(.bold))
                Text(title)
                    .font(.caption.weight(.bold))
            }
            .foregroundStyle(isSelected ? .white : .arTextSecondary)
            .padding(.horizontal, 14)
            .padding(.vertical, 9)
            .background(
                Capsule(style: .continuous)
                    .fill(isSelected ? Color.arAccent : Color.clear)
            )
            .shadow(color: isSelected ? Color.arAccent.opacity(0.22) : .clear, radius: 12, y: 5)
        }
        .buttonStyle(.plain)
    }
}

// MARK: - Control Sheet

private struct ControlSheetView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @EnvironmentObject private var coach: CoachViewModel

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 16) {
                    CinematicHeader(
                        title: "Command",
                        subtitle: "Plan, alarms, permissions, diagnostics.",
                        icon: "slider.horizontal.3"
                    )

                    PlanView()

                    AlarmsView()

                    SettingsView()
                        .environmentObject(coach)
                }
                .padding(.horizontal, 20)
                .padding(.top, 18)
                .padding(.bottom, 32)
            }
            .background(CinematicBackdrop())
            .navigationBarTitleDisplayMode(.inline)
        }
        .presentationDetents([.medium, .large])
        .presentationDragIndicator(.visible)
    }
}

#Preview {
    MainTabView()
        .environmentObject(SettingsStore())
        .environmentObject(AlarmCenter())
        .environmentObject(CoachViewModel())
}
