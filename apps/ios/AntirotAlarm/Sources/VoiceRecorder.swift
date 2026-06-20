import AVFoundation
import Foundation

@MainActor
final class VoiceRecorder: NSObject, ObservableObject {
    private enum Vad {
        static let minimumClipDuration: TimeInterval = 10
        static let preferredClipDuration: TimeInterval = 30
        static let hardClipDuration: TimeInterval = 60
        static let settledSilenceDuration: TimeInterval = 1.5
        static let gentleVoicePower: Float = -46
        static let fallbackSpeechWindow: TimeInterval = 4
    }

    @Published private(set) var isRecording = false
    @Published var lastError: String?

    private var recorder: AVAudioRecorder?
    private var currentURL: URL?
    private var startedAt: Date?
    private var lastVoiceAt: Date?
    private var vadTimer: Timer?
    private var onSegmentReady: ((URL) -> Void)?

    func start(onSegmentReady: ((URL) -> Void)? = nil) async {
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
            recorder.isMeteringEnabled = true
            recorder.prepareToRecord()
            recorder.record()
            self.recorder = recorder
            self.currentURL = url
            self.startedAt = Date()
            self.lastVoiceAt = Date()
            self.onSegmentReady = onSegmentReady
            self.isRecording = true
            startGentleVadTimer()
        } catch {
            lastError = error.localizedDescription
            isRecording = false
        }
    }

    func stop() -> URL? {
        guard isRecording else { return nil }
        vadTimer?.invalidate()
        vadTimer = nil
        recorder?.stop()
        recorder = nil
        isRecording = false
        startedAt = nil
        lastVoiceAt = nil
        onSegmentReady = nil
        return currentURL
    }

    private func startGentleVadTimer() {
        vadTimer?.invalidate()
        vadTimer = Timer.scheduledTimer(withTimeInterval: 0.2, repeats: true) { [weak self] _ in
            Task { @MainActor in
                self?.evaluateGentleVad()
            }
        }
    }

    private func evaluateGentleVad() {
        guard isRecording, let recorder, let startedAt else { return }
        recorder.updateMeters()

        let now = Date()
        let elapsed = now.timeIntervalSince(startedAt)
        let averagePower = recorder.averagePower(forChannel: 0)
        let peakPower = recorder.peakPower(forChannel: 0)

        if averagePower >= Vad.gentleVoicePower || peakPower >= Vad.gentleVoicePower + 6 {
            lastVoiceAt = now
            return
        }

        let silenceDuration = now.timeIntervalSince(lastVoiceAt ?? startedAt)
        let hasEnoughAudio = elapsed >= Vad.minimumClipDuration
        let hasSettledSilence = silenceDuration >= Vad.settledSilenceDuration
        let shouldPreferFlush = elapsed >= Vad.preferredClipDuration && hasSettledSilence
        let shouldMinimumFlush = hasEnoughAudio && hasSettledSilence
        let shouldHardFlush = elapsed >= Vad.hardClipDuration

        if shouldHardFlush || shouldPreferFlush || shouldMinimumFlush {
            finishSegment()
        } else if elapsed >= Vad.minimumClipDuration + Vad.fallbackSpeechWindow && lastVoiceAt == startedAt {
            finishSegment()
        }
    }

    private func finishSegment() {
        let callback = onSegmentReady
        guard let url = stop() else { return }
        callback?(url)
    }

    private func requestMicrophonePermission() async -> Bool {
        await withCheckedContinuation { continuation in
            AVAudioSession.sharedInstance().requestRecordPermission { granted in
                continuation.resume(returning: granted)
            }
        }
    }
}
