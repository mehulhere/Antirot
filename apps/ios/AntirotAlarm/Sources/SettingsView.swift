import SwiftUI
import UIKit

struct SettingsView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @State private var screenTimeMessage = "Not requested"
    @State private var showDeveloperSettings = false
    @State private var showFullError = false

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            AntirotSectionHeader(title: "Settings", icon: "gearshape")

            // Account
            HStack(spacing: 12) {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Signed in")
                        .font(.subheadline)
                        .foregroundStyle(.arTextPrimary)
                    Text(settings.statusMessage)
                        .font(.caption)
                        .foregroundStyle(.arTextSecondary)
                }
                Spacer()
                Button {
                    resetBackendSession()
                } label: {
                    Text("Logout")
                }
                .buttonStyle(AntirotDestructiveButtonStyle())
            }
            .minimalCard(cornerRadius: 12, padding: 14)

            // Permissions — compact 3-dot row
            VStack(alignment: .leading, spacing: 10) {
                Text("PERMISSIONS")
                    .font(.caption2.weight(.medium))
                    .tracking(1.0)
                    .foregroundStyle(.arTextMuted)

                HStack(spacing: 20) {
                    permissionDot(
                        label: "Notifications",
                        color: notificationStatusColor
                    ) {
                        Task { await alarmCenter.requestNotificationPermission() }
                    }

                    permissionDot(
                        label: "AlarmKit",
                        color: alarmCenter.alarmKitStatus.contains("authorized")
                            ? .arSuccess : .arDanger
                    ) {
                        Task { await alarmCenter.requestAlarmKitPermission() }
                    }

                    permissionDot(
                        label: "Screen Time",
                        color: screenTimeStatusColor
                    ) {
                        Task {
                            let result = await ScreenTimeCenter.requestAuthorization()
                            screenTimeMessage = result
                            if !result.contains("authorized") && result != "Not requested" {
                                alarmCenter.lastErrorDetails = result
                            }
                        }
                    }

                    Spacer()
                }
            }
            .minimalCard(cornerRadius: 12, padding: 14)

            // System info
            VStack(spacing: 0) {
                infoRow(
                    label: "Device ID",
                    value: String(settings.deviceId.prefix(12)) + "..."
                )
                SectionDivider()
                infoRow(
                    label: "Server",
                    value: URL(string: settings.effectiveServerURL)?.host() ?? "api.antirot.org"
                )
            }
            .minimalCard(cornerRadius: 12, padding: 0)

            // Developer (hidden toggle)
            VStack(spacing: 0) {
                Button {
                    withAnimation(.easeInOut(duration: 0.25)) {
                        showDeveloperSettings.toggle()
                    }
                } label: {
                    HStack {
                        Text("Developer")
                            .font(.subheadline)
                            .foregroundStyle(.arTextMuted)
                        Spacer()
                        Image(systemName: showDeveloperSettings ? "chevron.down" : "chevron.right")
                            .font(.caption2)
                            .foregroundStyle(.arTextMuted)
                    }
                    .padding(.horizontal, 14)
                    .padding(.vertical, 12)
                }
                .buttonStyle(.plain)

                if showDeveloperSettings {
                    SectionDivider()

                    VStack(alignment: .leading, spacing: 12) {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("Server URL")
                                .font(.caption)
                                .foregroundStyle(.arTextMuted)
                            TextField("https://api.antirot.org", text: $settings.serverURL)
                                .textContentType(.URL)
                                .keyboardType(.URL)
                                .textInputAutocapitalization(.never)
                                .font(.subheadline)
                                .foregroundStyle(.arTextPrimary)
                                .padding(10)
                                .background(Color.arElevated)
                                .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                        }

                        VStack(alignment: .leading, spacing: 4) {
                            Text("API Token")
                                .font(.caption)
                                .foregroundStyle(.arTextMuted)
                            SecureField("API token", text: $settings.apiToken)
                                .font(.subheadline)
                                .foregroundStyle(.arTextPrimary)
                                .padding(10)
                                .background(Color.arElevated)
                                .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                        }
                    }
                    .padding(14)
                    .transition(.opacity.combined(with: .move(edge: .top)))
                }
            }
            .minimalCard(cornerRadius: 12, padding: 0)
        }
        .alert("Full Error", isPresented: $showFullError) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(alarmCenter.lastErrorDetails ?? "No error details.")
        }
    }

    // MARK: - Components

    @ViewBuilder
    private func permissionDot(label: String, color: Color, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            VStack(spacing: 6) {
                StatusDot(color: color, animated: false)
                Text(label)
                    .font(.caption2)
                    .foregroundStyle(.arTextSecondary)
            }
        }
        .buttonStyle(.plain)
    }

    private func infoRow(label: String, value: String) -> some View {
        HStack {
            Text(label)
                .font(.subheadline)
                .foregroundStyle(.arTextMuted)
            Spacer()
            Text(value)
                .font(.caption.monospaced())
                .foregroundStyle(.arTextSecondary)
                .lineLimit(1)
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 10)
    }

    // MARK: - Helpers

    private var notificationStatusColor: Color {
        switch alarmCenter.notificationStatus {
        case .authorized:
            return .arSuccess
        case .provisional:
            return .arWarning
        default:
            return .arDanger
        }
    }

    private var screenTimeStatusColor: Color {
        if screenTimeMessage.contains("authorized") {
            return .arSuccess
        } else if screenTimeMessage == "Not requested" {
            return .arWarning
        } else {
            return .arDanger
        }
    }

    private func resetBackendSession() {
        settings.resetBackendSession()
        alarmCenter.lastMessage = "Logged out. Sign in again when you're ready."
        alarmCenter.lastErrorDetails = nil
    }
}

#Preview {
    SettingsView()
        .padding(.horizontal, 24)
        .background(Color.arBg)
        .environmentObject(SettingsStore())
        .environmentObject(AlarmCenter())
}
