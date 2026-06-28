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
            MeshBackground()

            VStack(spacing: 0) {
                header
                    .padding(.horizontal, 20)
                    .padding(.top, 12)

                ScrollViewReader { proxy in
                    ScrollView(.vertical, showsIndicators: false) {
                        VStack(spacing: 18) {
                            FocusDial(
                                isRecording: coach.isRecording,
                                isThinking: coach.isSending
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
            // Red monogram
            ZStack {
                RoundedRectangle(cornerRadius: 10)
                    .fill(Color.antirotGlowPrimary)
                    .overlay(
                        RoundedRectangle(cornerRadius: 10)
                            .strokeBorder(Color.antirotBorderStrong, lineWidth: 1)
                    )
                Text("A")
                    .font(.headline.bold())
                    .foregroundStyle(.antirotAccent)
            }
            .frame(width: 36, height: 36)
            .accessibilityHidden(true)

            VStack(alignment: .leading, spacing: 2) {
                Text("Coach")
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
                    .background(Color.antirotBgElevated)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
                    .overlay(
                        RoundedRectangle(cornerRadius: 12)
                            .strokeBorder(Color.antirotBorder, lineWidth: 1)
                    )
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Reset conversation")

            StatusDot(color: settings.registered ? .antirotCyan : .antirotDanger)
        }
    }

    private var currentTaskStrip: some View {
        let snapshot = SharedTaskStore.read()

        return VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                Image(systemName: "target")
                    .foregroundStyle(.antirotAccent)
                Text(snapshot.mode.uppercased())
                    .font(.caption2.weight(.bold))
                    .tracking(1)
                    .foregroundStyle(.antirotTextMuted)
                Spacer()
                if let dueAt = snapshot.dueAt {
                    Text(dueAt, style: .relative)
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(.antirotGold)
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
        .layeredCard(cornerRadius: 14, padding: 16)
        .overlay(alignment: .leading) {
            Rectangle()
                .fill(Color.antirotAccent)
                .frame(width: 3)
                .clipShape(UnevenRoundedRectangle(topLeadingRadius: 14, bottomLeadingRadius: 14))
        }
    }

    private var quickActions: some View {
        let actions = CoachQuickAction.primary(for: coach.runtimeState, at: quickActionRefreshDate)

        return ScrollView(.horizontal, showsIndicators: false) {
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
                        .background(Color.antirotGlowPrimary)
                        .clipShape(Capsule())
                        .overlay(
                            Capsule()
                                .strokeBorder(Color.antirotBorderStrong, lineWidth: 1)
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
        return [
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
                .layeredCard(cornerRadius: 14, padding: 14)
            }
        }
    }

    private var pendingAlarmStrip: some View {
        let visibleAlarms = alarmCenter.nextReminderAlarms
        return VStack(alignment: .leading, spacing: 12) {
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
                .layeredCard(cornerRadius: 14, padding: 14)
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
                    .layeredCard(cornerRadius: 14, padding: 14)
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
                        .frame(width: 50, height: 50)
                        .background(
                            Circle()
                                .fill(
                                    coach.isRecording
                                        ? Color.antirotDanger
                                        : Color.antirotAccent
                                )
                        )
                        .shadow(
                            color: (coach.isRecording ? Color.antirotDanger : Color.antirotAccent).opacity(0.4),
                            radius: 16,
                            y: 6
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
                    .background(Color.antirotBgElevated)
                    .clipShape(RoundedRectangle(cornerRadius: 18))
                    .overlay(
                        RoundedRectangle(cornerRadius: 18)
                            .strokeBorder(
                                draftFocused ? Color.antirotBorderStrong : Color.antirotBorder,
                                lineWidth: 1
                            )
                    )

                Button {
                    Task {
                        await coach.sendDraft(client: client)
                        await coach.refreshRuntimeState(client: client, deviceId: settings.deviceId)
                    }
                } label: {
                    Image(systemName: "arrow.up")
                        .font(.headline.weight(.bold))
                        .foregroundStyle(
                            coach.draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                                ? .white
                                : Color.antirotBgElevated
                        )
                        .frame(width: 42, height: 42)
                        .background(
                            Circle().fill(
                                coach.draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                                    ? Color.antirotAccent
                                    : Color.antirotGold
                            )
                        )
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
                .fill(Color.antirotBgElevated)
                .overlay(alignment: .top) {
                    Rectangle()
                        .fill(Color.antirotBorderStrong)
                        .frame(height: 0.5)
                }
                .ignoresSafeArea(.container, edges: .bottom)
        )
        .padding(.bottom, 72)
        .frame(maxHeight: .infinity, alignment: .bottom)
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
            return .antirotAccent.opacity(0.20)
        case .coach:
            return .antirotBgElevated
        case .system:
            return .antirotGold.opacity(0.12)
        }
    }

    private var borderColor: Color {
        switch message.role {
        case .user:
            return .antirotBorderStrong
        case .coach:
            return .antirotBorder
        case .system:
            return .antirotGold.opacity(0.18)
        }
    }

    private var bubbleShape: UnevenRoundedRectangle {
        switch message.role {
        case .user:
            return UnevenRoundedRectangle(
                topLeadingRadius: 14, bottomLeadingRadius: 14,
                bottomTrailingRadius: 4, topTrailingRadius: 14
            )
        case .coach:
            return UnevenRoundedRectangle(
                topLeadingRadius: 14, bottomLeadingRadius: 4,
                bottomTrailingRadius: 14, topTrailingRadius: 14
            )
        case .system:
            return UnevenRoundedRectangle(
                topLeadingRadius: 14, bottomLeadingRadius: 14,
                bottomTrailingRadius: 14, topTrailingRadius: 14
            )
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
                    .foregroundStyle(.antirotTextMuted)
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 12)
            .background(fill)
            .clipShape(bubbleShape)
            .overlay(
                bubbleShape
                    .strokeBorder(borderColor, lineWidth: 1)
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
