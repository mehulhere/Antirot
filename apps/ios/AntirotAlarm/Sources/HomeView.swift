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

    var body: some View {
        ZStack {
            SiriCoachBackground()

            VStack(spacing: 0) {
                header
                    .padding(.horizontal, 20)
                    .padding(.top, 12)

                ScrollViewReader { proxy in
                    ScrollView(.vertical, showsIndicators: false) {
                        VStack(spacing: 18) {
                            SiriCoachOrb(
                                isListening: coach.isRecording,
                                isThinking: coach.isSending,
                                isSpeaking: coach.isSpeaking,
                                statusText: coach.statusText
                            )
                            .padding(.top, 14)

                            currentTaskStrip
                            quickActions
                            transcript
                            pendingAlarmStrip
                        }
                        .padding(.horizontal, 20)
                        .padding(.bottom, 210)
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

            composer
        }
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

    private var header: some View {
        HStack(spacing: 12) {
            Image("favicon")
                .resizable()
                .frame(width: 36, height: 36)
                .clipShape(RoundedRectangle(cornerRadius: 10))
                .accessibilityHidden(true)

            VStack(alignment: .leading, spacing: 2) {
                Text("Antirot Coach")
                    .font(.headline.bold())
                    .foregroundStyle(.antirotTextPrimary)
                Text(settings.registered ? "Backend connected" : "Offline")
                    .font(.caption)
                    .foregroundStyle(.antirotTextMuted)
            }

            Spacer()

            Button {
                resetLocalConversation()
            } label: {
                Image(systemName: "trash")
                    .font(.subheadline.weight(.bold))
                    .foregroundStyle(.antirotTextPrimary)
                    .frame(width: 38, height: 38)
                    .background(Color.white.opacity(0.08))
                    .clipShape(RoundedRectangle(cornerRadius: 12))
                    .overlay(
                        RoundedRectangle(cornerRadius: 12)
                            .strokeBorder(Color.white.opacity(0.1), lineWidth: 1)
                    )
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Reset conversation")

            StatusDot(color: settings.registered ? .antirotSuccess : .antirotAccentRed)
        }
    }

    private var currentTaskStrip: some View {
        let snapshot = SharedTaskStore.read()

        return VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                Image(systemName: "target")
                    .foregroundStyle(.antirotAccentOrange)
                Text(snapshot.mode.uppercased())
                    .font(.caption2.weight(.bold))
                    .tracking(1)
                    .foregroundStyle(.antirotTextMuted)
                Spacer()
                if let dueAt = snapshot.dueAt {
                    Text(dueAt, style: .relative)
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(.antirotAccentAmber)
                }
            }

            Text(snapshot.title)
                .font(.headline)
                .foregroundStyle(.antirotTextPrimary)
                .lineLimit(2)

            Text(snapshot.subtitle)
                .font(.subheadline)
                .foregroundStyle(.antirotTextSecondary)
                .lineLimit(2)
        }
        .glassCard(cornerRadius: 18, padding: 16)
    }

    private var quickActions: some View {
        let actions = CoachQuickAction.primary(for: coach.runtimeState, at: quickActionRefreshDate)

        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 10) {
                ForEach(actions) { action in
                    Button {
                        Task {
                            await coach.handleQuickAction(action, client: client)
                            await coach.refreshRuntimeState(client: client, deviceId: settings.deviceId)
                        }
                    } label: {
                        HStack(spacing: 8) {
                            Image(systemName: action.systemImage)
                                .font(.caption.weight(.bold))
                            Text(action.title)
                                .font(.caption.weight(.semibold))
                                .lineLimit(1)
                        }
                        .foregroundStyle(.antirotTextPrimary)
                        .padding(.horizontal, 13)
                        .padding(.vertical, 10)
                        .background(Color.white.opacity(0.08))
                        .clipShape(Capsule())
                        .overlay(
                            Capsule()
                                .strokeBorder(Color.white.opacity(0.09), lineWidth: 1)
                        )
                    }
                    .buttonStyle(.plain)
                }

                if actions.isEmpty {
                    Text("No quick actions for this state.")
                        .font(.caption)
                        .foregroundStyle(.antirotTextMuted)
                        .padding(.vertical, 10)
                }
            }
            .padding(.vertical, 2)
        }
    }

    private func presentNamePromptIfNeeded() {
        guard !settings.onboardingNameSent else { return }
        showNamePrompt = coach.runtimeState == "onboarding" || coach.runtimeState == "unknown"
    }

    private func resetLocalConversation() {
        settings.resetOnboardingNamePrompt()
        onboardingName = ""
        coach.resetConversation()
        presentNamePromptIfNeeded()
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
        [
            "The user just shared their name during onboarding. Use it naturally, then continue with the Antirot first onboarding message.",
            "Silent client context is available below for scheduling only.",
            "Do not mention timezone, profile setup, profile updates, saved fields, or that anything was saved unless the user explicitly asks.",
            "The first onboarding message asks for a gist of long-term goals, short-term goals, what the day looks like, and what the user plans to get done today.",
            "Name: \(name)",
            "Silent device timezone: \(timezone)"
        ].joined(separator: "\n")
    }

    private var transcript: some View {
        VStack(spacing: 12) {
            ForEach(coach.messages) { message in
                CoachBubble(message: message)
                    .id(message.id)
            }

            if coach.isSending {
                HStack {
                    ProgressView()
                        .tint(.antirotTextMuted)
                    Text(coach.statusText)
                        .font(.caption)
                        .foregroundStyle(.antirotTextMuted)
                    Spacer()
                }
                .glassCard(cornerRadius: 14, padding: 14)
            }
        }
    }

    private var pendingAlarmStrip: some View {
        let visibleAlarms = alarmCenter.nextReminderAlarms
        VStack(alignment: .leading, spacing: 12) {
            AntirotSectionHeader(title: "Pending Alarms", icon: "alarm")

            if visibleAlarms.isEmpty {
                HStack(spacing: 10) {
                    Image(systemName: "checkmark.circle")
                        .foregroundStyle(.antirotSuccess)
                    Text("No pending phone alarms.")
                        .font(.subheadline)
                        .foregroundStyle(.antirotTextSecondary)
                    Spacer()
                }
                .glassCard(cornerRadius: 16, padding: 14)
            } else {
                ForEach(visibleAlarms) { alarm in
                    HStack(spacing: 10) {
                        StatusDot(color: alarm.severity.color, animated: false)
                        VStack(alignment: .leading, spacing: 3) {
                            Text(alarm.title)
                                .font(.subheadline.weight(.semibold))
                                .foregroundStyle(.antirotTextPrimary)
                            Text(alarm.fireAt.formatted(date: .omitted, time: .shortened))
                                .font(.caption)
                                .foregroundStyle(.antirotTextMuted)
                        }
                        Spacer()
                    }
                    .glassCard(cornerRadius: 16, padding: 14)
                }
            }
        }
    }

    private var composer: some View {
        VStack(spacing: 10) {
            HStack(spacing: 10) {
                Button {
                    Task {
                        await coach.toggleVoice(client: client)
                        await coach.refreshRuntimeState(client: client, deviceId: settings.deviceId)
                    }
                } label: {
                    Image(systemName: coach.isRecording ? "stop.fill" : "mic.fill")
                        .font(.title3.weight(.bold))
                        .foregroundStyle(.white)
                        .frame(width: 54, height: 54)
                        .background(
                            Circle()
                                .fill(coach.isRecording ? Color.antirotAccentRed : Color.antirotAccentOrange)
                        )
                        .shadow(
                            color: (coach.isRecording ? Color.antirotAccentRed : Color.antirotAccentOrange).opacity(0.45),
                            radius: 18,
                            y: 8
                        )
                }
                .buttonStyle(.plain)
                .disabled(coach.isSending)

                TextField("Say it or type a short check-in", text: $coach.draft, axis: .vertical)
                    .lineLimit(1...3)
                    .focused($draftFocused)
                    .textInputAutocapitalization(.sentences)
                    .foregroundStyle(.antirotTextPrimary)
                    .padding(.horizontal, 14)
                    .padding(.vertical, 12)
                    .background(Color.white.opacity(0.08))
                    .clipShape(RoundedRectangle(cornerRadius: 18))
                    .overlay(
                        RoundedRectangle(cornerRadius: 18)
                            .strokeBorder(Color.white.opacity(draftFocused ? 0.22 : 0.08), lineWidth: 1)
                    )

                Button {
                    Task {
                        await coach.sendDraft(client: client)
                        await coach.refreshRuntimeState(client: client, deviceId: settings.deviceId)
                    }
                } label: {
                    Image(systemName: "arrow.up")
                        .font(.headline.weight(.bold))
                        .foregroundStyle(.white)
                        .frame(width: 42, height: 42)
                        .background(Circle().fill(Color.antirotAccentRed))
                }
                .buttonStyle(.plain)
                .disabled(coach.draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || coach.isSending)
                .opacity(coach.draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? 0.45 : 1)
            }

            Text(coach.isRecording ? "Listening: 10s minimum, gentle silence cutoff." : "Voice is preferred. Typing is the fallback.")
                .font(.caption2)
                .foregroundStyle(.antirotTextMuted)
        }
        .padding(.horizontal, 16)
        .padding(.top, 12)
        .padding(.bottom, 12)
        .background(
            Rectangle()
                .fill(.ultraThinMaterial)
                .overlay(Color.antirotBg.opacity(0.72))
                .ignoresSafeArea(.container, edges: .bottom)
        )
        .padding(.bottom, 72)
        .frame(maxHeight: .infinity, alignment: .bottom)
    }
}

