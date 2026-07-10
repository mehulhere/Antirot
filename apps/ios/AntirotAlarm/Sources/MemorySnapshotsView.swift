import SwiftUI

struct MemorySnapshotsView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var coach: CoachViewModel

    @State private var snapshots: [MemorySnapshotSummary] = []
    @State private var statusText = "Ready"
    @State private var isLoading = false
    @State private var restoringSnapshotId: String?

    private var client: APIClient {
        APIClient(baseURL: settings.baseURL, apiToken: settings.apiToken, userId: settings.userId)
    }

    var body: some View {
        ScrollView(.vertical, showsIndicators: true) {
            VStack(alignment: .leading, spacing: 16) {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Memory Snapshots")
                        .font(.title3.weight(.bold))
                        .foregroundStyle(.arTextPrimary)
                    Text("Save or restore the coach markdown files and runtime state. Only the last 10 snapshots are kept.")
                        .font(.caption)
                        .foregroundStyle(.arTextSecondary)
                }

                Button {
                    Task { await saveSnapshot() }
                } label: {
                    Label("Save Snapshot Now", systemImage: "tray.and.arrow.down")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(AntirotAccentButtonStyle(fullWidth: true))
                .disabled(isLoading)

                HStack {
                    Text(statusText)
                        .font(.caption)
                        .foregroundStyle(.arTextSecondary)
                    Spacer()
                    Button {
                        Task { await loadSnapshots() }
                    } label: {
                        Image(systemName: "arrow.clockwise")
                            .frame(width: 44, height: 44)
                            .background(Circle().fill(Color.white.opacity(0.07)))
                    }
                    .buttonStyle(.plain)
                    .disabled(isLoading)
                }

                LazyVStack(spacing: 12) {
                    if snapshots.isEmpty {
                        Text("No snapshots yet.")
                            .font(.subheadline)
                            .foregroundStyle(.arTextMuted)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .minimalCard(cornerRadius: 20, padding: 14)
                    } else {
                        ForEach(snapshots) { snapshot in
                            snapshotCard(snapshot)
                        }
                    }
                }
            }
            .padding(20)
        }
        .background(CinematicBackdrop())
        .navigationTitle("Snapshots")
        .navigationBarTitleDisplayMode(.inline)
        .task {
            await loadSnapshots()
        }
    }

    private func snapshotCard(_ snapshot: MemorySnapshotSummary) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .top, spacing: 10) {
                Image(systemName: "clock.arrow.circlepath")
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(.arAccent)
                    .frame(width: 22)

                VStack(alignment: .leading, spacing: 4) {
                    Text(snapshot.title)
                        .font(.subheadline.weight(.semibold))
                        .foregroundStyle(.arTextPrimary)
                    Text(snapshotSubtitle(snapshot))
                        .font(.caption2)
                        .foregroundStyle(.arTextMuted)
                        .lineLimit(3)
                }

                Spacer()
            }

            HStack {
                Text("\(snapshot.memoryKeys.count) files")
                    .font(.caption2.weight(.medium))
                    .foregroundStyle(.arTextSecondary)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 5)
                    .background(Color.arElevated)
                    .clipShape(Capsule(style: .continuous))

                if let state = snapshot.runtimeState?.state {
                    Text(state)
                        .font(.caption2.weight(.medium))
                        .foregroundStyle(.arTextSecondary)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 5)
                        .background(Color.arElevated)
                        .clipShape(Capsule(style: .continuous))
                }

                Spacer()

                Button {
                    Task { await restoreSnapshot(snapshot) }
                } label: {
                    if restoringSnapshotId == snapshot.id {
                        ProgressView()
                            .tint(.white)
                            .frame(width: 72)
                    } else {
                        Text("Restore")
                            .font(.caption.weight(.bold))
                            .frame(width: 72)
                    }
                }
                .buttonStyle(AntirotGhostButtonStyle())
                .disabled(isLoading || restoringSnapshotId != nil)
            }
        }
        .minimalCard(cornerRadius: 20, padding: 14)
    }

    private func snapshotSubtitle(_ snapshot: MemorySnapshotSummary) -> String {
        let created = snapshot.createdAt.formatted(date: .abbreviated, time: .shortened)
        let reason = snapshot.reason.replacingOccurrences(of: "_", with: " ")
        return "\(created) - \(reason)"
    }

    private func saveSnapshot() async {
        isLoading = true
        statusText = "Saving snapshot..."
        do {
            let response = try await client.createMemorySnapshot(CreateMemorySnapshotRequest(
                deviceId: settings.deviceId,
                title: "iOS developer snapshot",
                reason: "manual_ios_developer"
            ))
            statusText = "Saved. Keeping \(response.retainedCount)/\(response.retentionLimit)."
            coach.recordDiagnosticEvent(
                kind: "memory_snapshot.saved",
                summary: "Manual memory snapshot saved.",
                detail: response.snapshot.id
            )
            await loadSnapshots()
        } catch {
            statusText = "Save failed: \(error.localizedDescription)"
            coach.recordDiagnosticEvent(
                kind: "memory_snapshot.save_failed",
                summary: "Manual memory snapshot failed.",
                detail: error.localizedDescription
            )
        }
        isLoading = false
    }

    private func loadSnapshots() async {
        isLoading = true
        statusText = "Loading snapshots..."
        do {
            let response = try await client.fetchMemorySnapshots()
            snapshots = response.snapshots
            statusText = "Loaded \(response.snapshots.count)/\(response.retentionLimit) snapshots."
        } catch {
            statusText = "Load failed: \(error.localizedDescription)"
        }
        isLoading = false
    }

    private func restoreSnapshot(_ snapshot: MemorySnapshotSummary) async {
        restoringSnapshotId = snapshot.id
        statusText = "Restoring snapshot..."
        do {
            let response = try await client.restoreMemorySnapshot(id: snapshot.id)
            statusText = "Restored \(response.restoredMemoryKeys.count) files."
            coach.recordDiagnosticEvent(
                kind: "memory_snapshot.restored",
                summary: "Memory snapshot restored.",
                detail: response.snapshot.id
            )
            await coach.refreshRuntimeState(client: client, deviceId: settings.deviceId)
            await loadSnapshots()
        } catch {
            statusText = "Restore failed: \(error.localizedDescription)"
            coach.recordDiagnosticEvent(
                kind: "memory_snapshot.restore_failed",
                summary: "Memory snapshot restore failed.",
                detail: error.localizedDescription
            )
        }
        restoringSnapshotId = nil
    }
}

#Preview {
    NavigationStack {
        MemorySnapshotsView()
            .environmentObject(SettingsStore())
            .environmentObject(CoachViewModel())
    }
}
