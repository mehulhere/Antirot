import SwiftUI
import UniformTypeIdentifiers

struct ContentView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @State private var screenTimeMessage = "Not requested"
    @State private var isImportingSound = false
    @State private var showDeveloperSettings = false
    @State private var showFullError = false

    var body: some View {
        NavigationStack {
            Form {
                Section("Bridge") {
                    HStack(spacing: 12) {
                        Image("favicon")
                            .resizable()
                            .frame(width: 36, height: 36)
                            .clipShape(RoundedRectangle(cornerRadius: 8))
                            .accessibilityHidden(true)
                        VStack(alignment: .leading, spacing: 2) {
                            Text("Antirot")
                                .font(.headline)
                            Text("Sign in to link this phone to your coach.")
                                .font(.footnote)
                                .foregroundStyle(.secondary)
                        }
                    }
                    LabeledContent("Server", value: URL(string: settings.effectiveServerURL)?.host() ?? "api.antirot.org")
                    LabeledContent("Device ID", value: settings.deviceId)
                    LabeledContent("Status", value: settings.statusMessage)
                    Button("Continue with Google") {
                        Task { await signInWithGoogle() }
                    }
                    Button("Register device") {
                        Task { await alarmCenter.registerDevice() }
                    }
                    Button("Reset local login", role: .destructive) {
                        resetBridgeSession(message: "Local bridge login reset. Sign in again when you're ready.")
                    }
                    if alarmCenter.lastErrorDetails != nil {
                        Button("Show full error") {
                            showFullError = true
                        }
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
                    if alarmCenter.lastErrorDetails != nil {
                        Button("Show full error") {
                            showFullError = true
                        }
                    }
                }

                Section("Alarm Sound") {
                    Picker("Mode", selection: $settings.alarmSoundMode) {
                        ForEach(AlarmSoundMode.allCases) { mode in
                            Text(mode.label).tag(mode.rawValue)
                        }
                    }
                    .pickerStyle(.segmented)
                    LabeledContent("Selected", value: soundSelectionLabel)
                    Text(AlarmSoundMode(storedValue: settings.alarmSoundMode).detail)
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                    Button("Choose sound file") {
                        isImportingSound = true
                    }
                    Button("Use automatic bundled sounds") {
                        settings.alarmSoundMode = AlarmSoundMode.automatic.rawValue
                        settings.alarmSoundName = ""
                        alarmCenter.lastMessage = "Alarm sound reset to automatic bundled sounds"
                    }
                    Text("Custom files must be 30 seconds or shorter. Antirot copies them into the iOS Library/Sounds folder and uses them for AlarmKit when available.")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }

                Section("Widget") {
                    Button("Show current task in widget") {
                        let updated = SharedTaskStore.write(CurrentTaskSnapshot(
                            title: "Start one real work block",
                            subtitle: "Enough setup. Put one task on the board.",
                            mode: "working",
                            dueAt: Date().addingTimeInterval(45 * 60)
                        ))
                        alarmCenter.lastMessage = updated
                            ? "Widget updated. If it stays stale, remove and re-add the widget once."
                            : "Widget update failed: app-group storage unavailable in this install."
                    }
                    LabeledContent("App group", value: SharedTaskStore.canAccessAppGroup() ? "Available" : "Unavailable")
                    Text("Add the Antirot Current Task widget from the iOS Home Screen after installing the app. If this says app-group unavailable, the current signing method is blocking widget shared storage.")
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

                Section("Developer Settings") {
                    Toggle("Show bridge credentials", isOn: $showDeveloperSettings)
                    if showDeveloperSettings {
                        TextField("https://api.antirot.org", text: $settings.serverURL)
                            .textContentType(.URL)
                            .keyboardType(.URL)
                            .textInputAutocapitalization(.never)
                        LabeledContent("Effective URL", value: settings.effectiveServerURL)
                        SecureField("API token", text: $settings.apiToken)
                        Button("Reset server to api.antirot.org") {
                            settings.serverURL = SettingsStore.defaultServerURL
                            alarmCenter.lastMessage = "Bridge server reset to api.antirot.org"
                        }
                        Button("Reset bridge session", role: .destructive) {
                            resetBridgeSession(message: "Bridge session reset. Sign in again when you're ready.")
                        }
                        Text("Paste the device token from /etc/antirot/bridge.env. Do not commit or share that token.")
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                    }
                }
            }
            .navigationTitle("Antirot")
            .alert("Full Error", isPresented: $showFullError) {
                Button("OK", role: .cancel) {}
            } message: {
                Text(alarmCenter.lastErrorDetails ?? "No error details.")
            }
            .fileImporter(isPresented: $isImportingSound, allowedContentTypes: [.audio]) { result in
                switch result {
                case let .success(url):
                    do {
                        settings.alarmSoundName = try SoundLibrary.importAlarmSound(from: url)
                        settings.alarmSoundMode = AlarmSoundMode.custom.rawValue
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

    private var soundSelectionLabel: String {
        let mode = AlarmSoundMode(storedValue: settings.alarmSoundMode)
        switch mode {
        case .automatic:
            return "Auto: normal + loud"
        case .bundledNormal:
            return "Bundled normal"
        case .bundledLoud:
            return "Bundled loud"
        case .custom:
            return settings.alarmSoundName.isEmpty ? "Custom not imported yet" : settings.alarmSoundName
        }
    }

    private func signInWithGoogle() async {
        do {
            let response = try await GoogleAuthCenter.signIn(settings: settings)
            alarmCenter.lastMessage = "Signed in as \(response.email)"
            alarmCenter.lastErrorDetails = nil
            await alarmCenter.registerDevice()
        } catch {
            settings.statusMessage = "Google sign-in failed"
            alarmCenter.lastMessage = "Google sign-in failed"
            alarmCenter.lastErrorDetails = error.localizedDescription
        }
    }

    private func resetBridgeSession(message: String) {
        settings.resetBridgeSession()
        alarmCenter.lastMessage = message
        alarmCenter.lastErrorDetails = nil
    }
}

#Preview {
    ContentView()
        .environmentObject(SettingsStore())
        .environmentObject(AlarmCenter())
}
