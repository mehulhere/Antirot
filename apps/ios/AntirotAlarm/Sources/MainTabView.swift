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
            case .home: return "waveform"
            case .plan: return "calendar.badge.clock"
            case .alarms: return "alarm.fill"
            case .settings: return "gearshape.fill"
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
                .fill(.ultraThinMaterial)
                .opacity(0.8)
                .overlay(
                    Rectangle()
                        .fill(Color.antirotBgSecondary.opacity(0.6))
                )
                .overlay(alignment: .top) {
                    Rectangle()
                        .fill(Color.antirotBorder)
                        .frame(height: 0.5)
                }
                .ignoresSafeArea(.container, edges: .bottom)
        )
    }

    private func tabButton(for tab: Tab) -> some View {
        Button {
            withAnimation(.easeInOut(duration: 0.2)) {
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

                // Active indicator
                RoundedRectangle(cornerRadius: 1.5)
                    .fill(selectedTab == tab ? Color.antirotAccentRed : .clear)
                    .frame(width: 20, height: 3)
            }
            .foregroundStyle(selectedTab == tab ? .antirotAccentRed : .antirotTextMuted)
            .frame(maxWidth: .infinity)
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
