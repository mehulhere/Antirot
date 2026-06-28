import SwiftUI

// MARK: - Brand Colors (Red / Gold / Cyan)

extension Color {
    // Backgrounds
    static let antirotBg = Color(red: 0.031, green: 0.027, blue: 0.043)             // #08070b
    static let antirotBgSurface = Color(red: 0.071, green: 0.067, blue: 0.102)      // #12111a
    static let antirotBgElevated = Color(red: 0.102, green: 0.094, blue: 0.149)     // #1a1826
    static let antirotBgOverlay = Color(red: 0.047, green: 0.039, blue: 0.078).opacity(0.85) // rgba(12,10,20,0.85)

    // Accents
    static let antirotAccent = Color(red: 0.957, green: 0.247, blue: 0.369)         // #f43f5e
    static let antirotAccentBright = Color(red: 0.984, green: 0.443, blue: 0.522)   // #fb7185
    static let antirotAccentDim = Color(red: 0.882, green: 0.114, blue: 0.282)      // #e11d48
    static let antirotGold = Color(red: 0.961, green: 0.718, blue: 0.192)           // #f5b731
    static let antirotCyan = Color(red: 0.024, green: 0.714, blue: 0.831)           // #06b6d4

    // Semantic
    static let antirotDanger = Color(red: 0.937, green: 0.267, blue: 0.267)         // #ef4444
    static let antirotSuccess = Color(red: 0.204, green: 0.827, blue: 0.600)        // #34d399
    static let antirotWarning = Color(red: 0.984, green: 0.749, blue: 0.141)        // #fbbf24

    // Text
    static let antirotTextPrimary = Color(red: 0.941, green: 0.933, blue: 0.965)    // #f0eef6
    static let antirotTextSecondary = Color(red: 0.620, green: 0.588, blue: 0.722)  // #9e96b8
    static let antirotTextMuted = Color(red: 0.361, green: 0.329, blue: 0.471)      // #5c5478

    // Borders
    static let antirotBorder = Color.white.opacity(0.05)
    static let antirotBorderStrong = Color(red: 0.957, green: 0.247, blue: 0.369).opacity(0.20)

    // Glows
    static let antirotGlowPrimary = Color(red: 0.957, green: 0.247, blue: 0.369).opacity(0.12)
    static let antirotGlowGold = Color(red: 0.961, green: 0.718, blue: 0.192).opacity(0.10)
    static let antirotGlowCyan = Color(red: 0.024, green: 0.714, blue: 0.831).opacity(0.08)
}

extension ShapeStyle where Self == Color {
    static var antirotBg: Color { .antirotBg }
    static var antirotBgSurface: Color { .antirotBgSurface }
    static var antirotBgElevated: Color { .antirotBgElevated }
    static var antirotAccent: Color { .antirotAccent }
    static var antirotAccentBright: Color { .antirotAccentBright }
    static var antirotAccentDim: Color { .antirotAccentDim }
    static var antirotGold: Color { .antirotGold }
    static var antirotCyan: Color { .antirotCyan }
    static var antirotDanger: Color { .antirotDanger }
    static var antirotSuccess: Color { .antirotSuccess }
    static var antirotWarning: Color { .antirotWarning }
    static var antirotTextPrimary: Color { .antirotTextPrimary }
    static var antirotTextSecondary: Color { .antirotTextSecondary }
    static var antirotTextMuted: Color { .antirotTextMuted }
    static var antirotBorder: Color { .antirotBorder }
    static var antirotBorderStrong: Color { .antirotBorderStrong }
    static var antirotGlowPrimary: Color { .antirotGlowPrimary }
    static var antirotGlowGold: Color { .antirotGlowGold }
    static var antirotGlowCyan: Color { .antirotGlowCyan }
}

// MARK: - Gradient Presets

extension LinearGradient {
    static let antirotAccent = LinearGradient(
        colors: [.antirotAccent, .antirotAccentDim],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )

    static let antirotGoldGradient = LinearGradient(
        colors: [.antirotGold, Color(red: 0.831, green: 0.592, blue: 0.039)],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )
}

// MARK: - Layered Card Modifier (replaces GlassCard)

struct LayeredCardModifier: ViewModifier {
    var cornerRadius: CGFloat = 14
    var padding: CGFloat = 20
    var showBorder: Bool = true

    func body(content: Content) -> some View {
        content
            .padding(padding)
            .background(
                RoundedRectangle(cornerRadius: cornerRadius)
                    .fill(Color.antirotBgElevated)
            )
            .overlay(
                RoundedRectangle(cornerRadius: cornerRadius)
                    .strokeBorder(
                        showBorder ? Color.antirotBorder : .clear,
                        lineWidth: 1
                    )
            )
            .overlay(alignment: .top) {
                if showBorder {
                    Rectangle()
                        .fill(
                            LinearGradient(
                                colors: [Color.white.opacity(0.06), .clear],
                                startPoint: .leading,
                                endPoint: .trailing
                            )
                        )
                        .frame(height: 1)
                        .clipShape(
                            UnevenRoundedRectangle(
                                topLeadingRadius: cornerRadius,
                                topTrailingRadius: cornerRadius
                            )
                        )
                }
            }
    }
}

