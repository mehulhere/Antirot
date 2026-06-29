import SwiftUI

struct LoginView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter

    @State private var appeared = false
    @State private var showFullError = false

    var body: some View {
        ZStack {
            AmbientBackground()

            VStack(spacing: 0) {
                Spacer()

                // Logo
                Image("favicon")
                    .resizable()
                    .frame(width: 96, height: 96)
                    .clipShape(RoundedRectangle(cornerRadius: 20))
                    .accentGlow(radius: 30)
                    .opacity(appeared ? 1 : 0)
                    .offset(y: appeared ? 0 : 18)
                    .animation(.easeOut(duration: 0.7).delay(0.1), value: appeared)
                    .accessibilityHidden(true)

                Spacer().frame(height: 24)

                // App name with gradient "rot"
                HStack(spacing: 0) {
                    Text("Anti")
                        .font(.system(size: 34, weight: .bold, design: .default))
                        .foregroundStyle(.antirotTextPrimary)

                    Text("rot")
                        .font(.system(size: 34, weight: .bold, design: .default))
                        .foregroundStyle(LinearGradient.antirotAccent)
                }
                .opacity(appeared ? 1 : 0)
                .offset(y: appeared ? 0 : 14)
                .animation(.easeOut(duration: 0.7).delay(0.25), value: appeared)

                Spacer().frame(height: 8)

                // Tagline
                Text("Your behavioral operating system")
                    .font(.subheadline)
                    .foregroundStyle(.antirotTextSecondary)
                    .opacity(appeared ? 1 : 0)
                    .offset(y: appeared ? 0 : 10)
                    .animation(.easeOut(duration: 0.7).delay(0.4), value: appeared)

                Spacer().frame(height: 48)

                // Sign-in button
                Button {
                    Task { await signInWithGoogle() }
                } label: {
                    HStack(spacing: 10) {
                        Image(systemName: "person.crop.circle.fill")
                            .font(.title3)
                        Text("Continue with Google")
                    }
                }
                .buttonStyle(AntirotAccentButtonStyle(fullWidth: true))
                .opacity(appeared ? 1 : 0)
                .offset(y: appeared ? 0 : 12)
                .animation(.easeOut(duration: 0.7).delay(0.55), value: appeared)

                Spacer().frame(height: 20)

                // Status message
                if !alarmCenter.lastMessage.isEmpty {
                    Text(alarmCenter.lastMessage)
                        .font(.footnote)
                        .foregroundStyle(.antirotTextMuted)
                        .multilineTextAlignment(.center)
                        .transition(.opacity.combined(with: .move(edge: .bottom)))
                        .animation(.easeOut(duration: 0.3), value: alarmCenter.lastMessage)
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

                // Footer
                Text("Built for humans who want to stay sharp.")
                    .font(.caption2)
                    .foregroundStyle(.antirotTextMuted.opacity(0.6))
                    .opacity(appeared ? 1 : 0)
                    .animation(.easeOut(duration: 0.8).delay(0.7), value: appeared)
                    .padding(.bottom, 16)
            }
            .padding(.horizontal, 32)
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
