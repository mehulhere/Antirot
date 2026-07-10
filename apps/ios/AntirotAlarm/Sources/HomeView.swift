import SwiftUI
import UIKit

enum HomeLayoutMetrics {
    static let headerTopPadding: CGFloat = 24
}

// MARK: - Home (Coach Room)

/// The cinematic coach room: a full-screen stylized coach, one dominant
/// circular action button per runtime state, optional quiet secondary
/// actions, and a draggable glass chat sheet pinned to the bottom. There is
/// no dashboard clutter here; secondary surfaces live in the bottom app bar.
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
        GeometryReader { proxy in
            ZStack(alignment: .bottom) {
                CoachStage(emotion: coach.coachEmotion, isThinking: coach.isSending)
                    .ignoresSafeArea()
                    .ignoresSafeArea(.keyboard)

                homeHeader
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                    .padding(.horizontal, 20)
                    .padding(.top, HomeLayoutMetrics.headerTopPadding)
                    .ignoresSafeArea(.keyboard)

                Color.clear
                    .contentShape(Rectangle())
                    .ignoresSafeArea()
                    .gesture(homeSwipeUpGesture(availableHeight: proxy.size.height))

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
        HStack(alignment: .center, spacing: 14) {
            VStack(alignment: .leading, spacing: 2) {
                Text("Coach")
                    .font(.system(size: 28, weight: .bold, design: .rounded))
                    .foregroundStyle(.arTextPrimary)
                HStack(spacing: 7) {
                    StatusDot(color: .arSuccess, animated: !reduceMotion)
                    Text("Antirot connected")
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(.arTextSecondary)
                }
            }

            Spacer(minLength: 8)

            StatePill(
                label: runtimeStateLabel,
                isActive: coach.runtimeState.lowercased() != "unknown"
            )
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 14)
        .smokedGlass(cornerRadius: 26, tint: .arSurface)
    }

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

    func homeSwipeUpGesture(availableHeight: CGFloat) -> some Gesture {
        DragGesture(minimumDistance: 24)
            .onEnded { value in
                let vertical = value.translation.height
                let horizontal = abs(value.translation.width)
                guard ChatSheetDetents.isCollapsed(sheetHeight) else { return }
                guard vertical < -36, abs(vertical) > horizontal * 1.2 else { return }
                openChat(availableHeight: sheetAvailableHeight(availableHeight))
            }
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
