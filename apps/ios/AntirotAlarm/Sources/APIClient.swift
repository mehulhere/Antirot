import Foundation

struct APIClient {
    private enum RequestTimeout {
        static let health: TimeInterval = 15
        static let standard: TimeInterval = 60
        static let chat: TimeInterval = 300
        static let speechTranscription: TimeInterval = 60
    }

    private static let maxAudioUploadBytes = 25 * 1024 * 1024

    enum APIError: Error, LocalizedError {
        case missingServerURL
        case invalidResponse(status: Int, body: String)
        case decodeFailed(body: String)
        case transportFailed(url: String, underlying: String)
        case audioUploadTooLarge(bytes: Int, max: Int)

        var errorDescription: String? {
            switch self {
            case .missingServerURL:
                "Backend URL is invalid. Open Developer Settings and reset it to api.antirot.org."
            case let .invalidResponse(status, body):
                "Antirot backend returned HTTP \(status): \(body)"
            case let .decodeFailed(body):
                "Antirot backend returned unexpected JSON: \(body)"
            case let .transportFailed(url, underlying):
                "Could not complete Antirot backend request at \(url). Network failed before an HTTP response or the backend took too long to reply. \(underlying)"
            case let .audioUploadTooLarge(bytes, max):
                "Voice note is too large (\(bytes) bytes). Keep uploads under \(max) bytes."
            }
        }

        var recoverySuggestion: String? {
            switch self {
            case .missingServerURL:
                "Open Developer Settings and reset the server URL to https://api.antirot.org."
            case .invalidResponse:
                "The device reached the backend. Share this HTTP status and body if sign-in still fails."
            case .decodeFailed:
                "The device reached the backend, but the app and backend disagree on the response shape. Rebuild the app from the latest code."
            case .transportFailed:
                "Open https://api.antirot.org/v1/health on this iPhone. If Safari works but chat still times out, retry once on the same network and share this full error including the NSURLError code."
            case .audioUploadTooLarge:
                "Record a shorter voice note and retry."
            }
        }

        var shortMessage: String {
            switch self {
            case .missingServerURL:
                "Backend URL is invalid"
            case let .invalidResponse(status, _):
                "Backend returned HTTP \(status)"
            case .decodeFailed:
                "Backend response did not match the app"
            case .transportFailed:
                "Backend network check failed"
            case .audioUploadTooLarge:
                "Voice note exceeds 25 MB"
            }
        }
    }

    var baseURL: URL?
    var apiToken: String

    func checkHealth() async throws -> HealthResponse {
        let url = try Self.endpointURL(baseURL: effectiveBaseURL(), path: "/v1/health")
        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        request.timeoutInterval = RequestTimeout.health
        let (data, urlResponse) = try await perform(request)
        let statusCode = (urlResponse as? HTTPURLResponse)?.statusCode ?? 500
        guard statusCode < 300 else {
            throw APIError.invalidResponse(status: statusCode, body: responseBody(data))
        }
        do {
            return try JSONDecoder.antirot.decode(HealthResponse.self, from: data)
        } catch {
            throw APIError.decodeFailed(body: responseBody(data))
        }
    }

    func registerDevice(_ request: DeviceRegistrationRequest) async throws -> DeviceRegistrationResponse {
        try await send(
            path: "/v1/devices/register",
            method: "POST",
            body: request,
            response: DeviceRegistrationResponse.self
        )
    }

    func signInWithGoogle(_ request: GoogleAuthRequest) async throws -> GoogleAuthResponse {
        try logRequest(method: "POST", path: "/v1/auth/google", includeAuth: false)
        return try await send(
            path: "/v1/auth/google",
            method: "POST",
            body: request,
            response: GoogleAuthResponse.self,
            includeAuth: false
        )
    }

    func claimPairing(_ request: PairingClaimRequest) async throws -> PairingClaimResponse {
        try await send(
            path: "/v1/pairing/claim",
            method: "POST",
            body: request,
            response: PairingClaimResponse.self
        )
    }

