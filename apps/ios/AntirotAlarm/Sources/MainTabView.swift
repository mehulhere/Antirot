import SwiftUI

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
                .padding(.horizontal, 24)
                .padding(.bottom, selectedScreen == .coach ? 132 : 10)

            // Hidden menu — small, quiet glass icon, top-right. Keeps stats,
            // plan, alarms, and settings out of the primary coach experience.
            Button {
                showControlSheet = true
            } label: {
                Image(systemName: "line.3.horizontal")
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(.arTextSecondary)
                    .frame(width: 38, height: 38)
                    .glassCapsule()
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
        .padding(5)
        .background(.ultraThinMaterial, in: Capsule(style: .continuous))
        .overlay(
            Capsule(style: .continuous)
                .stroke(Color.white.opacity(0.08), lineWidth: 0.5)
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
            .padding(.horizontal, 13)
            .padding(.vertical, 8)
            .background(
                Capsule(style: .continuous)
                    .fill(isSelected ? Color.arAccent : Color.clear)
            )
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
                VStack(spacing: 0) {
                    PlanView()

                    SectionDivider()
                        .padding(.vertical, 20)

                    AlarmsView()

                    SectionDivider()
                        .padding(.vertical, 20)

                    SettingsView()
                        .environmentObject(coach)
                }
                .padding(.horizontal, 24)
                .padding(.top, 16)
                .padding(.bottom, 32)
            }
            .background(Color.arBg)
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
