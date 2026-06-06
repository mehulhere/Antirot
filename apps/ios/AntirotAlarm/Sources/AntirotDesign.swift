import SwiftUI

// MARK: - Brand Colors

extension Color {
    static let antirotBg = Color(red: 0.039, green: 0.039, blue: 0.043)           // #0a0a0b
    static let antirotBgSecondary = Color(red: 0.067, green: 0.067, blue: 0.075)   // #111113
    static let antirotCard = Color.white.opacity(0.03)
    static let antirotCardHover = Color.white.opacity(0.06)
    static let antirotGlass = Color.white.opacity(0.04)
    static let antirotBorder = Color.white.opacity(0.06)
    static let antirotBorderAccent = Color(red: 0.863, green: 0.149, blue: 0.149).opacity(0.3)
    static let antirotTextPrimary = Color(red: 0.961, green: 0.961, blue: 0.961)   // #f5f5f5
    static let antirotTextSecondary = Color(red: 0.631, green: 0.631, blue: 0.667) // #a1a1aa
    static let antirotTextMuted = Color(red: 0.443, green: 0.443, blue: 0.478)     // #71717a
    static let antirotAccentRed = Color(red: 0.863, green: 0.149, blue: 0.149)     // #dc2626
    static let antirotAccentRedDim = Color(red: 0.6, green: 0.106, blue: 0.106)    // #991b1b
    static let antirotAccentOrange = Color(red: 0.918, green: 0.345, blue: 0.047)  // #ea580c
    static let antirotAccentAmber = Color(red: 0.851, green: 0.467, blue: 0.024)   // #d97706
    static let antirotGlowRed = Color(red: 0.863, green: 0.149, blue: 0.149).opacity(0.15)
    static let antirotGlowOrange = Color(red: 0.918, green: 0.345, blue: 0.047).opacity(0.1)
    static let antirotSuccess = Color(red: 0.133, green: 0.773, blue: 0.369)       // #22c55e
}

extension ShapeStyle where Self == Color {
    static var antirotBg: Color { .antirotBg }
    static var antirotBgSecondary: Color { .antirotBgSecondary }
    static var antirotCard: Color { .antirotCard }
    static var antirotCardHover: Color { .antirotCardHover }
    static var antirotGlass: Color { .antirotGlass }
    static var antirotBorder: Color { .antirotBorder }
    static var antirotBorderAccent: Color { .antirotBorderAccent }
    static var antirotTextPrimary: Color { .antirotTextPrimary }
    static var antirotTextSecondary: Color { .antirotTextSecondary }
    static var antirotTextMuted: Color { .antirotTextMuted }
    static var antirotAccentRed: Color { .antirotAccentRed }
    static var antirotAccentRedDim: Color { .antirotAccentRedDim }
    static var antirotAccentOrange: Color { .antirotAccentOrange }
    static var antirotAccentAmber: Color { .antirotAccentAmber }
    static var antirotGlowRed: Color { .antirotGlowRed }
    static var antirotGlowOrange: Color { .antirotGlowOrange }
    static var antirotSuccess: Color { .antirotSuccess }
}

// MARK: - Gradient Presets

extension LinearGradient {
    static let antirotAccent = LinearGradient(
        colors: [.antirotAccentRed, .antirotAccentOrange],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )
}

// MARK: - Glass Card Modifier

struct GlassCardModifier: ViewModifier {
    var cornerRadius: CGFloat = 16
    var padding: CGFloat = 20
    var showBorder: Bool = true

    func body(content: Content) -> some View {
        content
            .padding(padding)
            .background(
                RoundedRectangle(cornerRadius: cornerRadius)
                    .fill(.ultraThinMaterial)
                    .opacity(0.3)
            )
            .background(
                RoundedRectangle(cornerRadius: cornerRadius)
                    .fill(Color.antirotCard)
            )
            .overlay(
                RoundedRectangle(cornerRadius: cornerRadius)
                    .strokeBorder(Color.antirotBorder, lineWidth: showBorder ? 1 : 0)
            )
    }
}

extension View {
    func glassCard(cornerRadius: CGFloat = 16, padding: CGFloat = 20, showBorder: Bool = true) -> some View {
        modifier(GlassCardModifier(cornerRadius: cornerRadius, padding: padding, showBorder: showBorder))
    }
}

// MARK: - Accent Glow Modifier

struct AccentGlowModifier: ViewModifier {
    var color: Color = .antirotAccentRed
    var radius: CGFloat = 20

    func body(content: Content) -> some View {
        content
            .shadow(color: color.opacity(0.3), radius: radius, x: 0, y: 8)
    }
}

extension View {
    func accentGlow(color: Color = .antirotAccentRed, radius: CGFloat = 20) -> some View {
        modifier(AccentGlowModifier(color: color, radius: radius))
    }
}

