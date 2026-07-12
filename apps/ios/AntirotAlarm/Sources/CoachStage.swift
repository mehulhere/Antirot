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
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var motionPhase = false

    var body: some View {
        GeometryReader { proxy in
            Image(CoachStageLayoutMetrics.backgroundAssetName)
                .resizable()
                .scaledToFill()
                .scaleEffect(reduceMotion ? 1 : animatedScale)
                .offset(y: reduceMotion ? 0 : (motionPhase ? -2 : 2))
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
                .overlay(emotion.accentColor.opacity(isThinking ? 0.06 : 0.02))
        }
        .ignoresSafeArea()
        .accessibilityHidden(true)
        .onAppear {
            guard !reduceMotion else { return }
            withAnimation(.easeInOut(duration: isThinking ? 1.1 : 3.2).repeatForever(autoreverses: true)) {
                motionPhase = true
            }
        }
    }

    private var animatedScale: CGFloat {
        let base: CGFloat = emotion == .impatient || emotion == .strict ? 1.012 : 1.004
        return base + (motionPhase ? (isThinking ? 0.012 : 0.004) : 0)
    }
}