extension View {
    func layeredCard(cornerRadius: CGFloat = 14, padding: CGFloat = 20, showBorder: Bool = true) -> some View {
        modifier(LayeredCardModifier(cornerRadius: cornerRadius, padding: padding, showBorder: showBorder))
    }

    // Keep backward compatibility
    func glassCard(cornerRadius: CGFloat = 14, padding: CGFloat = 20, showBorder: Bool = true) -> some View {
        modifier(LayeredCardModifier(cornerRadius: cornerRadius, padding: padding, showBorder: showBorder))
    }
}

// MARK: - Accent Glow Modifier

struct AccentGlowModifier: ViewModifier {
    var color: Color = .antirotAccent
    var radius: CGFloat = 20

    func body(content: Content) -> some View {
        content
            .shadow(color: color.opacity(0.3), radius: radius, x: 0, y: 8)
    }
}

extension View {
    func accentGlow(color: Color = .antirotAccent, radius: CGFloat = 20) -> some View {
        modifier(AccentGlowModifier(color: color, radius: radius))
    }
}

// MARK: - Severity Color

extension AlarmJob.Severity {
    var color: Color {
        switch self {
        case .normal:
            return .antirotGold
        case .loud:
            return .antirotDanger
        case .urgent:
            return .antirotDanger
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

// MARK: - Mesh Background (replaces AmbientBackground)

struct MeshBackground: View {
    @State private var phase: Double = 0

    var body: some View {
        ZStack {
            Color.antirotBg.ignoresSafeArea()

            Circle()
                .fill(
                    RadialGradient(
                        colors: [Color.antirotGlowPrimary, .clear],
                        center: .center,
                        startRadius: 0,
                        endRadius: 300
                    )
                )
                .frame(width: 600, height: 600)
                .offset(x: -100, y: -200)
                .opacity(0.6 + 0.3 * sin(phase))

            Circle()
                .fill(
                    RadialGradient(
                        colors: [Color.antirotAccentDim.opacity(0.08), .clear],
                        center: .center,
                        startRadius: 0,
                        endRadius: 250
                    )
                )
                .frame(width: 500, height: 500)
                .offset(x: 150, y: 300)
                .opacity(0.4 + 0.2 * sin(phase + .pi))

            Circle()
                .fill(
                    RadialGradient(
                        colors: [Color.antirotGlowCyan, .clear],
                        center: .center,
                        startRadius: 0,
                        endRadius: 200
                    )
                )
                .frame(width: 400, height: 400)
                .offset(x: 80, y: -100)
                .opacity(0.3 + 0.15 * sin(phase + .pi * 0.5))
        }
        .ignoresSafeArea()
        .onAppear {
            withAnimation(.easeInOut(duration: 10).repeatForever(autoreverses: true)) {
                phase = .pi * 2
            }
        }
    }
}

// Keep backward compatibility
typealias AmbientBackground = MeshBackground

// MARK: - Status Dot (with glow ring)

struct StatusDot: View {
    let color: Color
    var animated: Bool = true

    @State private var isPulsing = false

    var body: some View {
        ZStack {
            if animated {
                Circle()
                    .fill(color.opacity(0.15))
                    .frame(width: 16, height: 16)
                    .scaleEffect(isPulsing ? 1.4 : 1.0)
            }
            Circle()
                .fill(color)
                .frame(width: 8, height: 8)
                .shadow(color: color.opacity(0.5), radius: isPulsing ? 6 : 2)
        }
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
                    .foregroundStyle(.antirotAccent)
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

// MARK: - Accent Button Style (Red)

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
            .shadow(color: .antirotAccent.opacity(configuration.isPressed ? 0.1 : 0.3), radius: 12, y: 6)
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
            .background(Color.white.opacity(0.04))
            .overlay(
                RoundedRectangle(cornerRadius: 10)
                    .strokeBorder(Color.antirotBorder, lineWidth: 1)
            )
            .clipShape(RoundedRectangle(cornerRadius: 10))
            .scaleEffect(configuration.isPressed ? 0.97 : 1.0)
            .animation(.easeOut(duration: 0.15), value: configuration.isPressed)
    }
}

// MARK: - Gold Button Style

struct AntirotGoldButtonStyle: ButtonStyle {
    var fullWidth: Bool = false

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.headline)
            .foregroundStyle(Color.antirotBgElevated)
            .frame(maxWidth: fullWidth ? .infinity : nil)
            .padding(.horizontal, 28)
            .padding(.vertical, 14)
            .background(
                LinearGradient.antirotGoldGradient
            )
            .clipShape(RoundedRectangle(cornerRadius: 12))
            .shadow(color: .antirotGold.opacity(configuration.isPressed ? 0.1 : 0.25), radius: 12, y: 6)
            .scaleEffect(configuration.isPressed ? 0.97 : 1.0)
            .animation(.easeOut(duration: 0.15), value: configuration.isPressed)
    }
}

// MARK: - Destructive Button Style

struct AntirotDestructiveButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.subheadline.weight(.medium))
            .foregroundStyle(.antirotDanger)
            .padding(.horizontal, 20)
            .padding(.vertical, 12)
            .background(Color.antirotDanger.opacity(0.1))
            .overlay(
                RoundedRectangle(cornerRadius: 10)
                    .strokeBorder(Color.antirotDanger.opacity(0.3), lineWidth: 1)
            )
            .clipShape(RoundedRectangle(cornerRadius: 10))
            .scaleEffect(configuration.isPressed ? 0.97 : 1.0)
            .animation(.easeOut(duration: 0.15), value: configuration.isPressed)
    }
}

