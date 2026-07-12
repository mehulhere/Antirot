import Foundation

#if canImport(FamilyControls)
import FamilyControls
#endif

enum ScreenTimeCenter {
    static func authorizationLabel() -> String {
        #if canImport(FamilyControls)
        if #available(iOS 16.0, *) {
            switch AuthorizationCenter.shared.authorizationStatus {
            case .approved: return "Authorized"
            case .denied: return "Denied"
            case .notDetermined: return "Not requested"
            @unknown default: return "Unknown"
            }
        }
        #endif
        return "Unavailable"
    }

    static func requestAuthorization() async -> String {
        #if canImport(FamilyControls)
        if #available(iOS 16.0, *) {
            do {
                try await AuthorizationCenter.shared.requestAuthorization(for: .individual)
                return "Screen Time authorized"
            } catch {
                return "Screen Time authorization failed: \(error.localizedDescription)"
            }
        }
        #endif
        return "Screen Time APIs are unavailable in this build"
    }

    static func currentCapability() async -> String {
        #if canImport(FamilyControls)
        if #available(iOS 16.0, *) {
            return AuthorizationCenter.shared.authorizationStatus == .approved ? "recent_summary" : "none"
        }
        #endif
        return "none"
    }
}
