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

        let client = APIClient(
            baseURL: settings.baseURL,
            apiToken: ""
        )

        do {
            _ = try await client.checkHealth()
        } catch {
            throw GoogleAuthError.backendHealthCheckFailed(error.localizedDescription)
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
            notificationCapability: settings.pushToken.isEmpty ? "local_notifications" : "remote_notification",
            usageCapability: await ScreenTimeCenter.currentCapability(),
            pushProvider: settings.pushToken.isEmpty ? nil : "apns",
            pushToken: settings.pushToken.isEmpty ? nil : settings.pushToken
        )

        let response: GoogleAuthResponse
        do {
            response = try await client.signInWithGoogle(request)
        } catch {
            throw GoogleAuthError.backendSignInFailed(error.localizedDescription)
        }
        settings.apiToken = response.deviceToken
        settings.userId = response.userId
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
    case backendHealthCheckFailed(String)
    case backendSignInFailed(String)

    var errorDescription: String? {
        switch self {
        case .missingClientID:
            return "Google Sign-In is missing its iOS client ID."
        case .missingPresenter:
            return "Could not open Google Sign-In from this screen."
        case .missingIdToken:
            return "Google did not return an ID token."
        case let .backendHealthCheckFailed(details):
            return "Antirot backend health check failed before Google sign-in. \(details)"
        case let .backendSignInFailed(details):
            return "Google returned an ID token, but Antirot backend sign-in failed. \(details)"
        }
    }

    var recoverySuggestion: String? {
        switch self {
        case .missingClientID:
            return "Confirm GIDClientID is present in the generated iOS Info.plist."
        case .missingPresenter:
            return "Try again from the login screen after the app is fully open."
        case .missingIdToken:
            return "Try Google sign-in again. If it repeats, check the iOS OAuth client configuration."
        case .backendHealthCheckFailed:
            return "Open https://api.antirot.org/v1/health on the same iPhone, then share the full error details shown here."
        case .backendSignInFailed:
            return "The phone reached Google. Share the backend HTTP status or NSURLError code shown in the full error."
        }
    }

    var shortMessage: String {
        switch self {
        case .missingClientID:
            return "Google iOS client ID is missing"
        case .missingPresenter:
            return "Could not open Google sign-in"
        case .missingIdToken:
            return "Google did not return an ID token"
        case .backendHealthCheckFailed:
            return "Backend health check failed"
        case .backendSignInFailed:
            return "Backend Google sign-in failed"
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