// MARK: - Focus Dial (replaces SiriCoachOrb)

struct FocusDial: View {
    var isRecording: Bool = false
    var isThinking: Bool = false
    var size: CGFloat = 120

    @State private var rotation1: Double = 0
    @State private var rotation2: Double = 0
    @State private var rotation3: Double = 0
    @State private var pulseScale: CGFloat = 1.0

    var body: some View {
        ZStack {
            // Outer ring - red
            Circle()
                .stroke(
                    Color.antirotAccent,
                    style: StrokeStyle(lineWidth: isRecording ? 3.5 : 2.5, lineCap: .round, dash: [size * 0.55, size * 0.35])
                )
                .frame(width: size, height: size)
                .rotationEffect(.degrees(rotation1))
                .shadow(color: .antirotAccent.opacity(0.3), radius: 8)

            // Mid ring - gold
            Circle()
                .stroke(
                    Color.antirotGold,
                    style: StrokeStyle(lineWidth: isRecording ? 2.5 : 1.8, lineCap: .round, dash: [size * 0.42, size * 0.30])
                )
                .frame(width: size * 0.76, height: size * 0.76)
                .rotationEffect(.degrees(rotation2))
                .shadow(color: .antirotGold.opacity(0.2), radius: 6)

            // Inner ring - cyan
            Circle()
                .stroke(
                    Color.antirotCyan,
                    style: StrokeStyle(lineWidth: isRecording ? 2.0 : 1.2, lineCap: .round, dash: [size * 0.32, size * 0.26])
                )
                .frame(width: size * 0.54, height: size * 0.54)
                .rotationEffect(.degrees(rotation3))
                .shadow(color: .antirotCyan.opacity(0.25), radius: 4)

            // Center circle
            Circle()
                .fill(
                    RadialGradient(
                        colors: [Color.antirotAccentDim.opacity(0.3), Color.antirotBg],
                        center: .center,
                        startRadius: 0,
                        endRadius: size * 0.2
                    )
                )
                .frame(width: size * 0.36, height: size * 0.36)
                .overlay(
                    Circle()
                        .strokeBorder(Color.antirotBorderStrong, lineWidth: 1)
                )

            // Center icon
            Image(systemName: isRecording ? "waveform" : "bolt.fill")
                .font(.system(size: size * 0.12, weight: .bold))
                .foregroundStyle(.antirotAccentBright)
        }
        .scaleEffect(pulseScale)
        .onAppear {
            startAnimations()
        }
        .onChange(of: isRecording) { _, _ in
            startAnimations()
        }
        .onChange(of: isThinking) { _, _ in
            startAnimations()
        }
    }

    private func startAnimations() {
        let outerSpeed: Double = isRecording ? 3 : (isThinking ? 4 : 10)
        let midSpeed: Double = isRecording ? 2.5 : (isThinking ? 3 : 7)
        let innerSpeed: Double = isRecording ? 2 : (isThinking ? 2.5 : 5)

        withAnimation(.linear(duration: outerSpeed).repeatForever(autoreverses: false)) {
            rotation1 = rotation1 + 360
        }
        withAnimation(.linear(duration: midSpeed).repeatForever(autoreverses: false)) {
            rotation2 = rotation2 - 360
        }
        withAnimation(.linear(duration: innerSpeed).repeatForever(autoreverses: false)) {
            rotation3 = rotation3 + 360
        }

        if isRecording {
            withAnimation(.easeInOut(duration: 1.0).repeatForever(autoreverses: true)) {
                pulseScale = 1.06
            }
        } else {
            withAnimation(.easeOut(duration: 0.3)) {
                pulseScale = 1.0
            }
        }
    }
}
