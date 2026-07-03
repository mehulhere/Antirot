import Foundation
import AVFoundation

enum SoundLibrary {
    static func importAlarmSound(from sourceURL: URL) async throws -> String {
        let fileExtension = sourceURL.pathExtension.lowercased()
        guard isSupportedNotificationSoundExtension(fileExtension) else {
            throw SoundLibraryError.unsupportedFormat(fileExtension)
        }

        let didAccess = sourceURL.startAccessingSecurityScopedResource()
        defer {
            if didAccess {
                sourceURL.stopAccessingSecurityScopedResource()
            }
        }

        let asset = AVURLAsset(url: sourceURL)
        let duration = CMTimeGetSeconds(try await asset.load(.duration))
        guard isValidNotificationSoundDuration(duration) else {
            throw SoundLibraryError.tooLong(duration)
        }

        let soundsDirectory = try soundsDirectory()
        let destinationName = "antirot-selected.\(fileExtension)"
        let destinationURL = soundsDirectory.appendingPathComponent(destinationName)
        if FileManager.default.fileExists(atPath: destinationURL.path) {
            try FileManager.default.removeItem(at: destinationURL)
        }
        try FileManager.default.copyItem(at: sourceURL, to: destinationURL)
        return destinationName
    }

    static func isSupportedNotificationSoundExtension(_ fileExtension: String) -> Bool {
        ["aiff", "caf", "wav"].contains(fileExtension.lowercased())
    }

    static func isValidNotificationSoundDuration(_ duration: Double) -> Bool {
        duration.isFinite && duration > 0 && duration < 30
    }

    private static func soundsDirectory() throws -> URL {
        let libraryURL = try FileManager.default.url(
            for: .libraryDirectory,
            in: .userDomainMask,
            appropriateFor: nil,
            create: true
        )
        let soundsURL = libraryURL.appendingPathComponent("Sounds", isDirectory: true)
        try FileManager.default.createDirectory(at: soundsURL, withIntermediateDirectories: true)
        return soundsURL
    }
}

enum SoundLibraryError: LocalizedError {
    case unsupportedFormat(String)
    case tooLong(Double)

    var errorDescription: String? {
        switch self {
        case let .unsupportedFormat(fileExtension):
            return "Unsupported sound format: \(fileExtension.isEmpty ? "missing extension" : fileExtension)"
        case let .tooLong(duration):
            if duration.isFinite {
                return "Sound is \(Int(duration.rounded())) seconds. iOS notification sounds must be shorter than 30 seconds."
            }
            return "The sound duration could not be read."
        }
    }
}
