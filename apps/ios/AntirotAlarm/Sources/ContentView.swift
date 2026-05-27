import SwiftUI
import UniformTypeIdentifiers

struct ContentView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @State private var screenTimeMessage = "Not requested"
    @State private var isImportingSound = false

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
                    LabeledContent("AlarmKit", value: alarmCenter.alarmKitStatus)
                    Button("Request real alarm permission") {
                        Task { await alarmCenter.requestAlarmKitPermission() }
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

                Section("Alarm Sound") {
                    LabeledContent("Selected", value: settings.alarmSoundName.isEmpty ? "System default" : settings.alarmSoundName)
                    Button("Choose sound file") {
                        isImportingSound = true
                    }
                    Button("Use system default") {
                        settings.alarmSoundName = ""
                        alarmCenter.lastMessage = "Alarm sound reset to system default"
                    }
                    Text("Use a 30-second-or-shorter audio file. Antirot copies it into the iOS Library/Sounds folder and uses it for AlarmKit when available.")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }

                Section("Widget") {
                    Button("Show current task in widget") {
                        SharedTaskStore.write(CurrentTaskSnapshot(
                            title: "Start one real work block",
                            subtitle: "Enough setup. Put one task on the board.",
                            mode: "working",
                            dueAt: Date().addingTimeInterval(45 * 60)
                        ))
                    }
                    Text("Add the Antirot Current Task widget from the iOS Home Screen after installing the app.")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }

                Section("Scheduled") {
                    if alarmCenter.scheduledAlarms.isEmpty {
                        Text("No alarms scheduled")
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
            .fileImporter(isPresented: $isImportingSound, allowedContentTypes: [.audio]) { result in
                switch result {
                case let .success(url):
                    do {
                        settings.alarmSoundName = try SoundLibrary.importAlarmSound(from: url)
                        alarmCenter.lastMessage = "Selected alarm sound: \(settings.alarmSoundName)"
                    } catch {
                        alarmCenter.lastMessage = "Sound import failed: \(error.localizedDescription)"
                    }
                case let .failure(error):
                    alarmCenter.lastMessage = "Sound selection failed: \(error.localizedDescription)"
                }
            }
        }
    }
}

#Preview {
    ContentView()
        .environmentObject(SettingsStore())
        .environmentObject(AlarmCenter())
}
