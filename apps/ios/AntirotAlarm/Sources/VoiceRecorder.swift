import AVFoundation
import Foundation

@MainActor
final class VoiceRecorder: NSObject, ObservableObject {
    @Published private(set) var isRecording = false
    @Published var lastError: String?

    private var recorder: AVAudioRecorder?
    private var currentURL: URL?

    func start() async {
        lastError = nil
        guard await requestMicrophonePermission() else {
            lastError = "Microphone permission is required for voice check-ins."
            return
        }

        do {
            let session = AVAudioSession.sharedInstance()
            try session.setCategory(.playAndRecord, mode: .spokenAudio, options: [.defaultToSpeaker])
            try session.setActive(true)

            let url = FileManager.default.temporaryDirectory
                .appendingPathComponent("antirot-voice-\(UUID().uuidString).m4a")
            let settings: [String: Any] = [
                AVFormatIDKey: Int(kAudioFormatMPEG4AAC),
                AVSampleRateKey: 44_100,
                AVNumberOfChannelsKey: 1,
                AVEncoderAudioQualityKey: AVAudioQuality.high.rawValue
            ]
            let recorder = try AVAudioRecorder(url: url, settings: settings)
            recorder.prepareToRecord()
            recorder.record()
            self.recorder = recorder
            self.currentURL = url
            self.isRecording = true
        } catch {
            lastError = error.localizedDescription
            isRecording = false
        }
    }

    func stop() -> URL? {
        guard isRecording else { return nil }
        recorder?.stop()
        recorder = nil
        isRecording = false
        return currentURL
    }

    private func requestMicrophonePermission() async -> Bool {
        await withCheckedContinuation { continuation in
            AVAudioSession.sharedInstance().requestRecordPermission { granted in
                continuation.resume(returning: granted)
            }
        }
    }
}
