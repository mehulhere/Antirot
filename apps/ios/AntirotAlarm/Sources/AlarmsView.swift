import SwiftUI
import UniformTypeIdentifiers

struct AlarmsView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @State private var isImportingSound = false

    var body: some View {
        ZStack {
            Color.antirotBg.ignoresSafeArea()

            ScrollView(.vertical, showsIndicators: false) {
                VStack(alignment: .leading, spacing: 28) {
                    // MARK: - Title
                    Text("Alarms")
                        .font(.title.bold())
                        .foregroundStyle(.antirotTextPrimary)

                    // MARK: - Test Alarms
                    testAlarmsSection

                    // MARK: - Sound Configuration
                    soundConfigSection

                    // MARK: - Upcoming
                    upcomingSection

                    // MARK: - Status
                    if !alarmCenter.lastMessage.isEmpty {
                        Text(alarmCenter.lastMessage)
                            .font(.footnote)
                            .foregroundStyle(.antirotTextMuted)
                            .frame(maxWidth: .infinity, alignment: .center)
                            .padding(.top, 4)
                    }
                }
                .padding(.horizontal, 20)
                .padding(.vertical, 16)
            }
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

    // MARK: - Test Alarms Section

    private var testAlarmsSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            AntirotSectionHeader(title: "Test alarms", icon: "waveform")

            HStack(spacing: 12) {
                testAlarmCard(
                    title: "Normal Test",
                    subtitle: "Standard wake alarm",
                    icon: "alarm",
                    accentColor: .antirotAccentOrange,
                    severity: .normal
                )

                testAlarmCard(
                    title: "Loud Test",
                    subtitle: "Emergency escalation",
                    icon: "alarm",
                    accentColor: .antirotAccentRed,
                    severity: .loud
                )
            }
        }
    }

    private func testAlarmCard(
        title: String,
        subtitle: String,
        icon: String,
        accentColor: Color,
        severity: AlarmJob.Severity
    ) -> some View {
        VStack(spacing: 12) {
            Image(systemName: icon)
                .font(.title2)
                .foregroundStyle(accentColor)

            VStack(spacing: 4) {
                Text(title)
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(.antirotTextPrimary)

                Text(subtitle)
                    .font(.caption)
                    .foregroundStyle(.antirotTextMuted)
            }

            Button {
                Task { await alarmCenter.scheduleTestAlarm(severity: severity) }
            } label: {
                Text("Schedule")
                    .font(.caption.weight(.medium))
            }
            .buttonStyle(AntirotGhostButtonStyle())
        }
        .frame(maxWidth: .infinity)
        .glassCard(cornerRadius: 14, padding: 16)
        .overlay(alignment: .top) {
            RoundedRectangle(cornerRadius: 14)
                .fill(accentColor)
                .frame(height: 2)
                .padding(.horizontal, 1)
        }
    }

    // MARK: - Sound Configuration Section

    private var soundConfigSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            AntirotSectionHeader(title: "Alarm sound", icon: "speaker.wave.2")

            VStack(alignment: .leading, spacing: 16) {
                // Segmented picker
                VStack(spacing: 8) {
                    Picker("Mode", selection: $settings.alarmSoundMode) {
                        ForEach(AlarmSoundMode.allCases) { mode in
                            Text(mode.label).tag(mode.rawValue)
                        }
                    }
                    .pickerStyle(.segmented)
                    .padding(8)
                    .background(
                        RoundedRectangle(cornerRadius: 10)
                            .fill(Color.antirotBgSecondary)
                    )
                }

                // Current selection
                HStack(spacing: 8) {
                    Image(systemName: "music.note")
                        .font(.caption)
                        .foregroundStyle(.antirotAccentOrange)
                    Text(soundSelectionLabel)
                        .font(.subheadline.weight(.medium))
                        .foregroundStyle(.antirotTextPrimary)
                }

                // Mode detail
                Text(AlarmSoundMode(storedValue: settings.alarmSoundMode).detail)
                    .font(.caption)
                    .foregroundStyle(.antirotTextMuted)
                    .fixedSize(horizontal: false, vertical: true)

                // Action buttons
                HStack(spacing: 10) {
                    Button {
                        isImportingSound = true
                    } label: {
                        Label("Choose sound", systemImage: "folder")
                    }
                    .buttonStyle(AntirotGhostButtonStyle())

                    Button {
                        settings.alarmSoundMode = AlarmSoundMode.automatic.rawValue
                        settings.alarmSoundName = ""
                        alarmCenter.lastMessage = "Alarm sound reset to automatic bundled sounds"
                    } label: {
                        Label("Reset to auto", systemImage: "arrow.counterclockwise")
                    }
                    .buttonStyle(AntirotGhostButtonStyle())
                }
            }
            .glassCard()
        }
    }

    // MARK: - Upcoming Section

    private var upcomingSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            AntirotSectionHeader(title: "Upcoming", icon: "clock")

            if alarmCenter.scheduledAlarms.isEmpty {
                VStack(spacing: 12) {
                    Image(systemName: "moon.stars")
                        .font(.title)
                        .foregroundStyle(.antirotTextMuted)
                    Text("Nothing on the horizon. Your coach will schedule alarms as needed.")
                        .font(.subheadline)
                        .foregroundStyle(.antirotTextMuted)
                        .multilineTextAlignment(.center)
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 24)
                .glassCard()
            } else {
                VStack(spacing: 10) {
                    ForEach(alarmCenter.scheduledAlarms) { alarm in
                        alarmRow(alarm)
                    }
                }
            }
        }
    }

    private func alarmRow(_ alarm: AlarmJob) -> some View {
        HStack(spacing: 14) {
            // Severity bar
            RoundedRectangle(cornerRadius: 2)
                .fill(alarm.severity.color)
                .frame(width: 4)

            VStack(alignment: .leading, spacing: 6) {
                HStack(spacing: 8) {
                    // Severity badge
                    Text(alarm.severity.label)
                        .font(.caption2.weight(.bold))
                        .foregroundStyle(alarm.severity.color)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(
                            Capsule()
                                .fill(alarm.severity.color.opacity(0.15))
                        )

                    Spacer()

                    // Fire time
                    Text(alarm.fireAt.formatted(date: .omitted, time: .shortened))
                        .font(.caption)
                        .foregroundStyle(.antirotTextSecondary)
                }

                Text(alarm.title)
                    .font(.headline)
                    .foregroundStyle(.antirotTextPrimary)

                Text(alarm.message)
                    .font(.subheadline)
                    .foregroundStyle(.antirotTextSecondary)
                    .lineLimit(2)
            }
        }
        .glassCard(cornerRadius: 14, padding: 14)
    }

    // MARK: - Computed Properties

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
}

#Preview {
    AlarmsView()
        .environmentObject(SettingsStore())
        .environmentObject(AlarmCenter())
}
