import SwiftUI

struct LoginView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter

    @State private var appeared = false
    @State private var showFullError = false

    var body: some View {
        ZStack {
            Color.arBg.ignoresSafeArea()

            VStack(spacing: 0) {
                Spacer()

                // Wordmark
                HStack(spacing: 0) {
                    Text("Anti")
                        .foregroundStyle(.arTextPrimary)
                    Text("rot")
                        .foregroundStyle(.arAccent)
                }
                .font(.system(size: 38, weight: .bold, design: .rounded))
                .tracking(2)

                Spacer().frame(height: 12)

                // Tagline
                Text("Behavioral operating system")
                    .font(.caption)
                    .foregroundStyle(.arTextMuted)

                Spacer().frame(height: 40)

                // Sign-in button
                Button {
                    Task { await signInWithGoogle() }
                } label: {
                    Text("Continue with Google")
                }
                .buttonStyle(AntirotAccentButtonStyle(fullWidth: true))

                Spacer().frame(height: 20)

                // Status message
                if !alarmCenter.lastMessage.isEmpty {
                    Text(alarmCenter.lastMessage)
                        .font(.footnote)
                        .foregroundStyle(.arTextMuted)
                        .multilineTextAlignment(.center)
                        .transition(.opacity)
                }

                // Error details button
                if alarmCenter.lastErrorDetails != nil {
                    Button("Show full error") {
                        showFullError = true
                    }
                    .buttonStyle(AntirotGhostButtonStyle())
                    .padding(.top, 10)
                    .transition(.opacity)
                }

                Spacer()
            }
            .padding(.horizontal, 24)
            .opacity(appeared ? 1 : 0)
            .animation(.easeOut(duration: 0.5), value: appeared)
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
