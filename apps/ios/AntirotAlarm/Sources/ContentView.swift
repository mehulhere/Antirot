import SwiftUI

struct ContentView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter

    var body: some View {
        Group {
            if settings.registered {
                MainTabView()
            } else {
                LoginView()
            }
        }
        .animation(.easeInOut(duration: 0.4), value: settings.registered)
    }
}

#Preview {
    ContentView()
        .environmentObject(SettingsStore())
        .environmentObject(AlarmCenter())
}
