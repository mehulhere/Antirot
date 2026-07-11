import SwiftUI

enum CoachStageLayoutMetrics {
    static let backgroundAssetName = "AntirotCoachEditorial"
    static let imageVerticalPositionFraction: CGFloat = 0.52
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
                            Color.arDeepBg.opacity(0.32),
                            Color.clear,
                            Color.clear,
                            Color.arDeepBg.opacity(0.52)
                        ],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                    .allowsHitTesting(false)
                }
        }
        .ignoresSafeArea()
        .accessibilityHidden(true)
    }
}
