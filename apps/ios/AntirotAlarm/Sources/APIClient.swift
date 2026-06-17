import Foundation

struct APIClient {
    enum APIError: Error, LocalizedError {
        case missingServerURL
        case invalidResponse(status: Int, body: String)
        case decodeFailed(body: String)

        var errorDescription: String? {
            switch self {
            case .missingServerURL:
                "Backend URL is invalid. Open Developer Settings and reset it to api.antirot.org."
            case let .invalidResponse(status, body):
                "Backend returned HTTP \(status): \(body)"
            case let .decodeFailed(body):
                "Backend returned unexpected JSON: \(body)"
            }
        }
    }

    var baseURL: URL?
    var apiToken: String

    func registerDevice(_ request: DeviceRegistrationRequest) async throws -> DeviceRegistrationResponse {
        try await send(
            path: "/devices/register",
            method: "POST",
            body: request,
            response: DeviceRegistrationResponse.self
        )
    }

    func signInWithGoogle(_ request: GoogleAuthRequest) async throws -> GoogleAuthResponse {
        try await send(
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

    func fetchPendingAlarms(deviceId: String) async throws -> [AlarmJob] {
        let baseURL = effectiveBaseURL()
        var components = URLComponents(url: baseURL.appendingPathComponent("/alarms/pending"), resolvingAgainstBaseURL: false)
        components?.queryItems = [URLQueryItem(name: "deviceId", value: deviceId)]
        guard let url = components?.url else { throw APIError.missingServerURL }
        var request = URLRequest(url: url)
        addAuth(to: &request)
        let (data, response) = try await URLSession.shared.data(for: request)
        let statusCode = (response as? HTTPURLResponse)?.statusCode ?? 500
        guard statusCode < 300 else {
            throw APIError.invalidResponse(status: statusCode, body: responseBody(data))
        }
        do {
            return try JSONDecoder.antirot.decode([AlarmJob].self, from: data)
        } catch {
            throw APIError.decodeFailed(body: responseBody(data))
        }
    }

    func acknowledge(alarmId: String, deviceId: String, action: String, minutes: Int? = nil) async throws {
        let payload = AlarmActionRequest(deviceId: deviceId, action: action, at: Date(), minutes: minutes)
        _ = try await send(
            path: "/alarms/\(alarmId)/\(action)",
            method: "POST",
            body: payload,
            response: EmptyResponse.self
        )
    }

    func chat(message: String) async throws -> ChatCoachResponse {
        try await send(
            path: "/v1/chat",
            method: "POST",
            body: ChatCoachRequest(message: message),
            response: ChatCoachResponse.self
        )
    }

    func transcribeAudio(fileURL: URL) async throws -> SpeechTranscriptionResponse {
        let baseURL = effectiveBaseURL()
        let boundary = "Boundary-\(UUID().uuidString)"
        var request = URLRequest(url: baseURL.appendingPathComponent("/v1/speech/transcribe"))
        request.httpMethod = "POST"
        request.timeoutInterval = 60
        request.setValue("multipart/form-data; boundary=\(boundary)", forHTTPHeaderField: "Content-Type")
        addAuth(to: &request)

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

        let (data, urlResponse) = try await URLSession.shared.data(for: request)
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
        includeAuth: Bool = true
    ) async throws -> ResponseBody {
        let baseURL = effectiveBaseURL()
        var request = URLRequest(url: baseURL.appendingPathComponent(path))
        request.httpMethod = method
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        if includeAuth {
            addAuth(to: &request)
        }
        request.httpBody = try JSONEncoder.antirot.encode(body)
        let (data, urlResponse) = try await URLSession.shared.data(for: request)
        let statusCode = (urlResponse as? HTTPURLResponse)?.statusCode ?? 500
        guard statusCode < 300 else {
            throw APIError.invalidResponse(status: statusCode, body: responseBody(data))
        }
        if ResponseBody.self == EmptyResponse.self {
            return EmptyResponse() as! ResponseBody
        }
        do {
            return try JSONDecoder.antirot.decode(ResponseBody.self, from: data)
        } catch {
            throw APIError.decodeFailed(body: responseBody(data))
        }
    }

    private func addAuth(to request: inout URLRequest) {
        guard !apiToken.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }
        request.setValue("Bearer \(apiToken)", forHTTPHeaderField: "Authorization")
    }

    private func effectiveBaseURL() -> URL {
        baseURL ?? URL(string: SettingsStore.defaultServerURL)!
    }

    private func responseBody(_ data: Data) -> String {
        let text = String(data: data, encoding: .utf8) ?? "<non-utf8 response>"
        return text.isEmpty ? "<empty response>" : String(text.prefix(300))
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
