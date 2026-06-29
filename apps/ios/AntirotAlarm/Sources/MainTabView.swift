import SwiftUI

struct MainTabView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @State private var selectedTab: Tab = .home

    enum Tab: String, CaseIterable {
        case home
        case plan
        case alarms
        case settings

        var icon: String {
            switch self {
            case .home: return "bolt.fill"
            case .plan: return "list.bullet.clipboard"
            case .alarms: return "bell.and.waves.left.and.right"
            case .settings: return "slider.horizontal.3"
            }
        }

        var label: String {
            switch self {
            case .home: return "Coach"
            case .plan: return "Plan"
            case .alarms: return "Alarms"
            case .settings: return "Settings"
            }
        }
    }

    var body: some View {
        ZStack(alignment: .bottom) {
            // Tab content
            Group {
                switch selectedTab {
                case .home:
                    HomeView()
                case .plan:
                    PlanView()
                case .alarms:
                    AlarmsView()
                case .settings:
                    SettingsView()
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .clipped()

            // Custom tab bar
            tabBar
        }
        .ignoresSafeArea(.keyboard)
    }

    private var tabBar: some View {
        HStack(spacing: 0) {
            ForEach(Tab.allCases, id: \.rawValue) { tab in
                tabButton(for: tab)
            }
        }
        .padding(.horizontal, 20)
        .padding(.top, 12)
        .padding(.bottom, 4)
        .background(
            Rectangle()
                .fill(Color.antirotBgElevated)
                .overlay(alignment: .top) {
                    Rectangle()
                        .fill(Color.antirotBorderStrong)
                        .frame(height: 0.5)
                }
                .ignoresSafeArea(.container, edges: .bottom)
        )
    }

    private func tabButton(for tab: Tab) -> some View {
        Button {
            withAnimation(.spring(duration: 0.35, bounce: 0.2)) {
                selectedTab = tab
            }
        } label: {
            VStack(spacing: 4) {
                Image(systemName: tab.icon)
                    .font(.system(size: 20, weight: selectedTab == tab ? .semibold : .regular))
                    .symbolEffect(.bounce, value: selectedTab == tab)

                Text(tab.label)
                    .font(.caption2)
                    .fontWeight(selectedTab == tab ? .semibold : .regular)
            }
            .foregroundStyle(selectedTab == tab ? .antirotAccent : .antirotTextMuted)
            .frame(maxWidth: .infinity)
            .padding(.vertical, 6)
            .background {
                if selectedTab == tab {
                    Capsule()
                        .fill(Color.antirotGlowPrimary)
                        .transition(.scale.combined(with: .opacity))
                }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

#Preview {
    MainTabView()
        .environmentObject(SettingsStore())
        .environmentObject(AlarmCenter())
        .environmentObject(CoachViewModel())
}
