import SwiftUI

enum AppBottomBarMetrics {
    static let horizontalPadding: CGFloat = 14
    static let bottomPadding: CGFloat = 12
    static let coachChatClearance: CGFloat = 104
    static let minimumHitTarget: CGFloat = 44
    static let usesFullScreenHitTestOverlay = false
}

enum AppChromeMetrics {
    static let showsCoachTopMenuShortcut = false
}

struct MainTabView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @EnvironmentObject private var coach: CoachViewModel
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

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
                .shadow(color: .black.opacity(0.44), radius: 26, y: 14)
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
        HStack(spacing: 4) {
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
        .padding(7)
        .smokedGlass(cornerRadius: 29, tint: .arSurface)
    }

    func select(_ screen: AppScreen) {
        withAnimation(reduceMotion ? .easeOut(duration: 0.14) : .spring(response: 0.28, dampingFraction: 0.82)) {
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
            VStack(spacing: 3) {
                Image(systemName: systemImage)
                    .font(.system(size: 17, weight: .semibold))
                Text(title)
                    .font(.caption2.weight(.bold))
                Capsule(style: .continuous)
                    .fill(isSelected ? Color.arAccent : Color.clear)
                    .frame(width: 16, height: 2)
            }
            .foregroundStyle(isSelected ? .white : .arTextSecondary)
            .frame(maxWidth: .infinity)
            .frame(minHeight: AppBottomBarMetrics.minimumHitTarget)
            .padding(.vertical, 7)
            .background(
                RoundedRectangle(cornerRadius: 21, style: .continuous)
                    .fill(isSelected ? Color.white.opacity(0.09) : Color.clear)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 21, style: .continuous)
                    .stroke(isSelected ? Color.arBorderActive : .clear, lineWidth: 0.6)
            )
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
