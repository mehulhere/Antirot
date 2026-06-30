import SwiftUI

// MARK: - Color Palette (Monochrome + Single Accent)

extension Color {
    // Backgrounds — warm near-blacks
    static let arBg = Color(red: 0.039, green: 0.039, blue: 0.039)                  // #0A0A0A
    static let arSurface = Color(red: 0.078, green: 0.078, blue: 0.078)              // #141414
    static let arElevated = Color(red: 0.110, green: 0.110, blue: 0.118)             // #1C1C1E
    static let arOverlay = Color.black.opacity(0.6)

    // Accent — sophisticated muted red
    static let arAccent = Color(red: 0.902, green: 0.224, blue: 0.275)               // #E63946
    static let arAccentDim = Color(red: 0.776, green: 0.157, blue: 0.157)            // #C62828
    static let arAccentSubtle = Color(red: 0.902, green: 0.224, blue: 0.275).opacity(0.12)

    // Text — clear hierarchy
    static let arTextPrimary = Color(red: 0.960, green: 0.960, blue: 0.960)          // #F5F5F5
    static let arTextSecondary = Color(red: 0.557, green: 0.557, blue: 0.576)        // #8E8E93
    static let arTextMuted = Color(red: 0.282, green: 0.282, blue: 0.290)            // #48484A

    // Borders
    static let arBorder = Color.white.opacity(0.06)
    static let arBorderActive = Color.white.opacity(0.12)

    // Semantic (used sparingly, never decoratively)
    static let arSuccess = Color(red: 0.188, green: 0.820, blue: 0.345)              // #30D158
    static let arWarning = Color(red: 1.000, green: 0.839, blue: 0.039)              // #FFD60A
    static let arDanger = Color(red: 1.000, green: 0.271, blue: 0.227)               // #FF453A
}

// ShapeStyle convenience
extension ShapeStyle where Self == Color {
    static var arBg: Color { .arBg }
    static var arSurface: Color { .arSurface }
    static var arElevated: Color { .arElevated }
    static var arAccent: Color { .arAccent }
    static var arAccentDim: Color { .arAccentDim }
    static var arTextPrimary: Color { .arTextPrimary }
    static var arTextSecondary: Color { .arTextSecondary }
    static var arTextMuted: Color { .arTextMuted }
    static var arBorder: Color { .arBorder }
    static var arSuccess: Color { .arSuccess }
    static var arWarning: Color { .arWarning }
    static var arDanger: Color { .arDanger }
}

// MARK: - Backward Compatibility Aliases
// Preserves compilation of code that still references old tokens.

extension Color {
    static let antirotBg = arBg
    static let antirotBgSurface = arSurface
    static let antirotBgElevated = arElevated
    static let antirotBgOverlay = arOverlay
    static let antirotAccent = arAccent
    static let antirotAccentBright = arAccent
    static let antirotAccentDim = arAccentDim
    static let antirotGold = arWarning
    static let antirotCyan = Color(red: 0.024, green: 0.714, blue: 0.831)
    static let antirotDanger = arDanger
    static let antirotSuccess = arSuccess
    static let antirotWarning = arWarning
    static let antirotTextPrimary = arTextPrimary
    static let antirotTextSecondary = arTextSecondary
    static let antirotTextMuted = arTextMuted
    static let antirotBorder = arBorder
    static let antirotBorderStrong = arBorderActive
    static let antirotGlowPrimary = arAccentSubtle
    static let antirotGlowGold = arWarning.opacity(0.10)
    static let antirotGlowCyan = Color(red: 0.024, green: 0.714, blue: 0.831).opacity(0.08)
}