    func fetchPendingAlarms(deviceId: String) async throws -> PendingAlarmsResponse {
        let baseURL = effectiveBaseURL()
        var components = URLComponents(url: try Self.endpointURL(baseURL: baseURL, path: "/v1/alarms/pending"), resolvingAgainstBaseURL: false)
        components?.queryItems = [
            URLQueryItem(name: "deviceId", value: deviceId),
            URLQueryItem(name: "reconcile", value: "true"),
            URLQueryItem(name: "limit", value: "200")
        ]
        guard let url = components?.url else { throw APIError.missingServerURL }
        var request = URLRequest(url: url)
        addAuth(to: &request)
        let (data, response) = try await perform(request)
        let statusCode = (response as? HTTPURLResponse)?.statusCode ?? 500
        guard statusCode < 300 else {
            throw APIError.invalidResponse(status: statusCode, body: responseBody(data))
        }
        do {
            return try JSONDecoder.antirot.decode(PendingAlarmsResponse.self, from: data)
        } catch {
            throw APIError.decodeFailed(body: responseBody(data))
        }
    }

    func reconcileAlarms(_ request: AlarmReconcileRequest) async throws -> AlarmReconcileResponse {
        try await send(
            path: "/v1/alarms/reconcile",
            method: "POST",
            body: request,
            response: AlarmReconcileResponse.self
        )
    }

    func acknowledge(alarmId: String, deviceId: String, action: String, minutes: Int? = nil) async throws -> AlarmActionResponse {
        let payload = AlarmActionRequest(deviceId: deviceId, action: action, at: Date(), minutes: minutes)
        return try await send(
            path: "/v1/alarms/\(alarmId)/\(action)",
            method: "POST",
            body: payload,
            response: AlarmActionResponse.self
        )
    }

    func chat(message: String, requestId: String = UUID().uuidString) async throws -> ChatCoachResponse {
        try await send(
            path: "/v1/chat",
            method: "POST",
            body: ChatCoachRequest(message: message, requestId: requestId),
            response: ChatCoachResponse.self,
            timeoutInterval: RequestTimeout.chat
        )
    }

    func fetchRuntimeState(deviceId: String) async throws -> RuntimeStateResponse {
        let baseURL = effectiveBaseURL()
        var request = URLRequest(url: try Self.endpointURL(baseURL: baseURL, path: "/v1/state"))
        addAuth(to: &request)
        let (data, response) = try await perform(request)
        let statusCode = (response as? HTTPURLResponse)?.statusCode ?? 500
        guard statusCode < 300 else {
            throw APIError.invalidResponse(status: statusCode, body: responseBody(data))
        }
        do {
            return try JSONDecoder.antirot.decode(RuntimeStateResponse.self, from: data)
        } catch {
            throw APIError.decodeFailed(body: responseBody(data))
        }
    }

    func saveOnboardingProfile(name: String, timezone: String) async throws -> OnboardingProfileResponse {
        try await send(
            path: "/v1/profile/onboarding",
            method: "POST",
            body: OnboardingProfileRequest(name: name, timezone: timezone),
            response: OnboardingProfileResponse.self
        )
    }

    func fetchStats() async throws -> StatsResponse {
        let baseURL = effectiveBaseURL()
        var request = URLRequest(url: try Self.endpointURL(baseURL: baseURL, path: "/v1/stats"))
        request.httpMethod = "GET"
        request.timeoutInterval = RequestTimeout.standard
        addAuth(to: &request)
        logPreparedRequest(request, includeAuth: true)
        let (data, response) = try await perform(request)
        let statusCode = (response as? HTTPURLResponse)?.statusCode ?? 500
        guard statusCode < 300 else {
            throw APIError.invalidResponse(status: statusCode, body: responseBody(data))
        }
        do {
            return try JSONDecoder.antirot.decode(StatsResponse.self, from: data)
        } catch {
            throw APIError.decodeFailed(body: responseBody(data))
        }
    }

    func fetchMemory(key: String) async throws -> MemoryResponse {
        let baseURL = effectiveBaseURL()
        let encodedKey = key.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? key
        var request = URLRequest(url: try Self.endpointURL(baseURL: baseURL, path: "/v1/memory/\(encodedKey)"))
        request.httpMethod = "GET"
        request.timeoutInterval = RequestTimeout.standard
        addAuth(to: &request)
        logPreparedRequest(request, includeAuth: true)
        let (data, response) = try await perform(request)
        let statusCode = (response as? HTTPURLResponse)?.statusCode ?? 500
        guard statusCode < 300 else {
            throw APIError.invalidResponse(status: statusCode, body: responseBody(data))
        }
        do {
            return try JSONDecoder.antirot.decode(MemoryResponse.self, from: data)
        } catch {
            throw APIError.decodeFailed(body: responseBody(data))
        }
    }

