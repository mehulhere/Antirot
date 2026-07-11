import SwiftUI
import UIKit

struct SettingsView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @EnvironmentObject private var coach: CoachViewModel
    @State private var screenTimeMessage = "Not requested"
    @State private var showDeveloperSettings = false
    @State private var showFullError = false
    @State private var diagnosticsStatus = "Copies the last 3 exchanges and changed files."
    @State private var memorySnapshotCache: [String: String] = [:]
    @State private var diagnosticsPreview = ""
    @State private var showDiagnosticsPreview = false

    private var client: APIClient {
        APIClient(baseURL: settings.baseURL, apiToken: settings.apiToken, userId: settings.userId)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 24) {
            HStack(spacing: 12) {
                Image(systemName: "person.crop.circle.fill")
                    .font(.system(size: 30))
                    .foregroundStyle(.arTextPrimary)
                    .frame(width: 48, height: 48)

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
            .padding(.vertical, 14)
            .overlay(alignment: .top) { Rectangle().fill(Color.arBorder).frame(height: 1) }
            .overlay(alignment: .bottom) { Rectangle().fill(Color.arBorder).frame(height: 1) }

            VStack(alignment: .leading, spacing: 10) {
                Text("PERMISSIONS")
                    .font(.system(size: 11, weight: .bold, design: .monospaced))
                    .tracking(1.2)
                    .foregroundStyle(.arAccent)

                HStack(spacing: 8) {
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
                }
            }
            .padding(.vertical, 4)

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
            .overlay(alignment: .top) { Rectangle().fill(Color.arBorder).frame(height: 1) }
            .overlay(alignment: .bottom) { Rectangle().fill(Color.arBorder).frame(height: 1) }

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
                        NavigationLink {
                            MemoryFilesView()
                                .environmentObject(settings)
                        } label: {
                            developerRow(
                                title: "Memory Files",
                                subtitle: "View backend markdown files",
                                icon: "doc.text.magnifyingglass",
                                trailing: "Open"
                            )
                        }
                        .buttonStyle(.plain)

                        NavigationLink {
                            MemorySnapshotsView()
                                .environmentObject(settings)
                                .environmentObject(coach)
                        } label: {
                            developerRow(
                                title: "Memory Snapshots",
                                subtitle: "Save and restore the coach state",
                                icon: "clock.arrow.circlepath",
                                trailing: "Open"
                            )
                        }
                        .buttonStyle(.plain)

                        Toggle(isOn: $settings.autoSnapshotOnStop) {
                            VStack(alignment: .leading, spacing: 2) {
                                Text("Snapshot on Stop")
                                    .font(.subheadline.weight(.semibold))
                                    .foregroundStyle(.arTextPrimary)
                                Text("Beta safety net. Keeps only the last 10 snapshots.")
                                    .font(.caption2)
                                    .foregroundStyle(.arTextMuted)
                            }
                        }
                        .tint(.arAccent)
                        .padding(.vertical, 12)

                        Button {
                            Task { await copyDiagnostics() }
                        } label: {
                            developerRow(
                                title: "Copy Diagnostics",
                                subtitle: diagnosticsStatus,
                                icon: "stethoscope",
                                trailing: "Copy"
                            )
                        }
                        .buttonStyle(.plain)

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
                                .background(Color.arDeepBg.opacity(0.56))
                                .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))
                                .overlay(RoundedRectangle(cornerRadius: 4).stroke(Color.arBorder, lineWidth: 1))
                        }

                        VStack(alignment: .leading, spacing: 4) {
                            Text("API Token")
                                .font(.caption)
                                .foregroundStyle(.arTextMuted)
                            SecureField("API token", text: $settings.apiToken)
                                .font(.subheadline)
                                .foregroundStyle(.arTextPrimary)
                                .padding(10)
                                .background(Color.arDeepBg.opacity(0.56))
                                .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))
                                .overlay(RoundedRectangle(cornerRadius: 4).stroke(Color.arBorder, lineWidth: 1))
                        }
                    }
                    .padding(14)
                    .transition(.opacity.combined(with: .move(edge: .top)))
                }
            }
            .overlay(alignment: .top) { Rectangle().fill(Color.arBorder).frame(height: 1) }
            .overlay(alignment: .bottom) { Rectangle().fill(Color.arBorder).frame(height: 1) }
        }
        .alert("Full Error", isPresented: $showFullError) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(alarmCenter.lastErrorDetails ?? "No error details.")
        }
        .sheet(isPresented: $showDiagnosticsPreview) {
            NavigationStack {
                ScrollView(.vertical, showsIndicators: true) {
                    Text(diagnosticsPreview)
                        .font(.system(size: 13, design: .monospaced))
                        .foregroundStyle(.arTextPrimary)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .textSelection(.enabled)
                        .padding(18)
                }
                .background(Color.arBg)
                .navigationTitle("Diagnostics")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .topBarTrailing) {
                        Button("Copy") {
                            UIPasteboard.general.string = diagnosticsPreview
                            diagnosticsStatus = "Copied diagnostics again."
                        }
                    }
                }
            }
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
            .frame(maxWidth: .infinity, minHeight: 54)
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

    private func developerRow(title: String, subtitle: String, icon: String, trailing: String) -> some View {
        HStack(spacing: 12) {
            Image(systemName: icon)
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(.arTextSecondary)
                .frame(width: 22)
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(.arTextPrimary)
                Text(subtitle)
                    .font(.caption2)
                    .foregroundStyle(.arTextMuted)
                    .lineLimit(2)
            }
            Spacer()
            Text(trailing)
                .font(.caption.weight(.semibold))
                .foregroundStyle(.arTextSecondary)
        }
        .padding(12)
        .overlay(alignment: .bottom) {
            Rectangle().fill(Color.arBorder).frame(height: 1)
        }
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

    private func copyDiagnostics() async {
        diagnosticsStatus = "Loading markdown snapshots..."
        let now = Date()
        let events = DiagnosticsReporter.reportEvents(from: coach.diagnosticEvents, now: now)
        let snapshots = await loadDiagnosticMemorySnapshots()
        let reportMarkdown = DiagnosticsReporter.buildMarkdown(
            messages: coach.messages,
            events: events,
            memorySnapshots: snapshots,
            runtimeState: coach.runtimeState,
            statusText: coach.statusText,
            deviceId: settings.deviceId,
            now: now
        )

        UIPasteboard.general.string = reportMarkdown
        diagnosticsPreview = reportMarkdown
        showDiagnosticsPreview = true
        diagnosticsStatus = "Copied. Saving report..."

        let windowStart = events.map(\.at).min() ?? now.addingTimeInterval(-30 * 60)
        do {
            let response = try await client.createReport(CreateReportRequest(
                deviceId: settings.deviceId,
                title: "iOS diagnostics report",
                windowStart: windowStart,
                windowEnd: now,
                reportMarkdown: reportMarkdown,
                events: events
            ))
            diagnosticsStatus = "Copied and saved: \(String(response.reportId.prefix(8)))"
            coach.recordDiagnosticEvent(
                kind: "diagnostics.saved",
                summary: "Diagnostics copied and saved.",
                detail: response.reportId
            )
        } catch {
            diagnosticsStatus = "Copied. Save failed."
            coach.recordDiagnosticEvent(
                kind: "diagnostics.save_failed",
                summary: "Diagnostics copied but backend save failed.",
                detail: error.localizedDescription
            )
        }
    }

    private func loadDiagnosticMemorySnapshots() async -> [DiagnosticMemorySnapshot] {
        var rows: [DiagnosticMemorySnapshot] = []

        for key in DiagnosticsReporter.memoryKeys {
            let previous = memorySnapshotCache[key]
            do {
                let response = try await client.fetchMemory(key: key)
                rows.append(DiagnosticMemorySnapshot(
                    key: key,
                    previous: previous,
                    content: response.content,
                    error: nil
                ))
                memorySnapshotCache[key] = response.content
            } catch {
                rows.append(DiagnosticMemorySnapshot(
                    key: key,
                    previous: previous,
                    content: nil,
                    error: error.localizedDescription
                ))
            }
        }

        return rows
    }
}

#Preview {
    SettingsView()
        .padding(.horizontal, 24)
        .background(Color.arBg)
        .environmentObject(SettingsStore())
        .environmentObject(AlarmCenter())
        .environmentObject(CoachViewModel())
}
