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

            Circle()
                .fill(Color.arAccent.opacity(0.08))
                .frame(width: 260, height: 260)
                .blur(radius: 100)
                .offset(x: -150, y: 300)

            VStack(spacing: 24) {
                Spacer()

                VStack(spacing: 18) {
                    Image(systemName: "bolt.fill")
                        .font(.system(size: 24, weight: .bold))
                        .foregroundStyle(.white)
                        .frame(width: 58, height: 58)
                        .background(Circle().fill(LinearGradient.antirotAccent))
                        .shadow(color: Color.arAccent.opacity(0.30), radius: 22, y: 8)

                    HStack(spacing: 0) {
                        Text("Anti")
                            .foregroundStyle(.arTextPrimary)
                        Text("rot")
                            .foregroundStyle(.arAccent)
                    }
                    .font(.system(size: 38, weight: .bold, design: .rounded))
                    .tracking(2)

                    VStack(spacing: 5) {
                        Text("BEHAVIORAL OPERATING SYSTEM")
                            .font(.caption2.weight(.bold))
                            .tracking(1.25)
                            .foregroundStyle(.arTextSecondary)
                        Text("Standards up. Drift down.")
                            .font(.subheadline.weight(.semibold))
                            .foregroundStyle(.arTextPrimary)
                    }

                    SectionDivider()

                    Button {
                        Task { await signInWithGoogle() }
                    } label: {
                        Label("Continue with Google", systemImage: "arrow.right")
                    }
                    .buttonStyle(AntirotAccentButtonStyle(fullWidth: true))

                    Text("Your coach state stays synced to your Antirot account.")
                        .font(.caption)
                        .foregroundStyle(.arTextMuted)
                        .multilineTextAlignment(.center)

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
                .padding(24)
                .smokedGlass(cornerRadius: 30, tint: .arSurface)

                Spacer()
            }
            .padding(.horizontal, 20)
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