    func createMemorySnapshot(_ request: CreateMemorySnapshotRequest) async throws -> CreateMemorySnapshotResponse {
        try await send(
            path: "/v1/memory/snapshots",
            method: "POST",
            body: request,
            response: CreateMemorySnapshotResponse.self
        )
    }

    func fetchMemorySnapshots() async throws -> ListMemorySnapshotsResponse {
        let baseURL = effectiveBaseURL()
        var request = URLRequest(url: try Self.endpointURL(baseURL: baseURL, path: "/v1/memory/snapshots"))
        request.httpMethod = "GET"
        request.timeoutInterval = RequestTimeout.standard
        addAuth(to: &request)
        logPreparedRequest(request, includeAuth: true)
        let (data, response) = try await perform(request)
        let statusCode = (response as? HTTPURLResponse)?.statusCode ?? 500
        guard statusCode < 300 else {
            throw APIError.invalidResponse(status: statusCode, body: responseBody(data))
        }
        do {
            return try JSONDecoder.antirot.decode(ListMemorySnapshotsResponse.self, from: data)
        } catch {
            throw APIError.decodeFailed(body: responseBody(data))
        }
    }

    func restoreMemorySnapshot(id: String, restoreRuntimeState: Bool = true) async throws -> RestoreMemorySnapshotResponse {
        let encodedId = id.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? id
        return try await send(
            path: "/v1/memory/snapshots/\(encodedId)/restore",
            method: "POST",
            body: RestoreMemorySnapshotRequest(restoreRuntimeState: restoreRuntimeState),
            response: RestoreMemorySnapshotResponse.self
        )
    }

    func createReport(_ request: CreateReportRequest) async throws -> CreateReportResponse {
        try await send(
            path: "/v1/reports",
            method: "POST",
            body: request,
            response: CreateReportResponse.self
        )
    }

    func transcribeAudio(fileURL: URL) async throws -> SpeechTranscriptionResponse {
        let baseURL = effectiveBaseURL()
        let boundary = "Boundary-\(UUID().uuidString)"
        var request = URLRequest(url: try Self.endpointURL(baseURL: baseURL, path: "/v1/speech/transcribe"))
        request.httpMethod = "POST"
        request.timeoutInterval = RequestTimeout.speechTranscription
        request.setValue("multipart/form-data; boundary=\(boundary)", forHTTPHeaderField: "Content-Type")
        addAuth(to: &request)

        let fileSize = try fileURL.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? 0
        guard fileSize <= Self.maxAudioUploadBytes else {
            throw APIError.audioUploadTooLarge(bytes: fileSize, max: Self.maxAudioUploadBytes)
        }
        let audioData = try Data(contentsOf: fileURL)
        var body = Data()
        body.appendMultipartFieldStart(
            boundary: boundary,
            name: "file",
            fileName: fileURL.lastPathComponent,
            contentType: "audio/mp4"
        )
        body.append(audioData)
        body.appendString("\r\n--\(boundary)--\r\n")
        request.httpBody = body

        let (data, urlResponse) = try await perform(request)
        let statusCode = (urlResponse as? HTTPURLResponse)?.statusCode ?? 500
        guard statusCode < 300 else {
            throw APIError.invalidResponse(status: statusCode, body: responseBody(data))
        }
        do {
            return try JSONDecoder.antirot.decode(SpeechTranscriptionResponse.self, from: data)
        } catch {
            throw APIError.decodeFailed(body: responseBody(data))
        }
    }

    func synthesizeSpeech(text: String) async throws -> SpeechSynthesisResponse {
        try await send(
            path: "/v1/speech/synthesize",
            method: "POST",
            body: SpeechSynthesisRequest(text: text, voiceId: nil),
            response: SpeechSynthesisResponse.self
        )
    }

    private func send<RequestBody: Encodable, ResponseBody: Decodable>(
        path: String,
        method: String,
        body: RequestBody,
        response: ResponseBody.Type,
        includeAuth: Bool = true,
        timeoutInterval: TimeInterval = RequestTimeout.standard
    ) async throws -> ResponseBody {
        let baseURL = effectiveBaseURL()
        var request = URLRequest(url: try Self.endpointURL(baseURL: baseURL, path: path))
        request.httpMethod = method
        request.timeoutInterval = timeoutInterval
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        if includeAuth {
            addAuth(to: &request)
        }
        request.httpBody = try JSONEncoder.antirot.encode(body)
        logPreparedRequest(request, includeAuth: includeAuth)
        let (data, urlResponse) = try await perform(request)
        let statusCode = (urlResponse as? HTTPURLResponse)?.statusCode ?? 500
        guard statusCode < 300 else {
            throw APIError.invalidResponse(status: statusCode, body: responseBody(data))
        }
        let responseData = ResponseBody.self == EmptyResponse.self && data.isEmpty
            ? Data("{}".utf8)
            : data
        do {
            return try JSONDecoder.antirot.decode(ResponseBody.self, from: responseData)
        } catch {
            throw APIError.decodeFailed(body: responseBody(data))
        }
    }

