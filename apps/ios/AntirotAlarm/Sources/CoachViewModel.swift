import AVFoundation
import Foundation

@MainActor
final class CoachViewModel: ObservableObject {
    @Published var messages: [CoachMessage] = [
        CoachMessage(
            role: .coach,
            text: "I’m Antirot. I’ve coached plenty of people like you: smart, intense, full of plans, and somehow still one bad hour away from drifting off the thing they claim matters.\n\nSo let’s see what you’ve got. I need to build your profile. Give me a gist of your long-term and short-term goals. You can update this later as well. Because obviously, ambition is not a gift everyone has.\n\nTell me what your day looks like and what you’re planning to get done today."
        )
    ]
    @Published var draft = ""
    @Published var isSending = false
    @Published var isSpeaking = false
    @Published var statusText = "Ready"
    @Published var runtimeState = "unknown"
    @Published var coachEmotion: CoachEmotion = .watching
    @Published var showConfetti = false
    @Published private(set) var diagnosticEvents: [ReportEventPayload] = []

    let recorder = VoiceRecorder()

    private var audioPlayer: AVAudioPlayer?
    private var emotionResetTask: Task<Void, Never>?
    private var pendingChatMessages: [QueuedChatMessage] = []
    private var chatQueueProcessing = false
    private var pendingVoiceSegments: [URL] = []
    private var voiceQueueProcessing = false

    var isRecording: Bool {
        recorder.isRecording
    }

    func resetConversation() {
        pendingChatMessages.removeAll()
        pendingVoiceSegments.removeAll()
        chatQueueProcessing = false
        voiceQueueProcessing = false
        draft = ""
        isSending = false
        statusText = "Ready"
        messages = [
            CoachMessage(
                role: .system,
                text: "Conversation reset."
            )
        ]
        recordDiagnosticEvent(kind: "conversation.reset", summary: "Conversation reset.")
    }

    func handleQuickAction(_ action: CoachQuickAction, client: APIClient) async {
        if action.fillsDraft {
            draft = action.message
            statusText = "Finish the sentence"
            return
        }

        await send(action.message, client: client)
    }

