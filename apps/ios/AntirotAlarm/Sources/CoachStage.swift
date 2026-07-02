import SwiftUI

// MARK: - Coach Stage

/// Full-screen, cinematic, stylized coach "presence".
///
/// The character is intentionally abstract: a sharp bust silhouette (head +
/// shoulders) with an expressive gaze and an ambient halo, not a cute mascot
/// and not uncanny realism. Emotion drives smoothly animated pose parameters
/// (gaze direction, eye narrowing, brow angle, head tilt, halo intensity, a
/// periodic clock-check glance) so transitions crossfade instead of hard-cut.
struct CoachStage: View {
    let emotion: CoachEmotion
    var isThinking: Bool = false

    @State private var glancing = false
    @State private var breathPhase: Double = 0

    private let glanceTimer = Timer.publish(every: 4.2, on: .main, in: .common).autoconnect()

    var body: some View {
        GeometryReader { proxy in
            let size = proxy.size
            let pose = resolvedPose

            ZStack {
                LinearGradient(
                    colors: [
                        Color(red: 0.012, green: 0.011, blue: 0.014),
                        Color.arBg,
                        Color(red: 0.025, green: 0.016, blue: 0.020)
                    ],
                    startPoint: .top,
                    endPoint: .bottom
                )

                Circle()
                    .fill(emotion.accentColor)
                    .frame(width: min(size.width, size.height) * 1.65, height: min(size.width, size.height) * 1.65)
                    .opacity(0.20 * pose.halo + 0.04)
                    .blur(radius: 82)
                    .offset(y: -size.height * 0.18)

                Ellipse()
                    .fill(Color.black.opacity(0.54))
                    .frame(width: size.width * 0.92, height: size.height * 0.16)
                    .blur(radius: 26)
                    .offset(y: size.height * 0.20)

                coachBust(size: size, pose: pose)

                VStack {
                    HStack {
                        StageStatusPill(emotion: emotion, isThinking: isThinking)
                        Spacer()
                    }
                    .padding(.top, 60)
                    .padding(.horizontal, 24)
                    Spacer()
                }

                LinearGradient(
                    colors: [Color.clear, Color.arBg.opacity(0.78)],
                    startPoint: .center,
                    endPoint: .bottom
                )
                .allowsHitTesting(false)
            }
        }
        .ignoresSafeArea()
        .onReceive(glanceTimer) { _ in
            guard emotion == .watching, !isThinking else { return }
            withAnimation(.easeInOut(duration: 0.6)) {
                glancing.toggle()
            }
        }
        .animation(.easeInOut(duration: 0.7), value: emotion)
        .animation(.easeInOut(duration: 0.6), value: glancing)
        .animation(.easeInOut(duration: 0.7), value: isThinking)
        .onAppear {
            withAnimation(.easeInOut(duration: 3.2).repeatForever(autoreverses: true)) {
                breathPhase = 1
            }
        }
    }

    // MARK: - Pose

    private struct Pose {
        var eyeY: CGFloat
        var eyeLength: CGFloat
        var browAngle: Double
        var headTilt: Double
        var halo: Double
        var clock: Double
    }

    private var resolvedPose: Pose {
        if isThinking {
            return Pose(eyeY: -0.16, eyeLength: 0.82, browAngle: -0.10, headTilt: 0.05, halo: 0.5, clock: 0.35)
        }
        switch emotion {
        case .watching:
            return Pose(eyeY: 0, eyeLength: 1.0, browAngle: 0, headTilt: 0, halo: 0.35, clock: glancing ? 0.9 : 0)
        case .checkingClock:
            return Pose(eyeY: 0, eyeLength: 1.0, browAngle: 0, headTilt: -0.06, halo: 0.35, clock: 1.0)
        case .thinking:
            return Pose(eyeY: -0.16, eyeLength: 0.82, browAngle: -0.10, headTilt: 0.05, halo: 0.5, clock: 0.35)
        case .focused:
            return Pose(eyeY: 0.05, eyeLength: 0.62, browAngle: 0.18, headTilt: 0, halo: 0.7, clock: 0)
        case .strict:
            return Pose(eyeY: 0, eyeLength: 0.6, browAngle: 0.22, headTilt: 0, halo: 0.6, clock: 0)
        case .impatient:
            return Pose(eyeY: 0, eyeLength: 0.62, browAngle: 0.20, headTilt: -0.03, halo: 0.55, clock: 0.25)
        case .approving:
            return Pose(eyeY: -0.05, eyeLength: 0.9, browAngle: -0.05, headTilt: 0.04, halo: 0.85, clock: 0)
        case .disappointed:
            return Pose(eyeY: 0.22, eyeLength: 0.85, browAngle: 0.08, headTilt: 0.02, halo: 0.15, clock: 0)
        case .celebrating:
            return Pose(eyeY: -0.12, eyeLength: 1.0, browAngle: -0.12, headTilt: 0, halo: 1.0, clock: 0)
        case .silentWaiting:
            return Pose(eyeY: 0.05, eyeLength: 0.42, browAngle: 0, headTilt: 0, halo: 0.2, clock: 0)
        case .neutralWatch, .thinkingDone:
            return Pose(eyeY: 0, eyeLength: 1.0, browAngle: 0, headTilt: 0, halo: 0.35, clock: 0)
        }
    }

