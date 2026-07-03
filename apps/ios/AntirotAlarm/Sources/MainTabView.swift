import SwiftUI

struct MainTabView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter

    @State private var showControlSheet = false

    var body: some View {
        ZStack(alignment: .topTrailing) {
            HomeView()
                .frame(maxWidth: .infinity, maxHeight: .infinity)

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
        }
    }
}

// MARK: - Control Sheet

private struct ControlSheetView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter

    var body: some View {
        ScrollView {
            VStack(spacing: 0) {
                PlanView()

                SectionDivider()
                    .padding(.vertical, 20)

                AlarmsView()

                SectionDivider()
                    .padding(.vertical, 20)

                SettingsView()
            }
            .padding(.horizontal, 24)
            .padding(.top, 16)
            .padding(.bottom, 32)
        }
        .background(Color.arBg)
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
