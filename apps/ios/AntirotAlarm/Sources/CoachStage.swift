import SwiftUI

enum CoachStageLayoutMetrics {
    static let backgroundAssetName = "AntirotCoachStage"
    static let imageVerticalPositionFraction: CGFloat = 0.50
}

// MARK: - Coach Stage

/// Fixed cinematic background for the Coach home screen. The character, halo,
/// spotlight, and red floor glow are baked into the image so the visible home
/// layout matches the reference instead of drifting through separate layers.
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
                            Color.black.opacity(0.12),
                            Color.clear,
                            Color.arBg.opacity(0.10)
                        ],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                    .allowsHitTesting(false)
                }
        }
        .ignoresSafeArea()
        .accessibilityLabel("Antirot coach")
    }
}
