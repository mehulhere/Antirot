import SwiftUI

// MARK: - Warm Smoked-Glass Palette

enum AntirotPaletteValues {
    static let backgroundRed = 0.082
    static let backgroundGreen = 0.075
    static let backgroundBlue = 0.067

    static let deepBackgroundRed = 0.035
    static let deepBackgroundGreen = 0.031
    static let deepBackgroundBlue = 0.028

    static let surfaceRed = 0.231
    static let surfaceGreen = 0.212
    static let surfaceBlue = 0.200

    static let elevatedRed = 0.318
    static let elevatedGreen = 0.290
    static let elevatedBlue = 0.275
}

extension Color {
    static let arBg = Color(
        red: AntirotPaletteValues.backgroundRed,
        green: AntirotPaletteValues.backgroundGreen,
        blue: AntirotPaletteValues.backgroundBlue
    )
    static let arDeepBg = Color(
        red: AntirotPaletteValues.deepBackgroundRed,
        green: AntirotPaletteValues.deepBackgroundGreen,
        blue: AntirotPaletteValues.deepBackgroundBlue
    )
    static let arSurface = Color(
        red: AntirotPaletteValues.surfaceRed,
        green: AntirotPaletteValues.surfaceGreen,
        blue: AntirotPaletteValues.surfaceBlue
    )
    static let arElevated = Color(
        red: AntirotPaletteValues.elevatedRed,
        green: AntirotPaletteValues.elevatedGreen,
        blue: AntirotPaletteValues.elevatedBlue
    )
    static let arGlassTint = Color(red: 0.285, green: 0.257, blue: 0.240)
    static let arOverlay = Color.black.opacity(0.66)

    // Accent — sophisticated muted red
    static let arAccent = Color(red: 0.902, green: 0.224, blue: 0.275)               // #E63946
    static let arAccentDim = Color(red: 0.776, green: 0.157, blue: 0.157)            // #C62828
    static let arAccentSubtle = Color(red: 0.902, green: 0.224, blue: 0.275).opacity(0.12)
    static let arCyan = Color(red: 0.275, green: 0.690, blue: 0.741)
    static let arAmber = Color(red: 0.914, green: 0.604, blue: 0.286)

    static let arTextPrimary = Color(red: 0.961, green: 0.945, blue: 0.925)
    static let arTextSecondary = Color(red: 0.714, green: 0.678, blue: 0.651)
    static let arTextMuted = Color(red: 0.486, green: 0.455, blue: 0.435)

    static let arBorder = Color(red: 0.961, green: 0.925, blue: 0.886).opacity(0.10)
    static let arBorderActive = Color(red: 1.000, green: 0.965, blue: 0.925).opacity(0.22)

    // Semantic (used sparingly, never decoratively)
    static let arSuccess = Color(red: 0.188, green: 0.820, blue: 0.345)              // #30D158
    static let arWarning = Color(red: 1.000, green: 0.839, blue: 0.039)              // #FFD60A
    static let arDanger = Color(red: 1.000, green: 0.271, blue: 0.227)               // #FF453A
}

// ShapeStyle convenience
extension ShapeStyle where Self == Color {
    static var arBg: Color { .arBg }
    static var arDeepBg: Color { .arDeepBg }
    static var arSurface: Color { .arSurface }
    static var arElevated: Color { .arElevated }
    static var arGlassTint: Color { .arGlassTint }
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
            .smokedGlass(cornerRadius: cornerRadius, shadow: false)
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
            .font(.caption.weight(.bold))
            .tracking(1.0)
            .foregroundStyle(isActive ? .arAccent : .arTextSecondary)
            .padding(.horizontal, 14)
            .padding(.vertical, 7)
            .background(
                Capsule(style: .continuous)
                    .fill(.ultraThinMaterial)
            )
            .background(
                Capsule(style: .continuous)
                    .fill(isActive ? Color.arAccent.opacity(0.12) : Color.white.opacity(0.035))
            )
            .overlay(
                Capsule(style: .continuous)
                    .stroke(isActive ? Color.arAccent.opacity(0.32) : Color.white.opacity(0.08), lineWidth: 0.6)
            )
    }
}

