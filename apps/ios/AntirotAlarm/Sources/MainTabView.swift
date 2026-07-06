import SwiftUI

enum AppBottomBarMetrics {
    static let horizontalPadding: CGFloat = 12
    static let bottomPadding: CGFloat = 10
    static let coachChatClearance: CGFloat = 92
    static let usesFullScreenHitTestOverlay = false
}

enum AppChromeMetrics {
    static let showsCoachTopMenuShortcut = false
}

struct MainTabView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @EnvironmentObject private var coach: CoachViewModel

    @State private var selectedScreen: AppScreen = .coach

    var body: some View {
        ZStack(alignment: .bottom) {
            Group {
                switch selectedScreen {
                case .coach:
                    HomeView()
                case .tasks:
                    TaskBoardView()
                case .stats:
                    StatsView()
                case .settings:
                    SettingsScreen()
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)

            appBar
                .frame(maxWidth: .infinity)
                .padding(.horizontal, AppBottomBarMetrics.horizontalPadding)
                .padding(.bottom, AppBottomBarMetrics.bottomPadding)
                .shadow(color: .black.opacity(0.40), radius: 20, y: 10)
        }
    }
}

enum AppScreen: CaseIterable {
    case coach
    case tasks
    case stats
    case settings

    var title: String {
        switch self {
        case .coach: return "Coach"
        case .tasks: return "Tasks"
        case .stats: return "Stats"
        case .settings: return "Settings"
        }
    }

    var systemImage: String {
        switch self {
        case .coach: return "bolt.fill"
        case .tasks: return "list.bullet"
        case .stats: return "chart.bar.fill"
        case .settings: return "gearshape"
        }
    }
}

private extension MainTabView {
    var appBar: some View {
        HStack(spacing: 2) {
            ForEach(AppScreen.allCases, id: \.self) { screen in
                AppBarButton(
                    title: screen.title,
                    systemImage: screen.systemImage,
                    isSelected: selectedScreen == screen
                ) {
                    select(screen)
                }
            }
        }
        .frame(maxWidth: .infinity)
        .padding(6)
        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 24, style: .continuous))
        .background(Color.black.opacity(0.34), in: RoundedRectangle(cornerRadius: 24, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 24, style: .continuous)
                .stroke(Color.white.opacity(0.08), lineWidth: 0.6)
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
            VStack(spacing: 4) {
                Image(systemName: systemImage)
                    .font(.system(size: 18, weight: .bold))
                Text(title)
                    .font(.caption2.weight(.bold))
            }
            .foregroundStyle(isSelected ? .white : .arTextSecondary)
            .frame(maxWidth: .infinity)
            .padding(.vertical, 10)
            .background(
                RoundedRectangle(cornerRadius: 18, style: .continuous)
                    .fill(isSelected ? Color.arAccent.opacity(0.20) : Color.clear)
            )
            .shadow(color: isSelected ? Color.arAccent.opacity(0.22) : .clear, radius: 12, y: 5)
        }
        .buttonStyle(.plain)
    }
}

private struct SettingsScreen: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @EnvironmentObject private var coach: CoachViewModel

    var body: some View {
        NavigationStack {
            CinematicScreen(
                title: "Settings",
                subtitle: "Account, alarms, developer tools.",
                icon: "gearshape"
            ) {
                VStack(spacing: 14) {
                    SettingsView()
                        .environmentObject(coach)
                }
            }
        }
    }
}

#Preview {
    MainTabView()
        .environmentObject(SettingsStore())
        .environmentObject(AlarmCenter())
        .environmentObject(CoachViewModel())
}