    func sendDraft(client: APIClient) async {
        let text = draft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return }
        draft = ""
        await send(text, client: client)
    }

    func refreshRuntimeState(client: APIClient, deviceId: String) async {
        let previous = runtimeState
        do {
            let response = try await client.fetchRuntimeState(deviceId: deviceId)
            runtimeState = response.runtimeState?.state ?? "unknown"
            if runtimeState != previous {
                recordDiagnosticEvent(
                    kind: "state.changed",
                    summary: "\(previous) -> \(runtimeState)",
                    detail: "source=runtime refresh"
                )
            }
        } catch {
            runtimeState = "unknown"
            if runtimeState != previous {
                recordDiagnosticEvent(
                    kind: "state.changed",
                    summary: "\(previous) -> unknown",
                    detail: "runtime refresh failed"
                )
            }
        }
    }

    private func applyEmotion(from response: ChatCoachResponse) {
        coachEmotion = response.emotion
        scheduleEmotionReset()
    }

    private func scheduleEmotionReset() {
        emotionResetTask?.cancel()
        emotionResetTask = Task { [weak self] in
            try? await Task.sleep(nanoseconds: 7 * 1_000_000_000)
            guard !Task.isCancelled, let self else { return }
            if !self.isSending {
                self.coachEmotion = .watching
            }
        }
    }

    func toggleVoice(client: APIClient) async {
        if recorder.isRecording {
            guard let url = recorder.stop() else { return }
            recordDiagnosticEvent(kind: "voice.stop", summary: "Voice recording stopped.")
            await enqueueVoiceSegment(url: url, client: client)
        } else {
            await recorder.start { [weak self] url in
                Task { @MainActor in
                    await self?.enqueueVoiceSegment(url: url, client: client)
                }
            }
            statusText = recorder.isRecording ? "Listening: gentle VAD waits for a useful clip" : (recorder.lastError ?? "Mic unavailable")
            recordDiagnosticEvent(
                kind: recorder.isRecording ? "voice.start" : "voice.failed",
                summary: statusText
            )
        }
    }

    func playVoiceMessage(_ url: URL) {
        guard FileManager.default.fileExists(atPath: url.path) else {
            statusText = "Voice file missing"
            messages.append(CoachMessage(
                role: .system,
                text: "Voice playback failed: the local audio file is no longer available."
            ))
            return
        }

        do {
            let session = AVAudioSession.sharedInstance()
            try session.setCategory(.playback, mode: .spokenAudio, options: [.duckOthers])
            try session.setActive(true)

            audioPlayer?.stop()
            let player = try AVAudioPlayer(contentsOf: url)
            player.prepareToPlay()
            guard player.play() else {
                throw VoicePlaybackError.playbackDidNotStart
            }
            audioPlayer = player
            statusText = "Playing voice message"
        } catch {
            statusText = "Voice playback failed"
            messages.append(CoachMessage(
                role: .system,
                text: "Voice playback failed: \(error.localizedDescription)"
            ))
        }
    }

    func send(_ text: String, visibleText: String? = nil, client: APIClient) async {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        let visible = visibleText?.trimmingCharacters(in: .whitespacesAndNewlines)
        let visibleText: String?
        if let visible {
            visibleText = visible.isEmpty ? nil : visible
        } else {
            visibleText = trimmed
        }
        pendingChatMessages.append(QueuedChatMessage(text: trimmed, visibleText: visibleText))
        recordDiagnosticEvent(
            kind: "chat.enqueued",
            summary: "Queued user message.",
            detail: visibleText ?? trimmed
        )
        await processChatQueue(client: client)
    }

    private func processChatQueue(client: APIClient) async {
        guard !chatQueueProcessing else { return }
        chatQueueProcessing = true

        while !pendingChatMessages.isEmpty {
            let queued = pendingChatMessages.removeFirst()
            if let visibleText = queued.visibleText {
                messages.append(CoachMessage(role: .user, text: visibleText))
            }
            isSending = true
            coachEmotion = .thinking
            statusText = pendingChatMessages.isEmpty ? "Thinking" : "Thinking (\(pendingChatMessages.count) queued)"

            do {
                let response = try await client.chat(message: queued.text)
                messages.append(CoachMessage(role: .coach, text: response.reply))
                statusText = "Ready"
                recordDiagnosticEvent(kind: "chat.reply", summary: "Coach reply received.", detail: response.reply)
                applyEmotion(from: response)
                if let preface = response.voicePreface?.trimmingCharacters(in: .whitespacesAndNewlines), !preface.isEmpty {
                    await speak(preface, client: client)
                } else {
                    await speak(response.reply, client: client)
                }
            } catch {
                statusText = "Chat failed"
                messages.append(CoachMessage(role: .system, text: error.localizedDescription))
                recordDiagnosticEvent(kind: "chat.failed", summary: "Chat failed.", detail: error.localizedDescription)
            }
        }

        isSending = false
        chatQueueProcessing = false
        if !pendingChatMessages.isEmpty {
            await processChatQueue(client: client)
        }
    }

    private func enqueueVoiceSegment(url: URL, client: APIClient) async {
        pendingVoiceSegments.append(url)
        await processVoiceQueue(client: client)
    }

    private func processVoiceQueue(client: APIClient) async {
        guard !voiceQueueProcessing else { return }
        voiceQueueProcessing = true

        while !pendingVoiceSegments.isEmpty {
            let url = pendingVoiceSegments.removeFirst()
            isSending = true
            statusText = pendingVoiceSegments.isEmpty ? "Transcribing" : "Transcribing (\(pendingVoiceSegments.count) queued)"

            do {
                let response = try await client.transcribeAudio(fileURL: url)
                messages.append(CoachMessage(role: .user, text: "Voice message", audioFileURL: url))
                recordDiagnosticEvent(kind: "voice.transcribed", summary: "Voice segment transcribed.", detail: response.text)
                await send(response.text, visibleText: "", client: client)
            } catch {
                statusText = "Voice failed"
                messages.append(CoachMessage(
                    role: .system,
                    text: "Voice transcription failed: \(error.localizedDescription)"
                ))
                recordDiagnosticEvent(kind: "voice.failed", summary: "Voice transcription failed.", detail: error.localizedDescription)
            }
        }

        if !chatQueueProcessing {
            isSending = false
            statusText = "Ready"
        }
        voiceQueueProcessing = false
        if !pendingVoiceSegments.isEmpty {
            await processVoiceQueue(client: client)
        }
    }

    private func speak(_ text: String, client: APIClient) async {
        guard !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }
        isSpeaking = true

        do {
            let response = try await client.synthesizeSpeech(text: text)
            guard let data = response.audioData else {
                isSpeaking = false
                return
            }
            audioPlayer = try AVAudioPlayer(data: data)
            audioPlayer?.prepareToPlay()
            audioPlayer?.play()
        } catch {
            statusText = "Text only"
        }

        isSpeaking = false
    }

    func recordDiagnosticEvent(kind: String, summary: String, detail: String? = nil) {
        diagnosticEvents.append(ReportEventPayload(
            at: Date(),
            kind: kind,
            summary: summary,
            detail: detail
        ))

        if diagnosticEvents.count > 120 {
            diagnosticEvents.removeFirst(diagnosticEvents.count - 120)
        }
    }
}

private struct QueuedChatMessage {
    var text: String
    var visibleText: String?
}

private enum VoicePlaybackError: LocalizedError {
    case playbackDidNotStart

    var errorDescription: String? {
        "The voice message could not start playing."
    }
}
