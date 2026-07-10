import SwiftUI
import UniformTypeIdentifiers

struct AlarmsView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @State private var isImportingSound = false

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            CinematicKicker(title: "Alarms", icon: "bell", tint: .arAmber)

            // Upcoming alarms
            if alarmCenter.scheduledAlarms.isEmpty {
                Text("No pending alarms")
                    .font(.subheadline)
                    .foregroundStyle(.arTextMuted)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(14)
                    .smokedGlass(cornerRadius: 20, tint: .arSurface, shadow: false)
            } else {
                VStack(spacing: 0) {
                    ForEach(Array(alarmCenter.scheduledAlarms.enumerated()), id: \.element.id) { index, alarm in
                        alarmRow(alarm)

                        if index < alarmCenter.scheduledAlarms.count - 1 {
                            SectionDivider()
                                .padding(.leading, 30)
                        }
                    }
                }
                .minimalCard(cornerRadius: 20, padding: 0)
            }

            // Sound config — single row
            Button {
                isImportingSound = true
            } label: {
                HStack(spacing: 10) {
                    Image(systemName: "speaker.wave.2")
                        .font(.subheadline)
                        .foregroundStyle(.arTextSecondary)
                        .frame(width: 30)
                    Text("Alarm Sound")
                        .font(.subheadline)
                        .foregroundStyle(.arTextPrimary)
                    Spacer()
                    Text(soundSelectionLabel)
                        .font(.caption)
                        .foregroundStyle(.arTextSecondary)
                    Image(systemName: "chevron.right")
                        .font(.caption2)
                        .foregroundStyle(.arTextMuted)
                }
                .padding(.horizontal, 14)
                .frame(minHeight: 50)
            }
            .buttonStyle(.plain)
            .minimalCard(cornerRadius: 20, padding: 0)

            // Test alarm — single row
            Button {
                Task { await alarmCenter.scheduleTestAlarm(severity: .normal) }
            } label: {
                HStack(spacing: 10) {
                    Image(systemName: "waveform")
                        .font(.subheadline)
                        .foregroundStyle(.arTextSecondary)
                        .frame(width: 30)
                    Text("Test Alarm")
                        .font(.subheadline)
                        .foregroundStyle(.arTextPrimary)
                    Spacer()
                    Image(systemName: "chevron.right")
                        .font(.caption2)
                        .foregroundStyle(.arTextMuted)
                }
                .padding(.horizontal, 14)
                .frame(minHeight: 50)
            }
            .buttonStyle(.plain)
            .minimalCard(cornerRadius: 20, padding: 0)
        }
        .fileImporter(isPresented: $isImportingSound, allowedContentTypes: [.audio]) { result in
            switch result {
            case let .success(url):
                Task {
                    do {
                        settings.alarmSoundName = try await SoundLibrary.importAlarmSound(from: url)
                        settings.alarmSoundMode = AlarmSoundMode.custom.rawValue
                        alarmCenter.lastMessage = "Selected alarm sound: \(settings.alarmSoundName)"
                    } catch {
                        alarmCenter.lastMessage = "Sound import failed: \(error.localizedDescription)"
                    }
                }
            case let .failure(error):
                alarmCenter.lastMessage = "Sound selection failed: \(error.localizedDescription)"
            }
        }
    }

    // MARK: - Alarm Row

    private func alarmRow(_ alarm: AlarmJob) -> some View {
        HStack(spacing: 10) {
            Circle()
                .fill(alarm.severity.color)
                .frame(width: 6, height: 6)

            Text(alarm.title)
                .font(.subheadline)
                .foregroundStyle(.arTextPrimary)
                .lineLimit(1)

            Spacer()

            Text(alarm.fireAt.formatted(date: .omitted, time: .shortened))
                .font(.caption)
                .foregroundStyle(.arTextSecondary)
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 10)
    }

    // MARK: - Computed

    private var soundSelectionLabel: String {
        let mode = AlarmSoundMode(storedValue: settings.alarmSoundMode)
        switch mode {
        case .automatic:
            return "Auto"
        case .bundledNormal:
            return "Bundled normal"
        case .bundledLoud:
            return "Bundled loud"
        case .custom:
            return settings.alarmSoundName.isEmpty ? "Custom" : settings.alarmSoundName
        }
    }
}

#Preview {
    AlarmsView()
        .padding(.horizontal, 24)
        .background(Color.arBg)
        .environmentObject(SettingsStore())
        .environmentObject(AlarmCenter())
}