// MARK: - Severity Color

extension AlarmJob.Severity {
    var color: Color {
        switch self {
        case .normal:
            return .antirotAccentOrange
        case .loud:
            return .antirotAccentRed
        case .urgent:
            return .antirotAccentRed
        }
    }

    var label: String {
        switch self {
        case .normal: return "NORMAL"
        case .loud: return "LOUD"
        case .urgent: return "URGENT"
        }
    }
}

// MARK: - Ambient Background

struct AmbientBackground: View {
    @State private var phase: Double = 0

    var body: some View {
        ZStack {
            Color.antirotBg.ignoresSafeArea()

            Circle()
                .fill(
                    RadialGradient(
                        colors: [Color.antirotGlowRed, .clear],
                        center: .center,
                        startRadius: 0,
                        endRadius: 300
                    )
                )
                .frame(width: 600, height: 600)
                .offset(x: -100, y: -200)
                .opacity(0.5 + 0.3 * sin(phase))

            Circle()
                .fill(
                    RadialGradient(
                        colors: [Color.antirotGlowOrange, .clear],
                        center: .center,
                        startRadius: 0,
                        endRadius: 250
                    )
                )
                .frame(width: 500, height: 500)
                .offset(x: 150, y: 300)
                .opacity(0.4 + 0.2 * sin(phase + .pi))
        }
        .ignoresSafeArea()
        .onAppear {
            withAnimation(.easeInOut(duration: 8).repeatForever(autoreverses: true)) {
                phase = .pi * 2
            }
        }
    }
}

// MARK: - Status Dot

struct StatusDot: View {
    let color: Color
    var animated: Bool = true

    @State private var isPulsing = false

    var body: some View {
        Circle()
            .fill(color)
            .frame(width: 8, height: 8)
            .shadow(color: color.opacity(0.5), radius: isPulsing ? 6 : 2)
            .scaleEffect(isPulsing ? 1.2 : 1.0)
            .onAppear {
                guard animated else { return }
                withAnimation(.easeInOut(duration: 1.5).repeatForever(autoreverses: true)) {
                    isPulsing = true
                }
            }
    }
}

// MARK: - Section Header Style

struct AntirotSectionHeader: View {
    let title: String
    var icon: String?

    var body: some View {
        HStack(spacing: 8) {
            if let icon {
                Image(systemName: icon)
                    .font(.caption)
                    .foregroundStyle(.antirotAccentRed)
            }
            Text(title.uppercased())
                .font(.caption)
                .fontWeight(.semibold)
                .tracking(1.5)
                .foregroundStyle(.antirotTextMuted)
            Rectangle()
                .fill(Color.antirotBorder)
                .frame(height: 1)
        }
    }
}

// MARK: - Accent Button Style

struct AntirotAccentButtonStyle: ButtonStyle {
    var fullWidth: Bool = false

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.headline)
            .foregroundStyle(.white)
            .frame(maxWidth: fullWidth ? .infinity : nil)
            .padding(.horizontal, 28)
            .padding(.vertical, 14)
            .background(
                LinearGradient.antirotAccent
            )
            .clipShape(RoundedRectangle(cornerRadius: 12))
            .shadow(color: .antirotAccentRed.opacity(configuration.isPressed ? 0.1 : 0.3), radius: 12, y: 6)
            .scaleEffect(configuration.isPressed ? 0.97 : 1.0)
            .animation(.easeOut(duration: 0.15), value: configuration.isPressed)
    }
}

// MARK: - Ghost Button Style

struct AntirotGhostButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.subheadline.weight(.medium))
            .foregroundStyle(.antirotTextSecondary)
            .padding(.horizontal, 20)
            .padding(.vertical, 12)
            .background(Color.antirotGlass)
            .overlay(
                RoundedRectangle(cornerRadius: 10)
                    .strokeBorder(Color.antirotBorder, lineWidth: 1)
            )
            .clipShape(RoundedRectangle(cornerRadius: 10))
            .scaleEffect(configuration.isPressed ? 0.97 : 1.0)
            .animation(.easeOut(duration: 0.15), value: configuration.isPressed)
    }
}

// MARK: - Destructive Button Style

struct AntirotDestructiveButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.subheadline.weight(.medium))
            .foregroundStyle(.antirotAccentRed)
            .padding(.horizontal, 20)
            .padding(.vertical, 12)
            .background(Color.antirotAccentRed.opacity(0.1))
            .overlay(
                RoundedRectangle(cornerRadius: 10)
                    .strokeBorder(Color.antirotAccentRed.opacity(0.3), lineWidth: 1)
            )
            .clipShape(RoundedRectangle(cornerRadius: 10))
            .scaleEffect(configuration.isPressed ? 0.97 : 1.0)
            .animation(.easeOut(duration: 0.15), value: configuration.isPressed)
    }
}
