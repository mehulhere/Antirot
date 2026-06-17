import SwiftUI
import UIKit

struct SettingsView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @State private var screenTimeMessage = "Not requested"
    @State private var showDeveloperSettings = false
    @State private var showFullError = false
    @State private var showConsole = false

    var body: some View {
        ZStack {
            Color.antirotBg.ignoresSafeArea()

            ScrollView(.vertical, showsIndicators: false) {
                VStack(alignment: .leading, spacing: 28) {
                    Text("Settings")
                        .font(.title.bold())
                        .foregroundStyle(.antirotTextPrimary)

                    accountSection
                    permissionsSection
                    widgetSection
                    deviceSection
                    developerSection
                    consoleSection
                    statusToast
                }
                .padding(.horizontal, 20)
                .padding(.bottom, 40)
            }
        }
        .alert("Full Error", isPresented: $showFullError) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(alarmCenter.lastErrorDetails ?? "No error details.")
        }
    }

    // MARK: - Account

    @ViewBuilder
    private var accountSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            AntirotSectionHeader(title: "Account", icon: "person.circle")

            VStack(spacing: 0) {
                HStack(spacing: 14) {
                    Image("favicon")
                        .resizable()
                        .frame(width: 36, height: 36)
                        .clipShape(RoundedRectangle(cornerRadius: 8))
                        .accessibilityHidden(true)

                    VStack(alignment: .leading, spacing: 3) {
                        Text("Signed in")
                            .font(.headline)
                            .foregroundStyle(.antirotTextPrimary)
                        Text(settings.statusMessage)
                            .font(.caption)
                            .foregroundStyle(.antirotTextMuted)
                    }

                    Spacer()
                }

                Divider()
                    .overlay(Color.antirotBorder)
                    .padding(.vertical, 14)

                Button {
                    resetBackendSession()
                } label: {
                    HStack {
                        Image(systemName: "rectangle.portrait.and.arrow.right")
                        Text("Logout")
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(AntirotDestructiveButtonStyle())
            }
            .glassCard()
        }
    }

    // MARK: - Permissions

    @ViewBuilder
    private var permissionsSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            AntirotSectionHeader(title: "Permissions", icon: "checkmark.shield")

            // Notifications
            permissionCard(
                icon: "bell.badge",
                label: "Notifications",
                value: String(describing: alarmCenter.notificationStatus),
                statusColor: notificationStatusColor
            ) {
                Task { await alarmCenter.requestNotificationPermission() }
            }

            // AlarmKit
            permissionCard(
                icon: "alarm",
                label: "AlarmKit",
                value: alarmCenter.alarmKitStatus,
                statusColor: alarmCenter.alarmKitStatus.contains("authorized")
                    ? .antirotSuccess : .antirotAccentRed
            ) {
                Task { await alarmCenter.requestAlarmKitPermission() }
            }

            // Screen Time
            permissionCard(
                icon: "hourglass",
                label: "Screen Time",
                value: screenTimeMessage,
                statusColor: screenTimeStatusColor
            ) {
                Task {
                    let result = await ScreenTimeCenter.requestAuthorization()
                    screenTimeMessage = result
                    if !result.contains("authorized") && result != "Not requested" {
                        alarmCenter.lastErrorDetails = result
                    }
                }
            }
        }
    }

    @ViewBuilder
    private func permissionCard(
        icon: String,
        label: String,
        value: String,
        statusColor: Color,
        action: @escaping () -> Void
    ) -> some View {
        HStack(spacing: 12) {
            StatusDot(color: statusColor)

            Image(systemName: icon)
                .font(.subheadline)
                .foregroundStyle(.antirotTextSecondary)

            Text(label)
                .font(.subheadline.weight(.medium))
                .foregroundStyle(.antirotTextPrimary)

            Spacer()

            Text(value)
                .font(.caption)
                .foregroundStyle(.antirotTextMuted)
                .lineLimit(1)

            Button("Request", action: action)
                .buttonStyle(AntirotGhostButtonStyle())
        }
        .glassCard(padding: 14)
    }

    // MARK: - Widget

    @ViewBuilder
    private var widgetSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            AntirotSectionHeader(title: "Widget", icon: "rectangle.on.rectangle")

            VStack(spacing: 14) {
                Button {
                    let updated = SharedTaskStore.write(CurrentTaskSnapshot(
                        title: "Start one real work block",
                        subtitle: "Enough setup. Put one task on the board.",
                        mode: "working",
                        dueAt: Date().addingTimeInterval(45 * 60)
                    ))
                    alarmCenter.lastMessage = updated
                        ? "Widget updated. If it stays stale, remove and re-add the widget once."
                        : "Widget update failed: app-group storage unavailable in this install."
                } label: {
                    HStack {
                        Image(systemName: "arrow.triangle.2.circlepath")
                        Text("Show current task in widget")
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(AntirotGhostButtonStyle())

                HStack(spacing: 8) {
                    StatusDot(
                        color: SharedTaskStore.canAccessAppGroup()
                            ? .antirotSuccess : .antirotAccentRed,
                        animated: false
                    )
                    Text("App group:")
                        .font(.caption)
                        .foregroundStyle(.antirotTextMuted)
                    Text(SharedTaskStore.canAccessAppGroup() ? "Available" : "Unavailable")
                        .font(.caption.weight(.medium))
                        .foregroundStyle(
                            SharedTaskStore.canAccessAppGroup()
                                ? .antirotSuccess : .antirotAccentRed
                        )
                    Spacer()
                }
            }
            .glassCard()
        }
    }

    // MARK: - Device Details

    @ViewBuilder
    private var deviceSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            AntirotSectionHeader(title: "Device", icon: "iphone")

            VStack(spacing: 12) {
                deviceInfoRow(
                    label: "Server",
                    value: URL(string: settings.effectiveServerURL)?.host() ?? "api.antirot.org"
                )
                deviceInfoRow(
                    label: "Device ID",
                    value: String(settings.deviceId.prefix(12)) + "...",
                    monospaced: true
                )
                deviceInfoRow(label: "Status", value: settings.statusMessage)

                Divider()
                    .overlay(Color.antirotBorder)

                Button {
                    Task { await alarmCenter.registerDevice() }
                } label: {
                    HStack {
                        Image(systemName: "arrow.clockwise")
                        Text("Register device")
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(AntirotGhostButtonStyle())
            }
            .glassCard()
        }
    }

    @ViewBuilder
    private func deviceInfoRow(label: String, value: String, monospaced: Bool = false) -> some View {
        HStack {
            Text(label)
                .font(.subheadline)
                .foregroundStyle(.antirotTextMuted)
            Spacer()
            Text(value)
                .font(monospaced ? .caption.monospaced() : .subheadline)
                .foregroundStyle(.antirotTextSecondary)
                .lineLimit(1)
        }
    }

    // MARK: - Developer

    @ViewBuilder
    private var developerSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            AntirotSectionHeader(title: "Developer", icon: "wrench")

            VStack(spacing: 0) {
                Toggle(isOn: $showDeveloperSettings.animation(.easeInOut(duration: 0.25))) {
                    HStack(spacing: 8) {
                        Image(systemName: "chevron.left.forwardslash.chevron.right")
                            .font(.caption)
                            .foregroundStyle(.antirotAccentOrange)
                        Text("Show developer tools")
                            .font(.subheadline.weight(.medium))
                            .foregroundStyle(.antirotTextPrimary)
                    }
                }
                .tint(.antirotAccentRed)

                if showDeveloperSettings {
                    Divider()
                        .overlay(Color.antirotBorder)
                        .padding(.vertical, 14)

                    VStack(spacing: 14) {
                        VStack(alignment: .leading, spacing: 6) {
                            Text("Server URL")
                                .font(.caption)
                                .foregroundStyle(.antirotTextMuted)
                            TextField("https://api.antirot.org", text: $settings.serverURL)
                                .textContentType(.URL)
                                .keyboardType(.URL)
                                .textInputAutocapitalization(.never)
                                .font(.subheadline)
                                .foregroundStyle(.antirotTextPrimary)
                                .padding(12)
                                .background(Color.antirotBgSecondary)
                                .clipShape(RoundedRectangle(cornerRadius: 10))
                                .overlay(
                                    RoundedRectangle(cornerRadius: 10)
                                        .strokeBorder(Color.antirotBorder, lineWidth: 1)
                                )
                        }

                        HStack {
                            Text("Effective URL")
                                .font(.caption)
                                .foregroundStyle(.antirotTextMuted)
                            Spacer()
                            Text(settings.effectiveServerURL)
                                .font(.caption)
                                .foregroundStyle(.antirotTextSecondary)
                                .lineLimit(1)
                        }

                        VStack(alignment: .leading, spacing: 6) {
                            Text("API Token")
                                .font(.caption)
                                .foregroundStyle(.antirotTextMuted)
                            SecureField("API token", text: $settings.apiToken)
                                .font(.subheadline)
                                .foregroundStyle(.antirotTextPrimary)
                                .padding(12)
                                .background(Color.antirotBgSecondary)
                                .clipShape(RoundedRectangle(cornerRadius: 10))
                                .overlay(
                                    RoundedRectangle(cornerRadius: 10)
                                        .strokeBorder(Color.antirotBorder, lineWidth: 1)
                                )
                        }

                        Button {
                            settings.serverURL = SettingsStore.defaultServerURL
                            alarmCenter.lastMessage = "Backend server reset to api.antirot.org"
                        } label: {
                            HStack {
                                Image(systemName: "arrow.counterclockwise")
                                Text("Reset to api.antirot.org")
                            }
                            .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(AntirotGhostButtonStyle())
                    }
                    .transition(.opacity.combined(with: .move(edge: .top)))
                }
            }
            .glassCard()
        }
    }

    // MARK: - Console Section

    @ViewBuilder
    private var consoleSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            AntirotSectionHeader(title: "Console", icon: "terminal")

            VStack(spacing: 0) {
                Toggle(isOn: $showConsole.animation(.easeInOut(duration: 0.25))) {
                    HStack(spacing: 8) {
                        Image(systemName: "terminal")
                            .font(.caption)
                            .foregroundStyle(.antirotAccentOrange)
                        Text("See Console")
                            .font(.subheadline.weight(.medium))
                            .foregroundStyle(.antirotTextPrimary)
                    }
                }
                .tint(.antirotAccentRed)

                if showConsole {
                    Divider()
                        .overlay(Color.antirotBorder)
                        .padding(.vertical, 14)

                    VStack(alignment: .leading, spacing: 12) {
                        ScrollView(.vertical) {
                            VStack(alignment: .leading, spacing: 8) {
                                if let error = alarmCenter.lastErrorDetails {
                                    Text(error)
                                        .font(.system(.caption, design: .monospaced))
                                        .foregroundStyle(.red)
                                } else {
                                    Text("[SYSTEM OK] No errors reported.")
                                        .font(.system(.caption, design: .monospaced))
                                        .foregroundStyle(.green)
                                }
                                
                                Text("\n[DIAGNOSTICS]\nDevice ID: \(settings.deviceId)\nServer: \(settings.effectiveServerURL)\nPush Token: \(settings.pushToken.isEmpty ? "None" : settings.pushToken)\nApp Group: \(SharedTaskStore.canAccessAppGroup() ? "Available" : "Unavailable")")
                                    .font(.system(.caption, design: .monospaced))
                                    .foregroundStyle(.antirotTextSecondary)
                            }
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(12)
                        }
                        .frame(height: 150)
                        .background(Color.black.opacity(0.4))
                        .clipShape(RoundedRectangle(cornerRadius: 10))
                        .overlay(
                            RoundedRectangle(cornerRadius: 10)
                                .strokeBorder(Color.antirotBorder, lineWidth: 1)
                        )

                        if alarmCenter.lastErrorDetails != nil {
                            Button {
                                alarmCenter.lastErrorDetails = nil
                            } label: {
                                HStack {
                                    Image(systemName: "trash")
                                    Text("Clear Error Log")
                                }
                                .frame(maxWidth: .infinity)
                            }
                            .buttonStyle(AntirotDestructiveButtonStyle())
                        }
                    }
                    .transition(.opacity.combined(with: .move(edge: .top)))
                }
            }
            .glassCard()
        }
    }

    // MARK: - Status Toast

    @ViewBuilder
    private var statusToast: some View {
        if !alarmCenter.lastMessage.isEmpty {
            HStack(spacing: 10) {
                Image(systemName: "info.circle")
                    .font(.subheadline)
                    .foregroundStyle(.antirotAccentOrange)
                Text(alarmCenter.lastMessage)
                    .font(.caption)
                    .foregroundStyle(.antirotTextSecondary)
                    .lineLimit(3)
                Spacer()
            }
            .glassCard(cornerRadius: 12, padding: 14)
        }
    }

    // MARK: - Helpers

    private var notificationStatusColor: Color {
        switch alarmCenter.notificationStatus {
        case .authorized:
            return .antirotSuccess
        case .provisional:
            return .antirotAccentAmber
        default:
            return .antirotAccentRed
        }
    }

    private var screenTimeStatusColor: Color {
        if screenTimeMessage.contains("authorized") {
            return .antirotSuccess
        } else if screenTimeMessage == "Not requested" {
            return .antirotAccentAmber
        } else {
            return .antirotAccentRed
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
        .environmentObject(SettingsStore())
        .environmentObject(AlarmCenter())
}
