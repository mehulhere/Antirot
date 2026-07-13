import Foundation
import UIKit
import UserNotifications

@MainActor
final class AlarmCenter: ObservableObject {
    @Published var notificationStatus: UNAuthorizationStatus = .notDetermined
    @Published var alarmKitStatus: String = "unknown"
    @Published var scheduledAlarms: [AlarmJob] = []
    @Published var lastMessage: String = "No alarms scheduled"
    @Published var lastErrorDetails: String?

    private var settings: SettingsStore?
    private var localAdapterIds: [String: String] = [:]

    var nextReminderAlarms: [AlarmJob] {
        var nextByReminder: [String: AlarmJob] = [:]
        for alarm in scheduledAlarms {
            let key = reminderKey(for: alarm)
            if let current = nextByReminder[key], current.fireAt <= alarm.fireAt {
                continue
            }
            nextByReminder[key] = alarm
        }
        return nextByReminder.values.sorted { $0.fireAt < $1.fireAt }
    }

    func configure(settings: SettingsStore) async {
        self.settings = settings
        await refreshAuthorizationStatus()
        alarmKitStatus = AlarmKitCenter.authorizationLabel()
        RemoteNotificationRegistrar.register()
    }

    func requestNotificationPermission() async {
        do {
            let granted = try await UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .badge, .sound])
            await refreshAuthorizationStatus()
            if granted {
                RemoteNotificationRegistrar.register()
            }
            lastMessage = granted ? "Notification permission granted" : "Notification permission denied"
        } catch {
            recordError("Notification permission failed", error)
        }
    }

    func requestAlarmKitPermission() async {
        alarmKitStatus = await AlarmKitCenter.requestAuthorization()
        lastMessage = alarmKitStatus
    }

    func registerDevice() async {
        guard let settings else { return }
        let client = APIClient(baseURL: settings.baseURL, apiToken: settings.apiToken)
        do {
            let response = try await client.registerDevice(DeviceRegistrationRequest(
                deviceId: settings.deviceId,
                platform: "ios",
                appVersion: Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "0.1.0",
                notificationCapability: notificationCapability,
                usageCapability: await ScreenTimeCenter.currentCapability(),
                pushProvider: settings.pushToken.isEmpty ? nil : "apns",
                pushToken: settings.pushToken.isEmpty ? nil : settings.pushToken
            ))
            if let deviceToken = response.deviceToken {
                settings.apiToken = deviceToken
            }
            settings.registered = response.ok
            settings.statusMessage = response.message ?? "Registered as \(response.deviceId)"
            lastMessage = settings.statusMessage
            lastErrorDetails = nil
        } catch {
            settings.statusMessage = "Registration failed"
            lastMessage = settings.statusMessage
            recordError("Registration failed", error)
        }
    }

    func pollPendingAlarms() async {
        guard let settings else { return }
        let client = APIClient(baseURL: settings.baseURL, apiToken: settings.apiToken)
        do {
            let response = try await client.fetchPendingAlarms(deviceId: settings.deviceId)
            let cancellationResult = await cancelObsoleteSeries(response.cancellations)
            var confirmations: [ScheduledAlarmConfirmation] = []
            for alarm in response.alarms {
                let localAlarmId = try await schedule(alarm)
                if let deliveryToken = alarm.deliveryToken {
                    confirmations.append(ScheduledAlarmConfirmation(
                        alarmId: alarm.id,
                        deliveryToken: deliveryToken,
                        localAlarmId: localAlarmId
                    ))
                }
            }
            _ = try await client.reconcileAlarms(AlarmReconcileRequest(
                deviceId: settings.deviceId,
                scheduled: confirmations,
                cancelledSeriesIds: cancellationResult.confirmedSeriesIds
            ))
            if cancellationResult.failedLocalIds.isEmpty {
                lastMessage = response.alarms.isEmpty ? "Alarms reconciled" : "Scheduled \(response.alarms.count) alarm(s)"
                lastErrorDetails = nil
            } else {
                lastMessage = "Some obsolete alarms could not be cancelled; reconciliation will retry"
                lastErrorDetails = "🔴 FALLBACK: local alarm cancellation failed - Reason: required adapter rejected \(cancellationResult.failedLocalIds.joined(separator: ", ")) - Impact: tombstone remains unconfirmed and will retry"
            }
        } catch {
            recordError("Poll failed", error)
        }
    }

    func reconcileAlarms() async {
        await pollPendingAlarms()
    }

    private func cancelObsoleteSeries(_ tombstones: [AlarmCancellationTombstone]) async -> CancellationResult {
        var confirmedSeriesIds: [String] = []
        var failedLocalIds: [String] = []
        for tombstone in tombstones {
            var succeeded = true
            for localAlarmId in tombstone.localAlarmIds {
                if !(await cancelLocalAlarm(localAlarmId)) {
                    succeeded = false
                    failedLocalIds.append(localAlarmId)
                }
            }
            if succeeded {
                confirmedSeriesIds.append(tombstone.seriesId)
                scheduledAlarms.removeAll { $0.seriesId == tombstone.seriesId }
                localAdapterIds = localAdapterIds.filter { _, localAdapterId in
                    !tombstone.localAlarmIds.contains(localAdapterId)
                }
            }
        }
        return CancellationResult(
            confirmedSeriesIds: confirmedSeriesIds,
            failedLocalIds: failedLocalIds
        )
    }

    private func cancelLocalAlarm(_ localAlarmId: String) async -> Bool {
        if localAlarmId.hasPrefix("alarmkit:") {
            return await AlarmKitCenter.cancel(
                alarmId: String(localAlarmId.dropFirst("alarmkit:".count))
            )
        }
        if localAlarmId.hasPrefix("notification:") {
            let alarmId = String(localAlarmId.dropFirst("notification:".count))
            UNUserNotificationCenter.current().removePendingNotificationRequests(withIdentifiers: [alarmId])
            UNUserNotificationCenter.current().removeDeliveredNotifications(withIdentifiers: [alarmId])
            return true
        }
        return false
    }

    func scheduleTestAlarm(severity: AlarmJob.Severity) async {
        do {
            _ = try await schedule(.test(severity: severity))
        } catch {
            recordError("Test alarm failed", error)
        }
    }

    func schedule(_ alarm: AlarmJob) async throws -> String {
        if let localAdapterId = localAdapterIds[alarm.id] {
            return localAdapterId
        }

        let soundChoice = alarmSoundChoice(for: alarm.severity)
        let soundName = soundChoice.name
        let scheduledWithAlarmKit = try await AlarmKitCenter.schedule(alarm, soundName: soundName)
        if scheduledWithAlarmKit {
            scheduledAlarms.append(alarm)
            let widgetUpdated = writeCurrentTaskSnapshot(for: alarm)
            alarmKitStatus = AlarmKitCenter.authorizationLabel()
            let soundMessage = "Real AlarmKit alarm scheduled with \(soundChoice.label)"
            lastMessage = widgetUpdated ? soundMessage : "\(soundMessage). Widget app-group storage unavailable."
            let localAdapterId = "alarmkit:\(alarm.id)"
            localAdapterIds[alarm.id] = localAdapterId
            return localAdapterId
        }

        let content = UNMutableNotificationContent()
        content.title = alarm.title
        content.body = alarm.message
        content.sound = UNNotificationSound(named: UNNotificationSoundName(rawValue: soundName))
        content.categoryIdentifier = AlarmNotificationActions.categoryIdentifier
        content.userInfo = ["alarmId": alarm.id]

        let delay = max(1, alarm.fireAt.timeIntervalSinceNow)
        let trigger = UNTimeIntervalNotificationTrigger(timeInterval: delay, repeats: false)
        let request = UNNotificationRequest(identifier: alarm.id, content: content, trigger: trigger)
        try await UNUserNotificationCenter.current().add(request)
        scheduledAlarms.append(alarm)
        let localAdapterId = "notification:\(alarm.id)"
        localAdapterIds[alarm.id] = localAdapterId
        let widgetUpdated = writeCurrentTaskSnapshot(for: alarm)
        let scheduleMessage = "AlarmKit unavailable; scheduled notification fallback with \(soundChoice.label)"
        lastMessage = widgetUpdated ? scheduleMessage : "\(scheduleMessage). Widget app-group storage unavailable."
        return localAdapterId
    }

    func refreshAuthorizationStatus() async {
        let settings = await UNUserNotificationCenter.current().notificationSettings()
        notificationStatus = settings.authorizationStatus
    }

    private var notificationCapability: String {
        switch notificationStatus {
        case .authorized, .provisional, .ephemeral:
            settings?.pushToken.isEmpty == false ? "remote_notification" : "notification"
        default:
            "none"
        }
    }

    private func alarmSoundChoice(for severity: AlarmJob.Severity) -> (name: String, label: String) {
        guard let settings else {
            return (bundledSoundName(for: severity), "bundled sound")
        }

        let mode = AlarmSoundMode(storedValue: settings.alarmSoundMode)
        switch mode {
        case .automatic:
            return (bundledSoundName(for: severity), "automatic bundled sound")
        case .bundledNormal:
            return ("antirot-normal.wav", "bundled normal sound")
        case .bundledLoud:
            return ("antirot-loud.wav", "bundled loud sound")
        case .custom:
            if let customName = settings.alarmSoundName.nilIfBlank {
                return (customName, "custom sound")
            }
            return (bundledSoundName(for: severity), "automatic bundled sound because no custom sound is imported")
        }
    }

    private func bundledSoundName(for severity: AlarmJob.Severity) -> String {
        switch severity {
        case .normal:
            "antirot-normal.wav"
        case .loud, .urgent:
            "antirot-loud.wav"
        }
    }

    private func reminderKey(for alarm: AlarmJob) -> String {
        [
            alarm.kind.rawValue,
            alarm.title,
            alarm.message
        ].joined(separator: "|")
    }

    private func writeCurrentTaskSnapshot(for alarm: AlarmJob) -> Bool {
        let subtitle = switch alarm.kind {
        case .normalWake, .loudWake:
            "Wake up. Day does not start by negotiating with the pillow."
        case .routineOverdue:
            "Routine window is over. Come back."
        case .sessionOverdue, .sessionAlarm, .breakAlarm, .wakeAlarm, .idleAlarm:
            alarm.message
        case .nonResponse:
            "You vanished. Fix that."
        case .test:
            "Test alarm scheduled. Nothing heroic yet."
        case .unknown:
            alarm.message
        }
        return SharedTaskStore.write(CurrentTaskSnapshot(
            title: alarm.title,
            subtitle: subtitle,
            mode: alarm.kind.rawValue,
            dueAt: alarm.fireAt
        ))
    }

    private func recordError(_ summary: String, _ error: Error) {
        lastMessage = summary
        lastErrorDetails = [
            summary,
            error.localizedDescription,
            String(describing: error)
        ].joined(separator: "\n\n")
    }
}

private struct CancellationResult {
    var confirmedSeriesIds: [String]
    var failedLocalIds: [String]
}

private extension String {
    var nilIfBlank: String? {
        let trimmed = trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }
}
