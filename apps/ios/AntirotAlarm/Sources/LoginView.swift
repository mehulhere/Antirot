import SwiftUI

struct LoginView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    @State private var appeared = false
    @State private var showFullError = false

    var body: some View {
        ZStack {
            CinematicBackdrop()

            VStack(alignment: .leading, spacing: 24) {
                Spacer()

                VStack(alignment: .leading, spacing: 18) {
                    Text("ANTIROT / OPERATING SYSTEM")
                        .font(.system(size: 10, weight: .bold, design: .monospaced))
                        .tracking(1.4)
                        .foregroundStyle(.arAccent)

                    Text("Stop negotiating\nwith yourself.")
                        .font(.system(size: 46, weight: .semibold, design: .serif))
                        .foregroundStyle(.arTextPrimary)
                        .fixedSize(horizontal: false, vertical: true)

                    Text("Behavioral operating system")
                        .font(.subheadline)
                        .foregroundStyle(.arTextSecondary)

                    SectionDivider()

                    Button {
                        Task { await signInWithGoogle() }
                    } label: {
                        Label("Continue with Google", systemImage: "arrow.right")
                    }
                    .buttonStyle(AntirotAccentButtonStyle(fullWidth: true))

                    if !alarmCenter.lastMessage.isEmpty {
                        Text(alarmCenter.lastMessage)
                            .font(.footnote)
                            .foregroundStyle(.arTextSecondary)
                            .multilineTextAlignment(.center)
                            .transition(.opacity)
                    }

                    if alarmCenter.lastErrorDetails != nil {
                        Button("Show full error") {
                            showFullError = true
                        }
                        .buttonStyle(AntirotGhostButtonStyle())
                        .transition(.opacity)
                    }
                }

                Spacer()
            }
            .padding(.horizontal, 24)
            .padding(.vertical, 32)
            .opacity(appeared ? 1 : 0)
            .offset(y: appeared || reduceMotion ? 0 : 12)
            .animation(reduceMotion ? .easeOut(duration: 0.1) : .easeOut(duration: 0.5), value: appeared)
        }
        .alert("Full Error", isPresented: $showFullError) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(alarmCenter.lastErrorDetails ?? "No error details.")
        }
        .onAppear {
            appeared = true
        }
    }

    // MARK: - Actions

    private func signInWithGoogle() async {
        do {
            let response = try await GoogleAuthCenter.signIn(settings: settings)
            alarmCenter.lastMessage = "Signed in as \(response.email)"
            alarmCenter.lastErrorDetails = nil
            await alarmCenter.registerDevice()
        } catch {
            let message = shortErrorMessage(error)
            settings.statusMessage = message
            alarmCenter.lastMessage = message
            alarmCenter.lastErrorDetails = fullErrorDetails(error)
        }
    }

    private func shortErrorMessage(_ error: Error) -> String {
        if let apiError = error as? APIClient.APIError {
            return apiError.shortMessage
        }
        if let googleAuthError = error as? GoogleAuthError {
            return googleAuthError.shortMessage
        }
        return "Sign-in failed"
    }

    private func fullErrorDetails(_ error: Error) -> String {
        var parts = [error.localizedDescription]
        if let localizedError = error as? LocalizedError,
           let suggestion = localizedError.recoverySuggestion,
           !suggestion.isEmpty {
            parts.append("Suggestion: \(suggestion)")
        }
        let nsError = error as NSError
        parts.append("Domain: \(nsError.domain)")
        parts.append("Code: \(nsError.code)")
        return parts.joined(separator: "\n\n")
    }
}

#Preview {
    LoginView()
        .environmentObject(SettingsStore())
        .environmentObject(AlarmCenter())
}