// MARK: - Cinematic App System

enum AntirotCinematicMetrics {
    static let cardRadius: CGFloat = 22
    static let pillRadius: CGFloat = 28
    static let screenHorizontalPadding: CGFloat = 20
    static let screenTopPadding: CGFloat = 64
    static let bottomContentPadding: CGFloat = 126
}

struct CinematicBackdrop: View {
    var body: some View {
        ZStack {
            LinearGradient(
                colors: [
                    .arDeepBg,
                    Color(red: 0.105, green: 0.093, blue: 0.082),
                    Color.arBg
                ],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )

            Ellipse()
                .fill(Color(red: 0.576, green: 0.431, blue: 0.333).opacity(0.18))
                .frame(width: 340, height: 260)
                .blur(radius: 98)
                .offset(x: 150, y: -250)

            Circle()
                .fill(Color.arAccent.opacity(0.07))
                .frame(width: 250, height: 250)
                .blur(radius: 100)
                .offset(x: -170, y: 250)
        }
        .ignoresSafeArea()
    }
}

struct CinematicScreen<Content: View>: View {
    var title: String
    var subtitle: String
    var icon: String
    @ViewBuilder var content: Content

    var body: some View {
        ScrollView(.vertical, showsIndicators: true) {
            VStack(alignment: .leading, spacing: 16) {
                CinematicHeader(title: title, subtitle: subtitle, icon: icon)
                content
            }
            .padding(.horizontal, AntirotCinematicMetrics.screenHorizontalPadding)
            .padding(.top, AntirotCinematicMetrics.screenTopPadding)
            .padding(.bottom, AntirotCinematicMetrics.bottomContentPadding)
        }
        .background(CinematicBackdrop())
    }
}

struct CinematicHeader: View {
    let title: String
    let subtitle: String
    let icon: String

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            VStack(alignment: .leading, spacing: 3) {
                Text(title)
                    .font(.system(size: 29, weight: .bold, design: .rounded))
                    .foregroundStyle(.arTextPrimary)
                Text(subtitle)
                    .font(.subheadline.weight(.medium))
                    .foregroundStyle(.arTextSecondary)
                    .lineLimit(2)
            }

            Spacer(minLength: 0)

            Image(systemName: icon)
                .font(.subheadline.weight(.bold))
                .foregroundStyle(icon == "waveform.path.ecg" ? .arAccent : .arTextSecondary)
                .frame(width: 42, height: 42)
                .background(
                    Circle().fill(Color.white.opacity(0.07))
                )
                .overlay(Circle().stroke(Color.arBorderActive, lineWidth: 0.6))
        }
        .padding(16)
        .smokedGlass(cornerRadius: 26, shadow: false)
    }
}

struct CinematicGlassCard<Content: View>: View {
    var padding: CGFloat = 16
    var accent: Color = .arAccent
    @ViewBuilder var content: Content

    var body: some View {
        content
            .padding(padding)
            .overlay(alignment: .topLeading) {
                RoundedRectangle(cornerRadius: AntirotCinematicMetrics.cardRadius, style: .continuous)
                    .fill(
                        LinearGradient(
                            colors: [accent.opacity(0.08), .clear],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        )
                    )
                    .allowsHitTesting(false)
            }
            .smokedGlass(cornerRadius: AntirotCinematicMetrics.cardRadius)
    }
}

struct CinematicKicker: View {
    let title: String
    var icon: String?
    var tint: Color = .arAccent

    var body: some View {
        HStack(spacing: 8) {
            if let icon {
                Image(systemName: icon)
                    .font(.caption.weight(.bold))
                    .foregroundStyle(tint)
            }
            Text(title.uppercased())
                .font(.caption2.weight(.bold))
                .tracking(1.35)
                .foregroundStyle(.arTextSecondary)
            Spacer()
        }
    }
}

struct CinematicMetricTile: View {
    let title: String
    let value: String
    let icon: String
    var tint: Color = .arAccent

