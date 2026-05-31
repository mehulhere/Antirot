import Foundation
import GoogleSignIn
import UIKit

@MainActor
enum GoogleAuthCenter {
    static func signIn(settings: SettingsStore) async throws -> GoogleAuthResponse {
        guard let presentingViewController = UIApplication.shared.antirotRootViewController else {
            throw GoogleAuthError.missingPresenter
        }
        guard let clientID = Bundle.main.object(forInfoDictionaryKey: "GIDClientID") as? String else {
            throw GoogleAuthError.missingClientID
        }

        GIDSignIn.sharedInstance.configuration = GIDConfiguration(clientID: clientID)
        let result = try await GIDSignIn.sharedInstance.signIn(withPresenting: presentingViewController)
        guard let idToken = result.user.idToken?.tokenString else {
            throw GoogleAuthError.missingIdToken
        }

        let request = GoogleAuthRequest(
            idToken: idToken,
            deviceId: settings.deviceId,
            platform: "ios",
            appVersion: Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "unknown",
            notificationCapability: "local_notifications",
            usageCapability: await ScreenTimeCenter.currentCapability()
        )

        let response = try await APIClient(
            baseURL: settings.baseURL,
            apiToken: ""
        ).signInWithGoogle(request)
        settings.apiToken = response.deviceToken
        settings.registered = true
        settings.statusMessage = response.message
        return response
    }

    static func handle(url: URL) -> Bool {
        GIDSignIn.sharedInstance.handle(url)
    }
}

enum GoogleAuthError: LocalizedError {
    case missingClientID
    case missingPresenter
    case missingIdToken

    var errorDescription: String? {
        switch self {
        case .missingClientID:
            return "Google Sign-In is missing its iOS client ID."
        case .missingPresenter:
            return "Could not open Google Sign-In from this screen."
        case .missingIdToken:
            return "Google did not return an ID token."
        }
    }
}

private extension UIApplication {
    var antirotRootViewController: UIViewController? {
        connectedScenes
            .compactMap { $0 as? UIWindowScene }
            .flatMap(\.windows)
            .first { $0.isKeyWindow }?
            .rootViewController?
            .topPresentedViewController
    }
}

private extension UIViewController {
    var topPresentedViewController: UIViewController {
        presentedViewController?.topPresentedViewController ?? self
    }
}
