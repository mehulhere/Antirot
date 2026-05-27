import Foundation
import AVFoundation

enum SoundLibrary {
    static func importAlarmSound(from sourceURL: URL) throws -> String {
        let allowedExtensions = ["aif", "aiff", "caf", "m4a", "mp3", "wav"]
        let fileExtension = sourceURL.pathExtension.lowercased()
        guard allowedExtensions.contains(fileExtension) else {
            throw SoundLibraryError.unsupportedFormat(fileExtension)
        }

        let didAccess = sourceURL.startAccessingSecurityScopedResource()
        defer {
            if didAccess {
                sourceURL.stopAccessingSecurityScopedResource()
            }
        }

        let asset = AVURLAsset(url: sourceURL)
        let duration = CMTimeGetSeconds(asset.duration)
        if duration.isFinite && duration > 30 {
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
            return "Sound is \(Int(duration.rounded())) seconds. iOS alarm sounds must be 30 seconds or shorter."
        }
    }
}
