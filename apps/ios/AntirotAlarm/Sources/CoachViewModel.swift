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

    let recorder = VoiceRecorder()

    private var audioPlayer: AVAudioPlayer?

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

    func toggleVoice(client: APIClient) async {
        if recorder.isRecording {
            guard let url = recorder.stop() else { return }
            await transcribeAndSend(url: url, client: client)
        } else {
            await recorder.start()
            statusText = recorder.isRecording ? "Listening" : (recorder.lastError ?? "Mic unavailable")
        }
    }

    func send(_ text: String, client: APIClient) async {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty, !isSending else { return }

        messages.append(CoachMessage(role: .user, text: trimmed))
        isSending = true
        statusText = "Thinking"

        do {
            let response = try await client.chat(message: trimmed)
            messages.append(CoachMessage(role: .coach, text: response.reply))
            isSending = false
            statusText = "Ready"
            await speak(response.reply, client: client)
        } catch {
            isSending = false
            statusText = "Chat failed"
            messages.append(CoachMessage(role: .system, text: error.localizedDescription))
        }
    }

    private func transcribeAndSend(url: URL, client: APIClient) async {
        isSending = true
        statusText = "Transcribing"

        do {
            let response = try await client.transcribeAudio(fileURL: url)
            isSending = false
            statusText = "Ready"
            await send(response.text, client: client)
        } catch {
            isSending = false
            statusText = "Voice failed"
            messages.append(CoachMessage(
                role: .system,
                text: "Voice transcription failed: \(error.localizedDescription)"
            ))
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
