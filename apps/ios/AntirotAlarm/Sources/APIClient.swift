import Foundation

struct APIClient {
    enum APIError: Error, LocalizedError {
        case missingServerURL
        case invalidResponse

        var errorDescription: String? {
            switch self {
            case .missingServerURL:
                "Set the Antirot VPS URL first."
            case .invalidResponse:
                "The Antirot server returned an invalid response."
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

    func fetchPendingAlarms(deviceId: String) async throws -> [AlarmJob] {
        guard let baseURL else { throw APIError.missingServerURL }
        var components = URLComponents(url: baseURL.appendingPathComponent("/alarms/pending"), resolvingAgainstBaseURL: false)
        components?.queryItems = [URLQueryItem(name: "deviceId", value: deviceId)]
        guard let url = components?.url else { throw APIError.missingServerURL }
        var request = URLRequest(url: url)
        addAuth(to: &request)
        let (data, response) = try await URLSession.shared.data(for: request)
        guard (response as? HTTPURLResponse)?.statusCode ?? 500 < 300 else {
            throw APIError.invalidResponse
        }
        return try JSONDecoder.antirot.decode([AlarmJob].self, from: data)
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

    private func send<RequestBody: Encodable, ResponseBody: Decodable>(
        path: String,
        method: String,
        body: RequestBody,
        response: ResponseBody.Type
    ) async throws -> ResponseBody {
        guard let baseURL else { throw APIError.missingServerURL }
        var request = URLRequest(url: baseURL.appendingPathComponent(path))
        request.httpMethod = method
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        addAuth(to: &request)
        request.httpBody = try JSONEncoder.antirot.encode(body)
        let (data, urlResponse) = try await URLSession.shared.data(for: request)
        guard (urlResponse as? HTTPURLResponse)?.statusCode ?? 500 < 300 else {
            throw APIError.invalidResponse
        }
        if ResponseBody.self == EmptyResponse.self {
            return EmptyResponse() as! ResponseBody
        }
        return try JSONDecoder.antirot.decode(ResponseBody.self, from: data)
    }

    private func addAuth(to request: inout URLRequest) {
        guard !apiToken.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }
        request.setValue("Bearer \(apiToken)", forHTTPHeaderField: "Authorization")
    }
}

struct EmptyResponse: Codable {}

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