    // MARK: - Bust

    @ViewBuilder
    private func coachBust(size: CGSize, pose: Pose) -> some View {
        let headRadius = min(size.width, size.height) * 0.19
        let shoulderWidth = min(size.width, size.height) * 0.92
        let breath = 1.0 + 0.012 * breathPhase

        ZStack {
            VStack(spacing: 0) {
                Spacer().frame(height: size.height * 0.24)

                ZStack {
                    // Shoulders
                    RoundedRectangle(cornerRadius: shoulderWidth * 0.28, style: .continuous)
                        .fill(
                            LinearGradient(
                                colors: [
                                    Color(red: 0.165, green: 0.165, blue: 0.180),
                                    Color.arElevated,
                                    Color.arSurface,
                                    Color.black.opacity(0.95)
                                ],
                                startPoint: .top,
                                endPoint: .bottom
                            )
                        )
                        .frame(width: shoulderWidth, height: shoulderWidth * 0.54)
                        .overlay(alignment: .top) {
                            RoundedRectangle(cornerRadius: shoulderWidth * 0.28, style: .continuous)
                                .fill(
                                    LinearGradient(
                                        colors: [Color.white.opacity(0.16), Color.clear],
                                        startPoint: .top,
                                        endPoint: .center
                                    )
                                )
                                .frame(height: shoulderWidth * 0.18)
                        }
                        .overlay(alignment: .top) {
                            Capsule(style: .continuous)
                                .fill(emotion.accentColor.opacity(0.68))
                                .frame(width: shoulderWidth * 0.34, height: 3)
                                .blur(radius: 0.2)
                                .offset(y: 12)
                        }
                        .overlay(
                            RoundedRectangle(cornerRadius: shoulderWidth * 0.28, style: .continuous)
                                .stroke(
                                    LinearGradient(
                                        colors: [Color.white.opacity(0.16), Color.white.opacity(0.02)],
                                        startPoint: .top,
                                        endPoint: .bottom
                                    ),
                                    lineWidth: 0.8
                                )
                        )
                        .shadow(color: .black.opacity(0.55), radius: 30, y: 18)

                    // Neck
                    RoundedRectangle(cornerRadius: headRadius * 0.24, style: .continuous)
                        .fill(
                            LinearGradient(
                                colors: [Color.arElevated, Color.arSurface],
                                startPoint: .top,
                                endPoint: .bottom
                            )
                        )
                        .frame(width: headRadius * 0.58, height: headRadius * 1.12)
                        .overlay(
                            RoundedRectangle(cornerRadius: headRadius * 0.24, style: .continuous)
                                .stroke(Color.white.opacity(0.07), lineWidth: 0.5)
                        )
                        .offset(y: -headRadius * 0.72)
                }

                Spacer()
            }
            .scaleEffect(breath, anchor: .bottom)

            VStack(spacing: 0) {
                Spacer().frame(height: size.height * 0.18)
                head(headRadius: headRadius, pose: pose)
                Spacer()
            }
            .rotationEffect(.degrees(pose.headTilt * 180 / .pi))
        }
        .frame(width: size.width, height: size.height)
    }

