import AVFoundation
import SwiftUI

struct HomeView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @EnvironmentObject private var coach: CoachViewModel
    @FocusState private var draftFocused: Bool
    @State private var onboardingName = ""
    @State private var showNamePrompt = false
    @State private var quickActionRefreshDate = Date()

    private var client: APIClient {
        APIClient(baseURL: settings.baseURL, apiToken: settings.apiToken, userId: settings.userId)
    }

    private var hasDraft: Bool {
        !coach.draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    var body: some View {
        VStack(spacing: 0) {
            statusBar
            conversation
            composer
        }
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
        .onReceive(Timer.publish(every: 60, on: .main, in: .common).autoconnect()) { date in
            quickActionRefreshDate = date
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

    // MARK: - Zone 1: Status Bar

    private var statusBar: some View {
        HStack {
            StatePill(
                label: coach.runtimeState,
                isActive: coach.runtimeState.lowercased() == "working"
            )
            Spacer()
            StatusDot(color: settings.registered ? .arSuccess : .arDanger)
        }
        .padding(.horizontal, 24)
        .frame(height: 44)
    }

    // MARK: - Zone 2: Conversation

    private var conversation: some View {
        ScrollViewReader { proxy in
            ScrollView(.vertical, showsIndicators: false) {
                LazyVStack(spacing: 10) {
                    ForEach(coach.messages) { message in
                        CoachBubble(message: message)
                            .id(message.id)
                    }

                    if coach.isSending {
                        HStack(spacing: 8) {
                            ProgressView()
                                .tint(.arTextMuted)
                            Text(coach.statusText)
                                .font(.caption)
                                .foregroundStyle(.arTextMuted)
                            Spacer()
                        }
                        .padding(14)
                        .background(
                            RoundedRectangle(cornerRadius: 16, style: .continuous)
                                .fill(Color.arElevated)
                        )
                    }
                }
                .padding(.horizontal, 20)
                .padding(.top, 8)
                .padding(.bottom, 120)
            }
            .onChange(of: coach.messages.count) { _, _ in
                if let last = coach.messages.last?.id {
                    withAnimation(.easeOut(duration: 0.25)) {
                        proxy.scrollTo(last, anchor: .bottom)
                    }
                }
            }
        }
    }

    // MARK: - Zone 3: Composer

    private var composer: some View {
        let actions = CoachQuickAction.primary(for: coach.runtimeState, at: quickActionRefreshDate)

        return VStack(spacing: 8) {
            SectionDivider()

            if !actions.isEmpty {
                quickActionChips(actions)
            }

            HStack(spacing: 10) {
                micButton
                textField
                if hasDraft {
                    sendButton
                        .transition(.scale.combined(with: .opacity))
                }
            }

            Text(coach.isRecording ? "Listening..." : "Voice is preferred")
                .font(.caption2)
                .foregroundStyle(.arTextMuted)
                .padding(.bottom, 4)
        }
        .padding(.horizontal, 20)
        .padding(.top, 8)
        .padding(.bottom, 8)
        .background(Color.arBg.ignoresSafeArea(.container, edges: .bottom))
        .animation(.spring(duration: 0.3), value: hasDraft)
    }

    private func quickActionChips(_ actions: [CoachQuickAction]) -> some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 8) {
                ForEach(actions) { action in
                    Button {
                        Task {
                            await coach.handleQuickAction(action, client: client)
                            await coach.refreshRuntimeState(client: client, deviceId: settings.deviceId)
                        }
                    } label: {
                        Text(action.title)
                            .font(.caption.weight(.medium))
                            .foregroundStyle(.arTextSecondary)
                            .padding(.horizontal, 12)
                            .padding(.vertical, 7)
                            .background(
                                RoundedRectangle(cornerRadius: 10, style: .continuous)
                                    .fill(Color.arSurface)
                            )
                    }
                    .buttonStyle(.plain)
                }
            }
        }
    }

    private var micButton: some View {
        Button {
            Task {
                await coach.toggleVoice(client: client)
                await coach.refreshRuntimeState(client: client, deviceId: settings.deviceId)
            }
        } label: {
            Image(systemName: coach.isRecording ? "stop.fill" : "mic.fill")
                .font(.title3.weight(.semibold))
                .foregroundStyle(.white)
                .frame(width: 52, height: 52)
                .background(
                    Circle()
                        .fill(coach.isRecording ? Color.arDanger : Color.arAccent)
                )
                .scaleEffect(coach.isRecording ? 1.06 : 1.0)
                .animation(
                    coach.isRecording
                        ? .easeInOut(duration: 0.8).repeatForever(autoreverses: true)
                        : .default,
                    value: coach.isRecording
                )
        }
        .buttonStyle(.plain)
        .disabled(coach.isSending)
    }

    private var textField: some View {
        TextField("Type...", text: $coach.draft, axis: .vertical)
            .lineLimit(1...3)
            .focused($draftFocused)
            .textInputAutocapitalization(.sentences)
            .font(.body)
            .foregroundStyle(.arTextPrimary)
            .padding(.horizontal, 14)
            .padding(.vertical, 10)
            .background(
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .fill(Color.arSurface)
            )
    }

    private var sendButton: some View {
        Button {
            Task {
                await coach.sendDraft(client: client)
                await coach.refreshRuntimeState(client: client, deviceId: settings.deviceId)
            }
        } label: {
            Image(systemName: "arrow.up")
                .font(.subheadline.weight(.bold))
                .foregroundStyle(.white)
                .frame(width: 38, height: 38)
                .background(Circle().fill(Color.arAccent))
        }
        .buttonStyle(.plain)
        .disabled(coach.isSending)
    }

    // MARK: - Helpers

    private func presentNamePromptIfNeeded() {
        guard !settings.onboardingNameSent else { return }
        showNamePrompt = coach.runtimeState == "onboarding" || coach.runtimeState == "unknown"
    }

    private func sendNameOnboarding() async {
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

    private func onboardingMessage(name: String) -> String {
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

// MARK: - Coach Bubble

private struct CoachBubble: View {
    let message: CoachMessage
    @State private var showTimestamp = false
    @State private var player: AVAudioPlayer?

    var body: some View {
        if message.role == .system {
            systemBubble
        } else {
            chatBubble
        }
    }

    private var systemBubble: some View {
        Text(message.text)
            .font(.caption)
            .foregroundStyle(.arTextMuted)
            .multilineTextAlignment(.center)
            .frame(maxWidth: .infinity)
            .padding(.vertical, 4)
    }

    private var chatBubble: some View {
        let isUser = message.role == .user

        return HStack {
            if isUser { Spacer(minLength: 60) }

            VStack(alignment: isUser ? .trailing : .leading, spacing: 4) {
                if let audioURL = message.audioFileURL {
                    Button { playAudio(url: audioURL) } label: {
                        Label("Voice message", systemImage: "play.circle.fill")
                            .font(.body.weight(.medium))
                            .foregroundStyle(.arTextPrimary)
                    }
                    .buttonStyle(.plain)
                } else {
                    Text(message.text)
                        .font(.body)
                        .foregroundStyle(.arTextPrimary)
                        .lineSpacing(4)
                        .fixedSize(horizontal: false, vertical: true)
                }

                if showTimestamp {
                    Text(message.createdAt.formatted(date: .omitted, time: .shortened))
                        .font(.caption2)
                        .foregroundStyle(.arTextMuted)
                        .transition(.opacity)
                }
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 10)
            .background(
                RoundedRectangle(cornerRadius: 16, style: .continuous)
                    .fill(isUser ? Color.arSurface : Color.arElevated)
            )
            .onTapGesture {
                withAnimation(.easeOut(duration: 0.2)) {
                    showTimestamp.toggle()
                }
            }

            if !isUser { Spacer(minLength: 60) }
        }
    }

    private func playAudio(url: URL) {
        do {
            player = try AVAudioPlayer(contentsOf: url)
            player?.prepareToPlay()
            player?.play()
        } catch {
            player = nil
        }
    }
}

// MARK: - Preview

#Preview {
    HomeView()
        .environmentObject(SettingsStore())
        .environmentObject(AlarmCenter())
        .environmentObject(CoachViewModel())
}
