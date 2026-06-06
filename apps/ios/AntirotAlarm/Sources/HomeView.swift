import SwiftUI
import UIKit

struct HomeView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @State private var pairingCode = ""

    var body: some View {
        ZStack {
            Color.antirotBg.ignoresSafeArea()

            ScrollView(.vertical, showsIndicators: false) {
                VStack(spacing: 28) {
                    headerBar
                    currentTaskCard
                    quickActions
                    if settings.registered {
                        pairingSection
                    }
                    scheduledAlarmsSection
                    statusToast
                    Spacer(minLength: 40)
                }
                .padding(.horizontal, 20)
                .padding(.top, 12)
            }
        }
    }

    // MARK: - Header

    private var headerBar: some View {
        HStack(spacing: 10) {
            Image("favicon")
                .resizable()
                .frame(width: 36, height: 36)
                .clipShape(RoundedRectangle(cornerRadius: 10))
                .accessibilityHidden(true)

            Text("Antirot")
                .font(.title3.bold())
                .foregroundStyle(.antirotTextPrimary)

            Spacer()

            HStack(spacing: 6) {
                if settings.registered {
                    StatusDot(color: .antirotSuccess)
                    Text("Connected")
                        .font(.caption.weight(.medium))
                        .foregroundStyle(.antirotTextSecondary)
                } else {
                    StatusDot(color: .antirotAccentRed)
                    Text("Offline")
                        .font(.caption.weight(.medium))
                        .foregroundStyle(.antirotTextSecondary)
                }
            }
        }
        .padding(.vertical, 4)
    }

    // MARK: - Current Task Card

    private var currentTaskCard: some View {
        let snapshot = SharedTaskStore.read()

        return HStack(spacing: 0) {
            // Leading accent bar
            RoundedRectangle(cornerRadius: 2)
                .fill(Color.antirotAccentOrange)
                .frame(width: 3)
                .padding(.vertical, 4)

            VStack(alignment: .leading, spacing: 10) {
                // Mode badge
                Text(snapshot.mode.uppercased())
                    .font(.caption2.weight(.bold))
                    .tracking(1)
                    .foregroundStyle(.white)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 4)
                    .background(
                        Capsule()
                            .fill(Color.antirotAccentOrange)
                    )

                // Title
                Text(snapshot.title)
                    .font(.title3.bold())
                    .foregroundStyle(.antirotTextPrimary)
                    .lineLimit(2)

                // Subtitle
                Text(snapshot.subtitle)
                    .font(.subheadline)
                    .foregroundStyle(.antirotTextSecondary)
                    .lineLimit(3)

                // Countdown / due time
                if let dueAt = snapshot.dueAt {
                    HStack(spacing: 6) {
                        Image(systemName: "clock")
                            .font(.caption)
                            .foregroundStyle(.antirotAccentAmber)
                        Text(dueAt, style: .relative)
                            .font(.caption.weight(.medium))
                            .foregroundStyle(.antirotTextMuted)
                        Text("remaining")
                            .font(.caption)
                            .foregroundStyle(.antirotTextMuted)
                    }
                    .padding(.top, 2)
                }
            }
            .padding(.leading, 14)
        }
        .glassCard()
        .accentGlow(color: .antirotAccentOrange, radius: 16)
    }

    // MARK: - Quick Actions

    private var quickActions: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 12) {
                quickActionButton(
                    icon: "arrow.clockwise",
                    label: "Check alarms"
                ) {
                    Task { await alarmCenter.pollPendingAlarms() }
                }

                quickActionButton(
                    icon: "rectangle.on.rectangle",
                    label: "Widget preview"
                ) {
                    let _ = SharedTaskStore.write(CurrentTaskSnapshot(
                        title: "Start one real work block",
                        subtitle: "Enough setup. Put one task on the board.",
                        mode: "working",
                        dueAt: Date().addingTimeInterval(45 * 60)
                    ))
                }
            }
        }
    }

    private func quickActionButton(
        icon: String,
        label: String,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            VStack(spacing: 8) {
                Image(systemName: icon)
                    .font(.title3)
                    .foregroundStyle(.antirotTextPrimary)
                Text(label)
                    .font(.caption2.weight(.medium))
                    .foregroundStyle(.antirotTextSecondary)
                    .lineLimit(1)
            }
            .frame(width: 80, height: 72)
        }
        .glassCard(cornerRadius: 12, padding: 12)
    }

    // MARK: - Pairing Section

    private var pairingSection: some View {
        VStack(alignment: .leading, spacing: 14) {
            AntirotSectionHeader(title: "Pair with coach", icon: "link")

            VStack(alignment: .leading, spacing: 14) {
                Text("Run the pairing command on your VPS, then enter the 6-digit code here.")
                    .font(.footnote)
                    .foregroundStyle(.antirotTextMuted)

                HStack(spacing: 10) {
                    TextField("000000", text: $pairingCode)
                        .keyboardType(.numberPad)
                        .textContentType(.oneTimeCode)
                        .font(.body.monospaced().weight(.semibold))
                        .foregroundStyle(.antirotTextPrimary)
                        .padding(.horizontal, 14)
                        .padding(.vertical, 12)
                        .background(
                            RoundedRectangle(cornerRadius: 10)
                                .fill(Color.antirotBgSecondary)
                        )
                        .overlay(
                            RoundedRectangle(cornerRadius: 10)
                                .strokeBorder(Color.antirotBorder, lineWidth: 1)
                        )
                        .onChange(of: pairingCode) { _, newValue in
                            pairingCode = String(newValue.filter(\.isNumber).prefix(6))
                        }

                    Button("Pair") {
                        Task { await pairDevice() }
                    }
                    .buttonStyle(AntirotAccentButtonStyle())
                    .disabled(pairingCode.count != 6)
                    .opacity(pairingCode.count == 6 ? 1 : 0.5)
                }
            }
            .glassCard()
        }
    }

    // MARK: - Scheduled Alarms

    private var scheduledAlarmsSection: some View {
        VStack(alignment: .leading, spacing: 14) {
            AntirotSectionHeader(title: "Scheduled", icon: "alarm")

            if alarmCenter.scheduledAlarms.isEmpty {
                emptyAlarmsCard
            } else {
                ForEach(alarmCenter.scheduledAlarms) { alarm in
                    alarmCard(alarm)
                }
            }
        }
    }

    private var emptyAlarmsCard: some View {
        HStack(spacing: 12) {
            Image(systemName: "moon.stars")
                .font(.title2)
                .foregroundStyle(.antirotTextMuted)
            Text("No alarms scheduled. Either you're disciplined or you haven't started yet.")
                .font(.subheadline)
                .foregroundStyle(.antirotTextMuted)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .glassCard()
    }

    private func alarmCard(_ alarm: AlarmJob) -> some View {
        HStack(spacing: 0) {
            // Leading severity bar
            RoundedRectangle(cornerRadius: 2)
                .fill(alarm.severity.color)
                .frame(width: 3)
                .padding(.vertical, 4)

            VStack(alignment: .leading, spacing: 8) {
                HStack(spacing: 10) {
                    // Severity badge
                    Text(alarm.severity.label)
                        .font(.caption2.weight(.bold))
                        .tracking(0.8)
                        .foregroundStyle(.white)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(
                            Capsule()
                                .fill(alarm.severity.color)
                        )

                    Spacer()

                    // Fire time
                    Text(alarm.fireAt.formatted(date: .omitted, time: .shortened))
                        .font(.caption.weight(.medium))
                        .foregroundStyle(.antirotTextMuted)
                }

                Text(alarm.title)
                    .font(.headline)
                    .foregroundStyle(.antirotTextPrimary)

                Text(alarm.message)
                    .font(.subheadline)
                    .foregroundStyle(.antirotTextSecondary)
                    .lineLimit(2)
            }
            .padding(.leading, 14)
        }
        .glassCard()
    }

    // MARK: - Status Toast

    @ViewBuilder
    private var statusToast: some View {
        if !alarmCenter.lastMessage.isEmpty {
            HStack(spacing: 10) {
                Image(systemName: "info.circle.fill")
                    .font(.subheadline)
                    .foregroundStyle(.antirotTextMuted)
                Text(alarmCenter.lastMessage)
                    .font(.caption)
                    .foregroundStyle(.antirotTextSecondary)
                    .lineLimit(2)
                Spacer()
            }
            .glassCard(cornerRadius: 12, padding: 14, showBorder: true)
            .transition(.move(edge: .bottom).combined(with: .opacity))
        }
    }

    // MARK: - Actions

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
    HomeView()
        .environmentObject(SettingsStore())
        .environmentObject(AlarmCenter())
}