    var body: some View {
        CinematicGlassCard(padding: 14, accent: tint) {
            VStack(alignment: .leading, spacing: 10) {
                Image(systemName: icon)
                    .font(.subheadline.weight(.bold))
                    .foregroundStyle(tint)
                Text(value)
                    .font(.system(size: 24, weight: .bold, design: .rounded))
                    .foregroundStyle(.arTextPrimary)
                    .lineLimit(1)
                    .minimumScaleFactor(0.68)
                Text(title)
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.arTextSecondary)
                    .lineLimit(2)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .frame(maxWidth: .infinity, minHeight: 92, alignment: .topLeading)
        }
    }
}

struct CinematicActionRow: View {
    let title: String
    let subtitle: String
    let icon: String
    var tint: Color = .arAccent
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 12) {
                Image(systemName: icon)
                    .font(.headline.weight(.semibold))
                    .foregroundStyle(tint)
                    .frame(width: 34, height: 34)
                    .background(Circle().fill(tint.opacity(0.14)))

                VStack(alignment: .leading, spacing: 2) {
                    Text(title)
                        .font(.subheadline.weight(.semibold))
                        .foregroundStyle(.arTextPrimary)
                    Text(subtitle)
                        .font(.caption)
                        .foregroundStyle(.arTextSecondary)
                        .lineLimit(2)
                }

                Spacer(minLength: 8)

                Image(systemName: "chevron.right")
                    .font(.caption.weight(.bold))
                    .foregroundStyle(.arTextMuted)
            }
        }
        .buttonStyle(.plain)
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

struct SmokedGlassModifier: ViewModifier {
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    @Environment(\.colorSchemeContrast) private var colorSchemeContrast

    var cornerRadius: CGFloat
    var tint: Color
    var castsShadow: Bool

    func body(content: Content) -> some View {
        let shape = RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
        let borderOpacity = colorSchemeContrast == .increased ? 0.34 : 0.18

        content
            .background {
                if reduceTransparency {
                    shape.fill(Color.arSurface)
                } else {
                    shape.fill(.ultraThinMaterial)
                }
            }
            .background(shape.fill(tint.opacity(reduceTransparency ? 0.72 : 0.48)))
            .overlay {
                shape.stroke(
                    LinearGradient(
                        colors: [
                            Color.white.opacity(borderOpacity),
                            Color.white.opacity(0.035),
                            Color.black.opacity(0.16)
                        ],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    ),
                    lineWidth: colorSchemeContrast == .increased ? 0.9 : 0.65
                )
                .allowsHitTesting(false)
            }
            .overlay(alignment: .top) {
                shape
                    .fill(
                        LinearGradient(
                            colors: [Color.white.opacity(0.09), .clear],
                            startPoint: .top,
                            endPoint: .center
                        )
                    )
                    .allowsHitTesting(false)
            }
            .clipShape(shape)
            .shadow(
                color: castsShadow ? Color.black.opacity(0.32) : .clear,
                radius: castsShadow ? 22 : 0,
                y: castsShadow ? 12 : 0
            )
    }
}

extension View {
    func smokedGlass(
        cornerRadius: CGFloat = AntirotCinematicMetrics.cardRadius,
        tint: Color = .arGlassTint,
        shadow: Bool = true
    ) -> some View {
        modifier(SmokedGlassModifier(cornerRadius: cornerRadius, tint: tint, castsShadow: shadow))
    }
}

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
            .background(
                RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                    .fill(Color.arGlassTint.opacity(0.42))
            )
            .overlay(
                RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                    .stroke(
                        LinearGradient(
                            colors: [Color.white.opacity(0.22), Color.white.opacity(0.04)],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        ),
                        lineWidth: borderWidth
                    )
                    .allowsHitTesting(false)
            )
            .overlay(alignment: .top) {
                if sheen {
                    RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                        .fill(
                            LinearGradient(
                                colors: [Color.white.opacity(0.12), Color.clear],
                                startPoint: .top,
                                endPoint: .center
                            )
                        )
                        .clipShape(RoundedRectangle(cornerRadius: cornerRadius, style: .continuous))
                        .allowsHitTesting(false)
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