extension ShapeStyle where Self == Color {
    static var antirotBg: Color { .arBg }
    static var antirotBgSurface: Color { .arSurface }
    static var antirotBgElevated: Color { .arElevated }
    static var antirotAccent: Color { .arAccent }
    static var antirotAccentBright: Color { .arAccent }
    static var antirotAccentDim: Color { .arAccentDim }
    static var antirotGold: Color { .arWarning }
    static var antirotCyan: Color { .antirotCyan }
    static var antirotDanger: Color { .arDanger }
    static var antirotSuccess: Color { .arSuccess }
    static var antirotWarning: Color { .arWarning }
    static var antirotTextPrimary: Color { .arTextPrimary }
    static var antirotTextSecondary: Color { .arTextSecondary }
    static var antirotTextMuted: Color { .arTextMuted }
    static var antirotBorder: Color { .arBorder }
    static var antirotBorderStrong: Color { .arBorderActive }
    static var antirotGlowPrimary: Color { .antirotGlowPrimary }
    static var antirotGlowGold: Color { .antirotGlowGold }
    static var antirotGlowCyan: Color { .antirotGlowCyan }
}

// MARK: - Gradient Presets

extension LinearGradient {
    static let antirotAccent = LinearGradient(
        colors: [.arAccent, .arAccentDim],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )

    static let antirotGoldGradient = LinearGradient(
        colors: [.arWarning, .arWarning.opacity(0.8)],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )
}

// MARK: - Minimal Card Modifier

struct MinimalCardModifier: ViewModifier {
    var cornerRadius: CGFloat = 16
    var padding: CGFloat = 16

    func body(content: Content) -> some View {
        content
            .padding(padding)
            .background(
                RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                    .fill(Color.arSurface)
            )
    }
}

extension View {
    func minimalCard(cornerRadius: CGFloat = 16, padding: CGFloat = 16) -> some View {
        modifier(MinimalCardModifier(cornerRadius: cornerRadius, padding: padding))
    }

    // Backward compat
    func layeredCard(cornerRadius: CGFloat = 16, padding: CGFloat = 16, showBorder: Bool = true) -> some View {
        modifier(MinimalCardModifier(cornerRadius: cornerRadius, padding: padding))
    }

    func glassCard(cornerRadius: CGFloat = 16, padding: CGFloat = 16, showBorder: Bool = true) -> some View {
        modifier(MinimalCardModifier(cornerRadius: cornerRadius, padding: padding))
    }
}

// MARK: - Section Divider

struct SectionDivider: View {
    var body: some View {
        Rectangle()
            .fill(Color.arBorder)
            .frame(height: 0.33)
    }
}

// MARK: - State Pill

struct StatePill: View {
    let label: String
    var isActive: Bool = false

    var body: some View {
        Text(label.uppercased())
            .font(.caption2.weight(.semibold))
            .tracking(0.8)
            .foregroundStyle(isActive ? .arAccent : .arTextSecondary)
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(
                Capsule(style: .continuous)
                    .fill(isActive ? Color.arAccentSubtle : Color.arSurface)
            )
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
            .scaleEffect(animated && isPulsing ? 1.3 : 1.0)
            .opacity(animated && isPulsing ? 0.7 : 1.0)
            .onAppear {
                guard animated else { return }
                withAnimation(.easeInOut(duration: 1.5).repeatForever(autoreverses: true)) {
                    isPulsing = true
                }
            }
    }
}

// MARK: - Section Header (minimal)

struct AntirotSectionHeader: View {
    let title: String
    var icon: String?

    var body: some View {
        HStack(spacing: 6) {
            if let icon {
                Image(systemName: icon)
                    .font(.caption2)
                    .foregroundStyle(.arTextMuted)
            }
            Text(title.uppercased())
                .font(.caption2.weight(.medium))
                .tracking(1.2)
                .foregroundStyle(.arTextMuted)
            Spacer()
        }
        .padding(.top, 4)
    }
}

// MARK: - Severity Color

extension AlarmJob.Severity {
    var color: Color {
        switch self {
        case .normal: return .arWarning
        case .loud: return .arDanger
        case .urgent: return .arDanger
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

// MARK: - Button Styles

struct AntirotAccentButtonStyle: ButtonStyle {
    var fullWidth: Bool = false

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.subheadline.weight(.semibold))
            .foregroundStyle(.white)
            .frame(maxWidth: fullWidth ? .infinity : nil)
            .padding(.horizontal, 24)
            .padding(.vertical, 14)
            .background(Color.arAccent)
            .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
            .scaleEffect(configuration.isPressed ? 0.97 : 1.0)
            .animation(.easeOut(duration: 0.15), value: configuration.isPressed)
    }
}

struct AntirotGhostButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.subheadline.weight(.medium))
            .foregroundStyle(.arTextSecondary)
            .padding(.horizontal, 16)
            .padding(.vertical, 10)
            .background(Color.arSurface)
            .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
            .scaleEffect(configuration.isPressed ? 0.97 : 1.0)
            .animation(.easeOut(duration: 0.15), value: configuration.isPressed)
    }
}