    private func addAuth(to request: inout URLRequest) {
        guard !apiToken.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }
        request.setValue("Bearer \(apiToken)", forHTTPHeaderField: "Authorization")
    }

    private func logRequest(method: String, path: String, includeAuth: Bool) throws {
        let url = try Self.endpointURL(baseURL: effectiveBaseURL(), path: path)
        print("Antirot API request preparing method=\(method) url=\(url.absoluteString) includeAuth=\(includeAuth)")
    }

    private func logPreparedRequest(_ request: URLRequest, includeAuth: Bool) {
        let method = request.httpMethod ?? "<missing>"
        let url = request.url?.absoluteString ?? "<missing>"
        print("Antirot API request method=\(method) url=\(url) timeout=\(Int(request.timeoutInterval))s includeAuth=\(includeAuth)")
    }

    private func perform(_ request: URLRequest) async throws -> (Data, URLResponse) {
        do {
            let (data, response) = try await URLSession.shared.data(for: request)
            if let httpResponse = response as? HTTPURLResponse {
                print("Antirot API response status=\(httpResponse.statusCode) url=\(request.url?.absoluteString ?? "<missing>")")
            }
            return (data, response)
        } catch {
            print("Antirot API transport failure url=\(request.url?.absoluteString ?? "<missing>") error=\(Self.transportFailureDescription(error))")
            throw APIError.transportFailed(
                url: request.url?.absoluteString ?? "<missing URL>",
                underlying: Self.transportFailureDescription(error)
            )
        }
    }

    private func effectiveBaseURL() -> URL {
        baseURL ?? URL(string: SettingsStore.defaultServerURL)!
    }

    static func endpointURL(baseURL: URL, path: String) throws -> URL {
        guard var components = URLComponents(url: baseURL, resolvingAgainstBaseURL: false) else {
            throw APIError.missingServerURL
        }
        let basePath = components.path.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        let routePath = path.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        components.path = "/" + [basePath, routePath]
            .filter { !$0.isEmpty }
            .joined(separator: "/")
        guard let url = components.url else {
            throw APIError.missingServerURL
        }
        return url
    }

    static func transportFailureDescription(_ error: Error) -> String {
        let nsError = error as NSError
        var parts = [error.localizedDescription, "\(nsError.domain) \(nsError.code)"]
        if let urlError = error as? URLError {
            if urlError.code == .timedOut {
                parts[0] = "The request timed out before the backend replied. This can happen when the coach LLM is still thinking or the current network is slow."
            }
            parts.append("URLError \(urlError.code.rawValue)")
        }
        if let failingURL = nsError.userInfo[NSURLErrorFailingURLErrorKey] as? URL {
            parts.append("failingURL=\(failingURL.absoluteString)")
        }
        return parts.joined(separator: " | ")
    }

    private func responseBody(_ data: Data) -> String {
        let text = String(data: data, encoding: .utf8) ?? "<non-utf8 response>"
        return text.isEmpty ? "<empty response>" : String(text.prefix(300))
    }
}

@MainActor
enum AlarmActionReconciler {
    static func reconcile() async {
        let settings = SettingsStore()
        guard !settings.apiToken.isEmpty else { return }
        let center = AlarmCenter()
        await center.configure(settings: settings)
        await center.reconcileAlarms()
    }
}

struct EmptyResponse: Codable {}

private extension Data {
    mutating func appendString(_ value: String) {
        append(Data(value.utf8))
    }

    mutating func appendMultipartFieldStart(
        boundary: String,
        name: String,
        fileName: String,
        contentType: String
    ) {
        appendString("--\(boundary)\r\n")
        appendString("Content-Disposition: form-data; name=\"\(name)\"; filename=\"\(fileName)\"\r\n")
        appendString("Content-Type: \(contentType)\r\n\r\n")
    }
}

extension JSONDecoder {
    static var antirot: JSONDecoder {
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        return decoder
    }
}

extension JSONEncoder {
    static var antirot: JSONEncoder {
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        return encoder
    }
}
