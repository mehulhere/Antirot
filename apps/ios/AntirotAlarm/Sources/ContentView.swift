import SwiftUI

struct ContentView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @State private var screenTimeMessage = "Not requested"

    var body: some View {
        NavigationStack {
            Form {
                Section("VPS") {
                    TextField("https://your-vps.example.com", text: $settings.serverURL)
                        .textContentType(.URL)
                        .keyboardType(.URL)
                        .textInputAutocapitalization(.never)
                    SecureField("API token", text: $settings.apiToken)
                    LabeledContent("Device ID", value: settings.deviceId)
                    LabeledContent("Status", value: settings.statusMessage)
                    Button("Register device") {
                        Task { await alarmCenter.registerDevice() }
                    }
                }

                Section("Permissions") {
                    LabeledContent("Notifications", value: String(describing: alarmCenter.notificationStatus))
                    Button("Request notification permission") {
                        Task { await alarmCenter.requestNotificationPermission() }
                    }
                    LabeledContent("Screen Time", value: screenTimeMessage)
                    Button("Request Screen Time permission") {
                        Task {
                            screenTimeMessage = await ScreenTimeCenter.requestAuthorization()
                        }
                    }
                }

                Section("Alarm Test") {
                    Button("Schedule normal test alarm") {
                        Task { await alarmCenter.scheduleTestAlarm(severity: .normal) }
                    }
                    Button("Schedule loud test alarm") {
                        Task { await alarmCenter.scheduleTestAlarm(severity: .loud) }
                    }
                    Button("Poll pending VPS alarms") {
                        Task { await alarmCenter.pollPendingAlarms() }
                    }
                    Text(alarmCenter.lastMessage)
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }

                Section("Scheduled") {
                    if alarmCenter.scheduledAlarms.isEmpty {
                        Text("No local alarms scheduled")
                            .foregroundStyle(.secondary)
                    } else {
                        ForEach(alarmCenter.scheduledAlarms) { alarm in
                            VStack(alignment: .leading, spacing: 4) {
                                Text(alarm.title)
                                    .font(.headline)
                                Text(alarm.message)
                                    .font(.subheadline)
                                Text(alarm.fireAt.formatted(date: .omitted, time: .shortened))
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                        }
                    }
                }
            }
            .navigationTitle("Antirot")
        }
    }
}

#Preview {
    ContentView()
        .environmentObject(SettingsStore())
        .environmentObject(AlarmCenter())
}
