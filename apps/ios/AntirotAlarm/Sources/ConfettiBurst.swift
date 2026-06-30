import SwiftUI

// MARK: - Confetti / Charged-Particle Burst

/// A short, energetic particle burst shown over the coach scene on a successful
/// Start. Sharp charged shards, not childish confetti. Plays for ~1.0s then
/// calls `onComplete` so the caller can stop rendering it.
struct ConfettiBurst: View {
    var onComplete: () -> Void

    @State private var startedAt: Date = .distantPast
    private let lifetime: Double = 1.0

    private let particles: [Particle] = ConfettiBurst.makeParticles()

    var body: some View {
        TimelineView(.animation(minimumInterval: 1.0 / 60.0)) { context in
            Canvas { graphics, size in
                let start = startedAt == .distantPast ? Date() : startedAt
                let elapsed = Date().timeIntervalSince(start)
                guard elapsed <= lifetime else { return }

                let progress = min(max(elapsed / lifetime, 0), 1)
                let center = CGPoint(x: size.width / 2, y: size.height / 2)

                for particle in particles {
                    let eased = ConfettiBurst.easeOut(progress: progress)
                    let distance = particle.speed * eased * min(size.width, size.height) * 0.55
                    let x = center.x + cos(particle.angle) * distance
                    let y = center.y + sin(particle.angle) * distance + particle.gravity * eased * eased
                    let opacity = 1.0 - progress
                    let rotation = particle.spin * eased
                    let scale = 1.0 - 0.35 * eased

                    var path = Path()
                    let w = particle.size * scale
                    let h = particle.size * 0.34 * scale
                    path.addRoundedRect(
                        in: CGRect(x: -w / 2, y: -h / 2, width: w, height: h),
                        cornerSize: CGSize(width: h / 2, height: h / 2)
                    )

                    var particleContext = graphics
                    particleContext.opacity = opacity
                    particleContext.translateBy(x: x, y: y)
                    particleContext.rotate(by: rotation)
                    particleContext.fill(path, with: .color(particle.color))
                }
            }
            .ignoresSafeArea()
        }
        .allowsHitTesting(false)
        .onAppear { startedAt = Date() }
        .task {
            // Dismiss just past the lifetime so the last frame is fully visible.
            try? await Task.sleep(nanoseconds: UInt64((lifetime + 0.05) * 1_000_000_000))
            onComplete()
        }
    }

    // MARK: - Particles

    private struct Particle {
        let angle: Double
        let speed: Double
        let size: CGFloat
        let spin: Double
        let gravity: CGFloat
        let color: Color
    }

    private static func makeParticles() -> [Particle] {
        var rng = SystemRandomNumberGenerator()
        let colors: [Color] = [.arAccent, .arAccentDim, .arWarning, .arSuccess, .arTextPrimary]
        return (0..<34).map { _ in
            Particle(
                angle: Double.random(in: 0...(2 * .pi), using: &rng),
                speed: Double.random(in: 0.55...1.0, using: &rng),
                size: CGFloat.random(in: 9...16, using: &rng),
                spin: Double.random(in: -(.pi * 3)...(.pi * 3), using: &rng),
                gravity: CGFloat.random(in: 20...60, using: &rng),
                color: colors.randomElement(using: &rng) ?? .arAccent
            )
        }
    }

    private static func easeOut(progress: Double) -> Double {
        1 - pow(1 - progress, 3)
    }
}
