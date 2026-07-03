import Foundation
import Security

protocol SecureTokenStoring: Sendable {
    func load() throws -> String
    func save(_ token: String) throws
    func clear() throws
}

struct SecureTokenStore: SecureTokenStoring {
    private let service = "org.antirot.ios.authentication"
    private let account = "device-token"

    func load() throws -> String {
        var query = baseQuery
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne

        var result: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        if status == errSecItemNotFound {
            return ""
        }
        guard status == errSecSuccess else {
            throw SecureTokenStoreError.keychain(status)
        }
        guard
            let data = result as? Data,
            let token = String(data: data, encoding: .utf8)
        else {
            throw SecureTokenStoreError.invalidTokenData
        }
        return token
    }

    func save(_ token: String) throws {
        if token.isEmpty {
            try clear()
            return
        }

        let data = Data(token.utf8)
        let attributes: [String: Any] = [
            kSecValueData as String: data,
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        ]
        let updateStatus = SecItemUpdate(baseQuery as CFDictionary, attributes as CFDictionary)
        if updateStatus == errSecSuccess {
            return
        }
        guard updateStatus == errSecItemNotFound else {
            throw SecureTokenStoreError.keychain(updateStatus)
        }

        var addQuery = baseQuery
        attributes.forEach { addQuery[$0.key] = $0.value }
        let addStatus = SecItemAdd(addQuery as CFDictionary, nil)
        guard addStatus == errSecSuccess else {
            throw SecureTokenStoreError.keychain(addStatus)
        }
    }

    func clear() throws {
        let status = SecItemDelete(baseQuery as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw SecureTokenStoreError.keychain(status)
        }
    }

    private var baseQuery: [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account
        ]
    }
}

enum SecureTokenStoreError: LocalizedError {
    case keychain(OSStatus)
    case invalidTokenData

    var errorDescription: String? {
        switch self {
        case let .keychain(status):
            let description = SecCopyErrorMessageString(status, nil) as String? ?? "Unknown Keychain error"
            return "Keychain operation failed (\(status)): \(description)"
        case .invalidTokenData:
            return "The authentication token stored in Keychain is invalid."
        }
    }
}
