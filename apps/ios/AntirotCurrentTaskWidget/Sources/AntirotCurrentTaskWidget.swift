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

    private var modeColor: Color {
        switch entry.snapshot.mode.lowercased() {
        case "working", "routine":
            return Color(red: 0.902, green: 0.224, blue: 0.275) // accent red
        case "idle":
            return Color(red: 0.486, green: 0.455, blue: 0.435) // warm muted stone
        default:
            return Color(red: 0.914, green: 0.604, blue: 0.286) // warm amber
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Text(entry.snapshot.mode.uppercased())
                    .font(.system(size: 9, weight: .bold))
                    .tracking(0.8)
                    .foregroundStyle(.white)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(Capsule().fill(modeColor))

                Spacer()

                Text(entry.snapshot.updatedAt, style: .time)
                    .font(.system(size: 9, weight: .medium))
                    .foregroundStyle(Color(red: 0.714, green: 0.678, blue: 0.651))
            }

            Text(entry.snapshot.title)
                .font(.system(size: 15, weight: .bold))
                .foregroundStyle(Color(red: 0.961, green: 0.945, blue: 0.925))
                .lineLimit(2)
                .minimumScaleFactor(0.8)

            Text(entry.snapshot.subtitle)
                .font(.system(size: 11))
                .foregroundStyle(Color(red: 0.714, green: 0.678, blue: 0.651))
                .lineLimit(2)

            Spacer(minLength: 0)

            if let dueAt = entry.snapshot.dueAt {
                HStack(spacing: 4) {
                    Image(systemName: "clock")
                        .font(.system(size: 9))
                    Text(dueAt, style: .relative)
                        .font(.system(size: 9, weight: .medium))
                }
                .foregroundStyle(modeColor)
            }
        }
        .containerBackground(for: .widget) {
            LinearGradient(
                colors: [
                    Color(red: 0.105, green: 0.093, blue: 0.082),
                    Color(red: 0.035, green: 0.031, blue: 0.028)
                ],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )
                .overlay(
                    LinearGradient(
                        colors: [Color.white.opacity(0.08), modeColor.opacity(0.10), .clear],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    )
                )
        }
    }
}

struct AntirotCurrentTaskWidget: Widget {
    let kind = "AntirotCurrentTaskWidget"

    var body: some WidgetConfiguration {
        StaticConfiguration(kind: kind, provider: CurrentTaskProvider()) { entry in
            CurrentTaskWidgetView(entry: entry)
        }
        .configurationDisplayName("Antirot Current Task")
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
