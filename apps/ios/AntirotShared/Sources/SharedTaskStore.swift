import Foundation
import WidgetKit

public struct CurrentTaskSnapshot: Codable, Equatable {
    public var title: String
    public var subtitle: String
    public var mode: String
    public var updatedAt: Date
    public var dueAt: Date?

    public init(
        title: String,
        subtitle: String,
        mode: String,
        updatedAt: Date = Date(),
        dueAt: Date? = nil
    ) {
        self.title = title
        self.subtitle = subtitle
        self.mode = mode
        self.updatedAt = updatedAt
        self.dueAt = dueAt
    }

    public static let empty = CurrentTaskSnapshot(
        title: "No active task",
        subtitle: "Open Antirot when you are ready to work.",
        mode: "idle"
    )
}

public enum SharedTaskStore {
    public static let appGroupId = "group.com.mehulhere.Antirot"
    private static let currentTaskKey = "currentTaskSnapshot"

    public static func read() -> CurrentTaskSnapshot {
        guard
            let defaults = UserDefaults(suiteName: appGroupId),
            let data = defaults.data(forKey: currentTaskKey),
            let snapshot = try? JSONDecoder().decode(CurrentTaskSnapshot.self, from: data)
        else {
            return .empty
        }
        return snapshot
    }

    public static func write(_ snapshot: CurrentTaskSnapshot) {
        guard
            let defaults = UserDefaults(suiteName: appGroupId),
            let data = try? JSONEncoder().encode(snapshot)
        else {
            return
        }
        defaults.set(data, forKey: currentTaskKey)
        if #available(iOS 14.0, *) {
            WidgetCenter.shared.reloadTimelines(ofKind: "AntirotCurrentTaskWidget")
        }
    }
}
