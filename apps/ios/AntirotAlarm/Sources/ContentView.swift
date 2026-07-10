import SwiftUI

struct ContentView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        Group {
            if settings.registered {
                MainTabView()
            } else {
                LoginView()
            }
        }
        .animation(reduceMotion ? nil : .easeInOut(duration: 0.35), value: settings.registered)
    }
}

#Preview {
    ContentView()
        .environmentObject(SettingsStore())
        .environmentObject(AlarmCenter())
        .environmentObject(CoachViewModel())
}