struct AntirotGoldButtonStyle: ButtonStyle {
    var fullWidth: Bool = false

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.subheadline.weight(.semibold))
            .foregroundStyle(.black)
            .frame(maxWidth: fullWidth ? .infinity : nil)
            .padding(.horizontal, 24)
            .padding(.vertical, 14)
            .background(Color.arWarning)
            .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
            .scaleEffect(configuration.isPressed ? 0.97 : 1.0)
            .animation(.easeOut(duration: 0.15), value: configuration.isPressed)
    }
}

struct AntirotDestructiveButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.subheadline.weight(.medium))
            .foregroundStyle(.arDanger)
            .padding(.horizontal, 16)
            .padding(.vertical, 10)
            .background(Color.arDanger.opacity(0.08))
            .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
            .scaleEffect(configuration.isPressed ? 0.97 : 1.0)
            .animation(.easeOut(duration: 0.15), value: configuration.isPressed)
    }
}

// MARK: - Accent Glow (retained for backward compat, minimal effect)

struct AccentGlowModifier: ViewModifier {
    var color: Color = .arAccent
    var radius: CGFloat = 12

    func body(content: Content) -> some View {
        content
            .shadow(color: color.opacity(0.15), radius: radius, x: 0, y: 4)
    }
}

extension View {
    func accentGlow(color: Color = .arAccent, radius: CGFloat = 12) -> some View {
        modifier(AccentGlowModifier(color: color, radius: radius))
    }
}

// MARK: - Legacy Aliases

// MeshBackground becomes solid bg
struct MeshBackground: View {
    var body: some View {
        Color.arBg.ignoresSafeArea()
    }
}

typealias AmbientBackground = MeshBackground

// FocusDial is retained as empty for compilation but renders nothing
struct FocusDial: View {
    var isRecording: Bool = false
    var isThinking: Bool = false
    var size: CGFloat = 120

    var body: some View {
        EmptyView()
    }
}

// MARK: - Liquid Glass

/// Translucent glass surface in the spirit of the iOS "Liquid Glass" language:
/// strong background blur, a hairline specular border, and a soft top sheen.
/// Built on `.ultraThinMaterial` so it works on the iOS 17 deployment target
/// and keeps the coach scene visible through the chat sheet.
struct LiquidGlassModifier: ViewModifier {
    var cornerRadius: CGFloat = 24
    var borderWidth: CGFloat = 0.5
    var sheen: Bool = true

    func body(content: Content) -> some View {
        content
            .background(
                RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                    .fill(.ultraThinMaterial)
            )
            .overlay(
                RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                    .stroke(
                        LinearGradient(
                            colors: [Color.white.opacity(0.20), Color.white.opacity(0.04)],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        ),
                        lineWidth: borderWidth
                    )
            )
            .overlay(alignment: .top) {
                if sheen {
                    RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                        .fill(
                            LinearGradient(
                                colors: [Color.white.opacity(0.10), .clear],
                                startPoint: .top,
                                endPoint: .center
                            )
                        )
                        .clipShape(RoundedRectangle(cornerRadius: cornerRadius, style: .continuous))
                }
            }
            .clipShape(RoundedRectangle(cornerRadius: cornerRadius, style: .continuous))
    }
}

extension View {
    /// Apply the Liquid Glass material treatment to any view.
    func liquidGlass(cornerRadius: CGFloat = 24, borderWidth: CGFloat = 0.5, sheen: Bool = true) -> some View {
        modifier(LiquidGlassModifier(cornerRadius: cornerRadius, borderWidth: borderWidth, sheen: sheen))
    }

    /// Quiet glass capsule used for small secondary controls.
    func glassCapsule() -> some View {
        self
            .liquidGlass(cornerRadius: 22, borderWidth: 0.5, sheen: false)
    }
}
