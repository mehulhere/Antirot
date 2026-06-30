import SwiftUI

// MARK: - Home (Coach Room)

/// The cinematic coach room: a full-screen stylized coach, one dominant
/// circular action button per runtime state, optional quiet secondary
/// actions, and a draggable glass chat sheet pinned to the bottom. There is
/// no dashboard clutter here — stats, plan, alarms, and settings stay hidden
/// behind the small top-right menu in `MainTabView`.
struct HomeView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @EnvironmentObject private var coach: CoachViewModel

    @State private var onboardingName = ""
    @State private var showNamePrompt = false
    @State private var sheetHeight: CGFloat = 108

    private var client: APIClient {
        APIClient(baseURL: settings.baseURL, apiToken: settings.apiToken, userId: settings.userId)
    }

    var body: some View {
        ZStack(alignment: .bottom) {
            CoachStage(emotion: coach.coachEmotion, isThinking: coach.isSending)
                .ignoresSafeArea()

            actionStack
                .padding(.bottom, sheetHeight + 22)
                .padding(.horizontal, 24)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)

            GlassSheet(
                height: $sheetHeight,
                messages: coach.messages,
                draft: $coach.draft,
                isRecording: coach.isRecording,
                isSending: coach.isSending,
                statusText: coach.statusText,
                latestOneLiner: latestOneLiner,
                onMic: { Task { await micTapped() } },
                onSend: { Task { await sendTapped() } }
            )
        }
        .confettiOverlay(trigger: $coach.showConfetti)
        .background(Color.arBg.ignoresSafeArea())
        .task {
            onboardingName = settings.onboardingName
            await alarmCenter.pollPendingAlarms()
            await coach.refreshRuntimeState(client: client, deviceId: settings.deviceId)
            presentNamePromptIfNeeded()
        }
        .onChange(of: coach.runtimeState) { _, _ in
            presentNamePromptIfNeeded()
        }
        .alert("Your name", isPresented: $showNamePrompt) {
            TextField("Name", text: $onboardingName)
            Button("Continue") {
                Task { await sendNameOnboarding() }
            }
            .disabled(onboardingName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
        } message: {
            Text("The rest can be handled by voice.")
        }
    }
}
// MARK: - Action Stack

private extension HomeView {
    var actionStack: some View {
        let set = CoachStateActions.actions(for: coach.runtimeState)
        return VStack(spacing: 14) {
            Spacer()
            if !set.secondary.isEmpty {
                HStack(spacing: 10) {
                    ForEach(set.secondary) { button in
                        SecondaryActionButton(title: button.title, systemImage: button.systemImage) {
                            Task { await sendStateButton(button) }
                        }
                    }
                }
            }
            PrimaryActionButton(
                title: set.primary.title,
                systemImage: set.primary.systemImage,
                isBusy: coach.isSending
            ) {
                Task { await sendStateButton(set.primary) }
            }
        }
    }

    var latestOneLiner: String {
        if let last = coach.messages.last(where: { $0.role == .coach }), !last.text.isEmpty {
            return String(last.text.prefix(120)).replacingOccurrences(of: "\n", with: " ")
        }
        return coach.coachEmotion.ambientOneLiner
    }

    func sendStateButton(_ button: CoachStateButton) async {
        if button.triggersConfetti {
            coach.showConfetti = true
        }
        await coach.send(button.message, client: client)
        await coach.refreshRuntimeState(client: client, deviceId: settings.deviceId)
    }

    func micTapped() async {
        await coach.toggleVoice(client: client)
        await coach.refreshRuntimeState(client: client, deviceId: settings.deviceId)
    }

    func sendTapped() async {
        await coach.sendDraft(client: client)
        await coach.refreshRuntimeState(client: client, deviceId: settings.deviceId)
    }

    func presentNamePromptIfNeeded() {
        guard !settings.onboardingNameSent else { return }
        showNamePrompt = coach.runtimeState == "onboarding" || coach.runtimeState == "unknown"
    }

    func sendNameOnboarding() async {
        let name = onboardingName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty else {
            showNamePrompt = true
            return
        }
        settings.onboardingName = name
        settings.onboardingNameSent = true
        showNamePrompt = false
        await coach.send(onboardingMessage(name: name), visibleText: "", client: client)
        await coach.refreshRuntimeState(client: client, deviceId: settings.deviceId)
    }

    func onboardingMessage(name: String) -> String {
        let timezone = TimeZone.current.identifier
        return [
            "The user just shared their name during onboarding. Return the deterministic Antirot first onboarding message exactly.",
            "Silent client context is available below for scheduling only.",
            "Do not mention timezone, profile setup, profile updates, saved fields, or that anything was saved unless the user explicitly asks.",
            "First onboarding message: I'm Antirot. I've coached plenty of people like you: smart, intense, full of plans, and somehow still one bad hour away from drifting off the thing they claim matters.\n\nSo let's see what you've got. I need to build your profile. Give me a gist of your long-term and short-term goals. You can update this later as well. Because obviously, ambition is not a gift everyone has.\n\nTell me what your day looks like and what you're planning to get done today.",
            "Name: \(name)",
            "Silent device timezone: \(timezone)"
        ].joined(separator: "\n")
    }
}
// MARK: - Preview

#Preview {
    HomeView()
        .environmentObject(SettingsStore())
        .environmentObject(AlarmCenter())
        .environmentObject(CoachViewModel())
}
