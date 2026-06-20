import AVFoundation
import Foundation

@MainActor
final class CoachViewModel: ObservableObject {
    @Published var messages: [CoachMessage] = [
        CoachMessage(
            role: .coach,
            text: "Tell me what you are doing now. Short is fine. I will turn it into the next move."
        )
    ]
    @Published var draft = ""
    @Published var isSending = false
    @Published var isSpeaking = false
    @Published var statusText = "Ready"
    @Published var runtimeState = "unknown"

    let recorder = VoiceRecorder()

    private var audioPlayer: AVAudioPlayer?
    private var pendingChatMessages: [String] = []
    private var chatQueueProcessing = false
    private var pendingVoiceSegments: [URL] = []
    private var voiceQueueProcessing = false

    var isRecording: Bool {
        recorder.isRecording
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
        do {
            let response = try await client.fetchRuntimeState(deviceId: deviceId)
            runtimeState = response.runtimeState?.state ?? "unknown"
        } catch {
            runtimeState = "unknown"
        }
    }

    func toggleVoice(client: APIClient) async {
        if recorder.isRecording {
            guard let url = recorder.stop() else { return }
            await enqueueVoiceSegment(url: url, client: client)
        } else {
            await recorder.start { [weak self] url in
                Task { @MainActor in
                    await self?.enqueueVoiceSegment(url: url, client: client)
                }
            }
            statusText = recorder.isRecording ? "Listening: gentle VAD waits for a useful clip" : (recorder.lastError ?? "Mic unavailable")
        }
    }

    func send(_ text: String, client: APIClient) async {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        pendingChatMessages.append(trimmed)
        await processChatQueue(client: client)
    }

    private func processChatQueue(client: APIClient) async {
        guard !chatQueueProcessing else { return }
        chatQueueProcessing = true

        while !pendingChatMessages.isEmpty {
            let trimmed = pendingChatMessages.removeFirst()
            messages.append(CoachMessage(role: .user, text: trimmed))
            isSending = true
            statusText = pendingChatMessages.isEmpty ? "Thinking" : "Thinking (\(pendingChatMessages.count) queued)"

            do {
                let response = try await client.chat(message: trimmed)
                messages.append(CoachMessage(role: .coach, text: response.reply))
                statusText = "Ready"
                await speak(response.reply, client: client)
            } catch {
                statusText = "Chat failed"
                messages.append(CoachMessage(role: .system, text: error.localizedDescription))
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
                messages.append(CoachMessage(role: .system, text: "Transcribed voice: \(response.text)"))
                await send(response.text, client: client)
            } catch {
                statusText = "Voice failed"
                messages.append(CoachMessage(
                    role: .system,
                    text: "Voice transcription failed: \(error.localizedDescription)"
                ))
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
}