private struct SiriCoachOrb: View {
    var isListening: Bool
    var isThinking: Bool
    var isSpeaking: Bool
    var statusText: String

    @State private var phase = false

    var body: some View {
        VStack(spacing: 14) {
            ZStack {
                ForEach(0..<3) { index in
                    Circle()
                        .stroke(
                            AngularGradient(
                                colors: [
                                    .antirotAccentRed,
                                    .antirotAccentOrange,
                                    .blue.opacity(0.85),
                                    .purple.opacity(0.8),
                                    .antirotAccentRed
                                ],
                                center: .center
                            ),
                            lineWidth: CGFloat(8 - index * 2)
                        )
                        .blur(radius: CGFloat(index * 4))
                        .opacity(0.65 - Double(index) * 0.12)
                        .scaleEffect(phase ? 1.0 + CGFloat(index) * 0.12 : 0.88 + CGFloat(index) * 0.08)
                        .rotationEffect(.degrees(phase ? 360 : 0))
                }

                Circle()
                    .fill(
                        RadialGradient(
                            colors: [
                                Color.white.opacity(0.38),
                                Color.antirotAccentOrange.opacity(0.22),
                                Color.antirotAccentRed.opacity(0.12),
                                Color.clear
                            ],
                            center: .topLeading,
                            startRadius: 8,
                            endRadius: 82
                        )
                    )
                    .overlay(
                        Circle()
                            .strokeBorder(Color.white.opacity(0.16), lineWidth: 1)
                    )
                    .shadow(color: .antirotAccentRed.opacity(0.32), radius: 28)

                Image(systemName: isListening ? "waveform" : "sparkles")
                    .font(.system(size: 32, weight: .semibold))
                    .foregroundStyle(.white)
                    .symbolEffect(.variableColor, isActive: isListening || isThinking || isSpeaking)
            }
            .frame(width: 156, height: 156)
            .onAppear {
                withAnimation(.linear(duration: 7).repeatForever(autoreverses: false)) {
                    phase = true
                }
            }

            Text(statusText)
                .font(.caption.weight(.semibold))
                .tracking(1.2)
                .foregroundStyle(.antirotTextMuted)
                .textCase(.uppercase)
        }
    }
}

