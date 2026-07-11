import SwiftUI
import UIKit

struct MemoryFilesView: View {
    @EnvironmentObject private var settings: SettingsStore

    @State private var selectedKey = DiagnosticsReporter.memoryKeys.first ?? "tasks"
    @State private var content = "Select a markdown file."
    @State private var statusText = "Ready"
    @State private var isLoading = false

    private var client: APIClient {
        APIClient(baseURL: settings.baseURL, apiToken: settings.apiToken, userId: settings.userId)
    }

    var body: some View {
        VStack(spacing: 14) {
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 8) {
                    ForEach(DiagnosticsReporter.memoryKeys, id: \.self) { key in
                        Button {
                            selectedKey = key
                            Task { await loadSelectedFile() }
                        } label: {
                            Text("\(key).md")
                                .font(.caption.weight(.semibold))
                                .foregroundStyle(selectedKey == key ? .white : .arTextSecondary)
                                .padding(.horizontal, 12)
                                .frame(minHeight: 44)
                                .background(
                                    RoundedRectangle(cornerRadius: 4, style: .continuous)
                                        .fill(
                                            selectedKey == key
                                                ? Color.arAccent.opacity(0.82)
                                                : Color.arElevated.opacity(0.58)
                                        )
                                )
                                .overlay(RoundedRectangle(cornerRadius: 4).stroke(Color.arBorder, lineWidth: 1))
                        }
                        .buttonStyle(.plain)
                    }
                }
                .padding(.horizontal, 20)
            }

            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text("\(selectedKey).md")
                        .font(.headline)
                        .foregroundStyle(.arTextPrimary)
                    Text(statusText)
                        .font(.caption)
                        .foregroundStyle(.arTextSecondary)
                }
                Spacer()
                Button {
                    Task { await loadSelectedFile() }
                } label: {
                    Image(systemName: "arrow.clockwise")
                        .frame(width: 44, height: 44)
                        .background(Color.arElevated, in: RoundedRectangle(cornerRadius: 4))
                }
                .buttonStyle(.plain)
                .disabled(isLoading)

                Button {
                    UIPasteboard.general.string = content
                    statusText = "Copied \(selectedKey).md"
                } label: {
                    Image(systemName: "doc.on.doc")
                        .frame(width: 44, height: 44)
                        .background(Color.arElevated, in: RoundedRectangle(cornerRadius: 4))
                }
                .buttonStyle(.plain)
            }
            .padding(.horizontal, 20)

            ScrollView(.vertical, showsIndicators: true) {
                Text(content)
                    .font(.system(size: 13, design: .monospaced))
                    .foregroundStyle(.arTextPrimary)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .textSelection(.enabled)
                    .padding(14)
            }
            .background(Color.arSurface, in: RoundedRectangle(cornerRadius: 4))
            .overlay(RoundedRectangle(cornerRadius: 4).stroke(Color.arBorder, lineWidth: 1))
            .padding(.horizontal, 20)
        }
        .padding(.vertical, 18)
        .background(CinematicBackdrop())
        .navigationTitle("Markdown Files")
        .navigationBarTitleDisplayMode(.inline)
        .task {
            await loadSelectedFile()
        }
    }

    private func loadSelectedFile() async {
        isLoading = true
        statusText = "Loading..."
        do {
            let memory = try await client.fetchMemory(key: selectedKey)
            content = memory.content.isEmpty ? "# Empty\n" : memory.content
            statusText = "Loaded \(selectedKey).md"
        } catch {
            content = "Failed to load \(selectedKey).md\n\n\(error.localizedDescription)"
            statusText = "Load failed"
        }
        isLoading = false
    }
}

#Preview {
    NavigationStack {
        MemoryFilesView()
            .environmentObject(SettingsStore())
    }
}
