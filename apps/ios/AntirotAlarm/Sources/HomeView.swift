import SwiftUI
import UIKit

enum HomeLayoutMetrics {
    static let headerTopPadding: CGFloat = 24
}

enum BackendConnectionPresentation {
    static func label(isReachable: Bool?) -> String {
        switch isReachable {
        case .some(true): return "CONNECTED"
        case .some(false): return "OFFLINE"
        case .none: return "SYNCING"
        }
    }
}

// MARK: - Home (Coach Room)

/// The editorial coach room: a full-screen animated coach, one dominant
/// state action, quiet secondary actions, and a compact command sheet.
struct HomeView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var alarmCenter: AlarmCenter
    @EnvironmentObject private var coach: CoachViewModel
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    @State private var onboardingName = ""
    @State private var showNamePrompt = false
    @State private var sheetHeight: CGFloat = ChatSheetDetents.collapsedHeight
    private let actionClearance: CGFloat = 132
    private let chatBottomClearance = AppBottomBarMetrics.coachChatClearance
    private var client: APIClient {
        APIClient(baseURL: settings.baseURL, apiToken: settings.apiToken, userId: settings.userId)
    }

    var body: some View {
        GeometryReader { _ in
            ZStack(alignment: .bottom) {
                CoachStage(emotion: coach.coachEmotion, isThinking: coach.isSending)
                    .ignoresSafeArea()
                    .ignoresSafeArea(.keyboard)

                homeHeader
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                    .padding(.horizontal, 20)
                    .padding(.top, HomeLayoutMetrics.headerTopPadding)
                    .ignoresSafeArea(.keyboard)

                actionStack
                    .padding(.bottom, min(sheetHeight, actionClearance) + chatBottomClearance + 22)
                    .padding(.horizontal, 20)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)
                    .ignoresSafeArea(.keyboard)

                GlassSheet(
                    height: $sheetHeight,
                    messages: coach.messages,
                    draft: $coach.draft,
                    isRecording: coach.isRecording,
                    isSending: coach.isSending,
                    statusText: coach.statusText,
                    latestOneLiner: latestOneLiner,
                    bottomInset: chatBottomClearance,
                    onMic: { Task { await micTapped() } },
                    onSend: { Task { await sendTapped() } },
                    onPlayVoiceMessage: { url in coach.playVoiceMessage(url) }
                )
            }
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
    var homeHeader: some View {
        HStack(alignment: .top, spacing: 14) {
            VStack(alignment: .leading, spacing: 2) {
                Text("COACH / \(connectionLabel)")
                    .font(.system(size: 11, weight: .semibold, design: .monospaced))
                    .tracking(1.2)
                    .foregroundStyle(.arAccent)
                Text("No excuses.")
                    .font(.system(.largeTitle, design: .serif, weight: .semibold))
                    .foregroundStyle(.arTextPrimary)
                    .accessibilityAddTraits(.isHeader)
            }

            Spacer(minLength: 8)

            StatePill(
                label: runtimeStateLabel,
                isActive: coach.runtimeState.lowercased() != "unknown"
            )
        }
        .overlay(alignment: .bottomLeading) {
            HStack(spacing: 7) {
                StatusDot(color: connectionColor, animated: !reduceMotion)
                Text(connectionLabel)
                    .font(.system(size: 11, weight: .medium, design: .monospaced))
                    .tracking(0.8)
                    .foregroundStyle(.arTextSecondary)
            }
            .offset(y: 24)
        }
    }

    var actionStack: some View {
        let set = CoachStateActions.actions(for: coach.runtimeState)
        return VStack(spacing: 12) {
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

    var runtimeStateLabel: String {
        switch coach.runtimeState.lowercased() {
        case "onboarding":
            return "Onboarding"
        case "idle":
            return "Idle"
        case "working":
            return "Working"
        case "break":
            return "Break"
        case "sleeping":
            return "Sleeping"
        case "vacation":
            return "Vacation"
        case "offline":
            return "Offline"
        case "unknown":
            return "Syncing state"
        default:
            return coach.runtimeState.capitalized
        }
    }

    var connectionLabel: String {
        BackendConnectionPresentation.label(isReachable: coach.isBackendReachable)
    }

    var connectionColor: Color {
        switch coach.isBackendReachable {
        case .some(true): return .arSuccess
        case .some(false): return .arDanger
        case .none: return .arWarning
        }
    }

    var latestOneLiner: String {
        if let last = coach.messages.last(where: { $0.role == .coach }), !last.text.isEmpty {
            return String(last.text.prefix(120)).replacingOccurrences(of: "\n", with: " ")
        }
        return coach.coachEmotion.ambientOneLiner
    }

    func sendStateButton(_ button: CoachStateButton) async {
        coach.recordDiagnosticEvent(
            kind: "button.\(button.id)",
            summary: "\(button.title) pressed.",
            detail: button.message
        )
        if button.id == "done", settings.autoSnapshotOnStop {
            await saveStopSnapshot()
        }
        if button.triggersConfetti {
            coach.showConfetti = true
        }
        await coach.send(button.message, client: client)
        await coach.refreshRuntimeState(client: client, deviceId: settings.deviceId)
    }

    func saveStopSnapshot() async {
        do {
            let response = try await client.createMemorySnapshot(CreateMemorySnapshotRequest(
                deviceId: settings.deviceId,
                title: "Before stop",
                reason: "auto_stop_ios"
            ))
            coach.recordDiagnosticEvent(
                kind: "memory_snapshot.auto_saved",
                summary: "Memory snapshot saved before stop.",
                detail: response.snapshot.id
            )
        } catch {
            coach.recordDiagnosticEvent(
                kind: "memory_snapshot.auto_save_failed",
                summary: "Memory snapshot before stop failed.",
                detail: error.localizedDescription
            )
        }
    }

    func micTapped() async {
        await coach.toggleVoice(client: client)
        await coach.refreshRuntimeState(client: client, deviceId: settings.deviceId)
    }

    func sendTapped() async {
        await MainActor.run {
            openChat(availableHeight: sheetAvailableHeight(UIScreen.main.bounds.height))
        }
        await coach.sendDraft(client: client)
        await coach.refreshRuntimeState(client: client, deviceId: settings.deviceId)
    }

    func sheetAvailableHeight(_ screenHeight: CGFloat) -> CGFloat {
        max(1, screenHeight - chatBottomClearance)
    }

    func openChat(availableHeight: CGFloat) {
        withAnimation(reduceMotion ? .easeOut(duration: 0.14) : .spring(response: 0.22, dampingFraction: 0.86)) {
            sheetHeight = ChatSheetDetents.nextExpandedHeight(
                from: sheetHeight,
                availableHeight: availableHeight
            )
        }
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