    @ViewBuilder
    private func head(headRadius: CGFloat, pose: Pose) -> some View {
        let eyeBase = headRadius * 0.34
        let eyeY = headRadius * pose.eyeY

        ZStack {
            Circle()
                .fill(
                    LinearGradient(
                        colors: [
                            Color(red: 0.150, green: 0.150, blue: 0.162),
                            Color.arSurface,
                            Color.black.opacity(0.88)
                        ],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    )
                )
                .frame(width: headRadius * 2, height: headRadius * 2.05)
                .overlay(
                    Circle()
                        .stroke(
                            LinearGradient(
                                colors: [Color.white.opacity(0.20), emotion.accentColor.opacity(0.16), Color.clear],
                                startPoint: .topLeading,
                                endPoint: .bottomTrailing
                            ),
                            lineWidth: 0.8
                        )
                )
                .shadow(color: emotion.accentColor.opacity(0.22), radius: 28, y: 10)
                .shadow(color: .black.opacity(0.58), radius: 22, y: 12)

            VStack(spacing: headRadius * 0.34) {
                gazeRow(headRadius: headRadius, eyeBase: eyeBase, pose: pose)
                    .offset(y: eyeY)

                RoundedRectangle(cornerRadius: 1.5, style: .continuous)
                    .fill(emotionMouthColor)
                    .frame(width: headRadius * 0.5, height: 2.2)
                    .offset(y: headRadius * 0.18)
            }
            .offset(y: -headRadius * 0.12)

            Image(systemName: "clock")
                .font(.system(size: headRadius * 0.42, weight: .semibold))
                .foregroundStyle(emotion.accentColor)
                .opacity(pose.clock)
                .offset(x: headRadius * 1.15, y: -headRadius * 0.65)
        }
    }

    @ViewBuilder
    private func gazeRow(headRadius: CGFloat, eyeBase: CGFloat, pose: Pose) -> some View {
        let length = eyeBase * pose.eyeLength

        HStack(spacing: headRadius * 0.42) {
            eye(length: length, browAngle: pose.browAngle)
            eye(length: length, browAngle: -pose.browAngle)
        }
    }

    @ViewBuilder
    private func eye(length: CGFloat, browAngle: Double) -> some View {
        ZStack {
            Capsule(style: .continuous)
                .fill(Color.arTextPrimary.opacity(0.92))
                .frame(width: length, height: 4.2)
                .shadow(color: emotion.accentColor.opacity(0.45), radius: 5)

            // Brow — a thin angled line above the eye for sharpness.
            Capsule(style: .continuous)
                .fill(emotion.accentColor.opacity(0.85))
                .frame(width: length * 1.05, height: 2.4)
                .rotationEffect(.degrees(browAngle * 180 / .pi))
                .offset(y: -6)
        }
    }

    private var emotionMouthColor: Color {
        switch emotion {
        case .approving, .celebrating:
            return .arSuccess.opacity(0.8)
        case .disappointed:
            return .arTextMuted
        case .strict, .impatient:
            return .arAccent.opacity(0.8)
        default:
            return Color.arTextSecondary.opacity(0.8)
        }
    }
}

private struct StageStatusPill: View {
    let emotion: CoachEmotion
    let isThinking: Bool

    var body: some View {
        HStack(spacing: 8) {
            Circle()
                .fill(isThinking ? Color.arWarning : emotion.accentColor)
                .frame(width: 7, height: 7)
                .shadow(color: (isThinking ? Color.arWarning : emotion.accentColor).opacity(0.65), radius: 7)

            Text(isThinking ? "THINKING" : emotion.stageLabel)
                .font(.caption2.weight(.bold))
                .tracking(1.1)
                .foregroundStyle(.arTextSecondary)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(
            Capsule(style: .continuous)
                .fill(Color.black.opacity(0.24))
        )
        .overlay(
            Capsule(style: .continuous)
                .stroke(Color.white.opacity(0.10), lineWidth: 0.5)
        )
    }
}

private extension CoachEmotion {
    var stageLabel: String {
        switch self {
        case .watching, .neutralWatch:
            return "WATCHING"
        case .checkingClock:
            return "CLOCK"
        case .thinking, .thinkingDone:
            return "THINKING"
        case .focused:
            return "FOCUSED"
        case .strict:
            return "STRICT"
        case .impatient:
            return "WAITING"
        case .approving:
            return "APPROVING"
        case .disappointed:
            return "DISAPPOINTED"
        case .celebrating:
            return "CLEAN"
        case .silentWaiting:
            return "SILENT"
        }
    }
}

// MARK: - Preview

#Preview("Coach Stage") {
    CoachStage(emotion: .focused, isThinking: false)
}

#Preview("Coach Stage – thinking") {
    CoachStage(emotion: .watching, isThinking: true)
}