private struct CoachBubble: View {
    var message: CoachMessage
    @State private var player: AVAudioPlayer?

    private var alignment: HorizontalAlignment {
        message.role == .user ? .trailing : .leading
    }

    private var fill: Color {
        switch message.role {
        case .user:
            return .antirotAccentRed.opacity(0.86)
        case .coach:
            return .white.opacity(0.08)
        case .system:
            return .antirotAccentAmber.opacity(0.18)
        }
    }

    var body: some View {
        HStack {
            if message.role == .user { Spacer(minLength: 48) }

            VStack(alignment: alignment, spacing: 5) {
                if let audioFileURL = message.audioFileURL {
                    Button {
                        playAudio(url: audioFileURL)
                    } label: {
                        Label("Voice message", systemImage: "play.circle.fill")
                            .font(.body.weight(.semibold))
                            .foregroundStyle(.antirotTextPrimary)
                    }
                    .buttonStyle(.plain)
                } else {
                    Text(message.text)
                        .font(.body)
                        .foregroundStyle(.antirotTextPrimary)
                        .fixedSize(horizontal: false, vertical: true)
                }

                Text(message.createdAt.formatted(date: .omitted, time: .shortened))
                    .font(.caption2)
                    .foregroundStyle(.white.opacity(0.45))
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 12)
            .background(fill)
            .clipShape(RoundedRectangle(cornerRadius: 18))
            .overlay(
                RoundedRectangle(cornerRadius: 18)
                    .strokeBorder(Color.white.opacity(0.08), lineWidth: 1)
            )

            if message.role != .user { Spacer(minLength: 48) }
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

private struct SiriCoachBackground: View {
    var body: some View {
        ZStack {
            Color.antirotBg.ignoresSafeArea()

            LinearGradient(
                colors: [
                    Color(red: 0.02, green: 0.02, blue: 0.04),
                    Color(red: 0.10, green: 0.03, blue: 0.05),
                    Color(red: 0.02, green: 0.04, blue: 0.08)
                ],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )
            .ignoresSafeArea()

            Rectangle()
                .fill(.ultraThinMaterial)
                .opacity(0.12)
                .ignoresSafeArea()
        }
    }
}

#Preview {
    HomeView()
        .environmentObject(SettingsStore())
        .environmentObject(AlarmCenter())
        .environmentObject(CoachViewModel())
}
