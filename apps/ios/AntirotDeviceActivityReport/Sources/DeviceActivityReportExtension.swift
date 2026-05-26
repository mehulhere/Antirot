import SwiftUI

#if canImport(DeviceActivity)
import DeviceActivity

@available(iOS 16.0, *)
struct DeviceActivityReportExtension: DeviceActivityReportScene {
    let context: DeviceActivityReport.Context = .init("AntirotUsage")

    let content: (DeviceActivityResults<DeviceActivityData>) -> AntirotUsageReportView

    init() {
        self.content = { results in
            AntirotUsageReportView(results: results)
        }
    }
}

@available(iOS 16.0, *)
struct AntirotUsageReportView: View {
    let results: DeviceActivityResults<DeviceActivityData>

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Recent Usage")
                .font(.headline)
            Text("iOS exposes Screen Time data through a privacy-preserving report extension. Antirot uses this as recent evidence, not a live activity feed.")
                .font(.footnote)
                .foregroundStyle(.secondary)
        }
        .padding()
    }
}
#endif
