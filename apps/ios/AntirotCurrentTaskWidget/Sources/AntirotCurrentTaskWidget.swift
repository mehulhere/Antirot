import SwiftUI
import WidgetKit

struct CurrentTaskEntry: TimelineEntry {
    let date: Date
    let snapshot: CurrentTaskSnapshot
}

struct CurrentTaskProvider: TimelineProvider {
    func placeholder(in context: Context) -> CurrentTaskEntry {
        CurrentTaskEntry(
            date: Date(),
            snapshot: CurrentTaskSnapshot(
                title: "Fix auth bugs",
                subtitle: "You said this matters. Prove it.",
                mode: "working",
                dueAt: Date().addingTimeInterval(45 * 60)
            )
        )
    }

    func getSnapshot(in context: Context, completion: @escaping (CurrentTaskEntry) -> Void) {
        completion(CurrentTaskEntry(date: Date(), snapshot: SharedTaskStore.read()))
    }

    func getTimeline(in context: Context, completion: @escaping (Timeline<CurrentTaskEntry>) -> Void) {
        let entry = CurrentTaskEntry(date: Date(), snapshot: SharedTaskStore.read())
        let nextUpdate = Calendar.current.date(byAdding: .minute, value: 15, to: Date()) ?? Date().addingTimeInterval(900)
        completion(Timeline(entries: [entry], policy: .after(nextUpdate)))
    }
}

struct CurrentTaskWidgetView: View {
    let entry: CurrentTaskEntry

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(entry.snapshot.mode.uppercased())
                    .font(.caption2)
                    .fontWeight(.bold)
                    .foregroundStyle(.orange)
                Spacer()
                Text(entry.snapshot.updatedAt, style: .time)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }

            Text(entry.snapshot.title)
                .font(.headline)
                .lineLimit(3)
                .minimumScaleFactor(0.8)

            Text(entry.snapshot.subtitle)
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(3)

            if let dueAt = entry.snapshot.dueAt {
                Text("Check at \(dueAt, style: .time)")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
        }
        .containerBackground(.background, for: .widget)
    }
}

struct AntirotCurrentTaskWidget: Widget {
    let kind = "AntirotCurrentTaskWidget"

    var body: some WidgetConfiguration {
        StaticConfiguration(kind: kind, provider: CurrentTaskProvider()) { entry in
            CurrentTaskWidgetView(entry: entry)
        }
        .configurationDisplayName("Current Task")
        .description("Shows what Antirot expects you to be doing right now.")
        .supportedFamilies([.systemSmall, .systemMedium])
    }
}

@main
struct AntirotWidgetBundle: WidgetBundle {
    var body: some Widget {
        AntirotCurrentTaskWidget()
    }
}
