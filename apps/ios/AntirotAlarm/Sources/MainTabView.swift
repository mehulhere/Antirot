import SwiftUI

enum AppBottomBarMetrics {
    static let horizontalPadding: CGFloat = 0
    static let bottomPadding: CGFloat = 0
    static let coachChatClearance: CGFloat = 76
    static let barHeight: CGFloat = 64
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

    @StateObject private var navigation = AppNavigationModel()

    var body: some View {
        Group {
            switch navigation.selectedScreen {
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
        .environmentObject(navigation)
        .safeAreaInset(edge: .bottom, spacing: 0) {
            if !navigation.isAppBarHidden {
                appBar
            }
        }
        .background(Color.arBg)
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

final class AppNavigationModel: ObservableObject {
    @Published var selectedScreen: AppScreen = .coach
    @Published var isAppBarHidden = false
}

private extension MainTabView {
    var appBar: some View {
        HStack(spacing: 0) {
            ForEach(AppScreen.allCases, id: \.self) { screen in
                AppBarButton(
                    title: screen.title,
                    systemImage: screen.systemImage,
                    isSelected: navigation.selectedScreen == screen
                ) {
                    select(screen)
                }
            }
        }
        .frame(maxWidth: .infinity)
        .frame(height: AppBottomBarMetrics.barHeight)
        .background(Color.arBg)
        .overlay(alignment: .top) {
            Rectangle()
                .fill(Color.arBorder)
                .frame(height: 1)
        }
    }

    func select(_ screen: AppScreen) {
        withAnimation(reduceMotion ? .easeOut(duration: 0.14) : .spring(response: 0.28, dampingFraction: 0.82)) {
            navigation.selectedScreen = screen
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
                    .font(.system(size: 16, weight: .medium))
                Text(title)
                    .font(.caption2.weight(.semibold))
                Rectangle()
                    .fill(isSelected ? Color.arAccent : Color.clear)
                    .frame(width: 20, height: 2)
            }
            .foregroundStyle(isSelected ? .arTextPrimary : .arTextMuted)
            .frame(maxWidth: .infinity)
            .frame(minHeight: AppBottomBarMetrics.minimumHitTarget)
            .contentShape(Rectangle())
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
