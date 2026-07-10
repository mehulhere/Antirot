import SwiftUI

enum CoachStageLayoutMetrics {
    static let backgroundAssetName = "AntirotCoachStageWarm"
    static let imageVerticalPositionFraction: CGFloat = 0.50
}

// MARK: - Coach Stage

/// Fixed warm editorial background for the Coach home screen. The subject and
/// architectural depth are baked into the artwork while SwiftUI contrast
/// overlays keep status, actions, and chat readable across iPhone crops.
struct CoachStage: View {
    let emotion: CoachEmotion
    var isThinking: Bool = false

    var body: some View {
        GeometryReader { proxy in
            Image(CoachStageLayoutMetrics.backgroundAssetName)
                .resizable()
                .scaledToFill()
                .frame(width: proxy.size.width, height: proxy.size.height)
                .clipped()
                .overlay {
                    LinearGradient(
                        colors: [
                            Color.arDeepBg.opacity(0.58),
                            Color.black.opacity(0.08),
                            Color.clear,
                            Color.arDeepBg.opacity(0.70)
                        ],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                    .allowsHitTesting(false)
                }
                .overlay(alignment: .leading) {
                    LinearGradient(
                        colors: [Color.arDeepBg.opacity(0.34), .clear],
                        startPoint: .leading,
                        endPoint: .trailing
                    )
                    .frame(width: proxy.size.width * 0.62)
                    .allowsHitTesting(false)
                }
        }
        .ignoresSafeArea()
        .accessibilityLabel("Antirot coach")
    }
}
