import SwiftUI
import UIKit
import UniformTypeIdentifiers

struct ContentView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @State private var screenTimeMessage = "Not requested"
    @State private var isImportingSound = false
    @State private var showDeveloperSettings = false
    @State private var showFullError = false
    @State private var pairingCode = ""

    var body: some View {
        NavigationStack {
            Form {
                if settings.registered {
                    connectedContent
                } else {
                    loginContent
                }
                bottomSettings
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

    @ViewBuilder
    private var loginContent: some View {
        Section {
            VStack(alignment: .center, spacing: 16) {
                Image("favicon")
                    .resizable()
                    .frame(width: 72, height: 72)
                    .clipShape(RoundedRectangle(cornerRadius: 16))
                    .accessibilityHidden(true)
                VStack(spacing: 6) {
                    Text("Sign in to Antirot")
                        .font(.title2.weight(.semibold))
                    Text("Link this phone to your coach and alarms.")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                        .multilineTextAlignment(.center)
                }
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 20)

            Button("Continue with Google") {
                Task { await signInWithGoogle() }
            }
            .font(.headline)

            if !alarmCenter.lastMessage.isEmpty {
                Text(alarmCenter.lastMessage)
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }

            if alarmCenter.lastErrorDetails != nil {
                Button("Show full error") {
                    showFullError = true
                }
            }
        }
    }

    @ViewBuilder
    private var connectedContent: some View {
        Section {
            HStack(spacing: 12) {
                Image("favicon")
                    .resizable()
                    .frame(width: 44, height: 44)
                    .clipShape(RoundedRectangle(cornerRadius: 10))
                    .accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 3) {
                    Text("Connected")
                        .font(.headline)
                    Text(settings.statusMessage)
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }
            }

            Button("Check alarms now") {
                Task { await alarmCenter.pollPendingAlarms() }
            }

            if !alarmCenter.lastMessage.isEmpty {
                Text(alarmCenter.lastMessage)
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
        }

        Section("Pair with coach") {
            Text("Run the pairing command on your VPS, then enter the 6-digit code here.")
                .font(.footnote)
                .foregroundStyle(.secondary)
            TextField("6-digit code", text: $pairingCode)
                .keyboardType(.numberPad)
                .textContentType(.oneTimeCode)
                .onChange(of: pairingCode) { _, newValue in
                    pairingCode = String(newValue.filter(\.isNumber).prefix(6))
                }
            Button("Pair device") {
                Task { await pairDevice() }
            }
            .disabled(pairingCode.count != 6)
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

    @ViewBuilder
    private var bottomSettings: some View {
        Section("Settings") {
            DisclosureGroup("Permissions") {
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

            DisclosureGroup("Alarm sound") {
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
            }

            DisclosureGroup("Widget") {
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
            }

            DisclosureGroup("Device details") {
                LabeledContent("Server", value: URL(string: settings.effectiveServerURL)?.host() ?? "api.antirot.org")
                LabeledContent("Device ID", value: settings.deviceId)
                LabeledContent("Status", value: settings.statusMessage)
                Button("Register device") {
                    Task { await alarmCenter.registerDevice() }
                }
                Button("Schedule normal test alarm") {
                    Task { await alarmCenter.scheduleTestAlarm(severity: .normal) }
                }
                Button("Schedule loud test alarm") {
                    Task { await alarmCenter.scheduleTestAlarm(severity: .loud) }
                }
                if alarmCenter.lastErrorDetails != nil {
                    Button("Show full error") {
                        showFullError = true
                    }
                }
            }

            DisclosureGroup("Developer") {
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
                }
            }
        }

        if settings.registered {
            Section {
                Button("Logout", role: .destructive) {
                    resetBridgeSession(message: "Logged out. Sign in again when you're ready.")
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
        pairingCode = ""
        alarmCenter.lastMessage = message
        alarmCenter.lastErrorDetails = nil
    }

    private func pairDevice() async {
        do {
            let request = PairingClaimRequest(
                code: pairingCode,
                deviceId: settings.deviceId,
                deviceName: UIDevice.current.name,
                platform: "ios"
            )
            let response = try await APIClient(
                baseURL: settings.baseURL,
                apiToken: settings.apiToken
            ).claimPairing(request)
            pairingCode = ""
            settings.statusMessage = response.message
            alarmCenter.lastMessage = "Paired with coach."
            alarmCenter.lastErrorDetails = nil
        } catch {
            alarmCenter.lastMessage = "Pairing failed"
            alarmCenter.lastErrorDetails = error.localizedDescription
        }
    }
}

#Preview {
    ContentView()
        .environmentObject(SettingsStore())
        .environmentObject(AlarmCenter())
}
