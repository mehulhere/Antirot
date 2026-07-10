import SwiftUI

// MARK: - Editorial Operator Palette

enum AntirotEditorialPalette {
    static let inkRed = 8.0 / 255.0
    static let inkGreen = 8.0 / 255.0
    static let inkBlue = 7.0 / 255.0

    static let graphiteRed = 23.0 / 255.0
    static let graphiteGreen = 23.0 / 255.0
    static let graphiteBlue = 20.0 / 255.0

    static let paperRed = 240.0 / 255.0
    static let paperGreen = 236.0 / 255.0
    static let paperBlue = 226.0 / 255.0

    static let signalOrangeRed = 228.0 / 255.0
    static let signalOrangeGreen = 91.0 / 255.0
    static let signalOrangeBlue = 44.0 / 255.0
}

enum AntirotEditorialMetrics {
    static let maximumRadius: CGFloat = 16
    static let sectionRadius: CGFloat = 4
    static let horizontalInset: CGFloat = 20
    static let ruleWidth: CGFloat = 1
}

extension Color {
    static let arBg = Color(
        red: AntirotEditorialPalette.inkRed,
        green: AntirotEditorialPalette.inkGreen,
        blue: AntirotEditorialPalette.inkBlue
    )
    static let arDeepBg = arBg
    static let arSurface = Color(
        red: AntirotEditorialPalette.graphiteRed,
        green: AntirotEditorialPalette.graphiteGreen,
        blue: AntirotEditorialPalette.graphiteBlue
    )
    static let arElevated = Color(red: 0.118, green: 0.114, blue: 0.102)
    static let arGlassTint = arSurface
    static let arOverlay = Color.black.opacity(0.74)

    static let arAccent = Color(
        red: AntirotEditorialPalette.signalOrangeRed,
        green: AntirotEditorialPalette.signalOrangeGreen,
        blue: AntirotEditorialPalette.signalOrangeBlue
    )
    static let arAccentDim = Color(red: 0.650, green: 0.220, blue: 0.090)
    static let arAccentSubtle = arAccent.opacity(0.12)
    static let arCyan = Color(red: 0.420, green: 0.690, blue: 0.740)
    static let arAmber = Color(red: 0.910, green: 0.640, blue: 0.235)

    static let arTextPrimary = Color(
        red: AntirotEditorialPalette.paperRed,
        green: AntirotEditorialPalette.paperGreen,
        blue: AntirotEditorialPalette.paperBlue
    )
    static let arTextSecondary = Color(red: 0.650, green: 0.635, blue: 0.600)
    static let arTextMuted = Color(red: 0.540, green: 0.529, blue: 0.498)

    static let arBorder = Color(red: 0.204, green: 0.196, blue: 0.180)
    static let arBorderActive = arTextPrimary.opacity(0.52)

    // Semantic (used sparingly, never decoratively)
    static let arSuccess = Color(red: 0.282, green: 0.780, blue: 0.455)
    static let arWarning = Color(red: 0.910, green: 0.640, blue: 0.235)
    static let arDanger = Color(red: 0.898, green: 0.282, blue: 0.302)
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
            .background(Color.arSurface)
            .clipShape(
                RoundedRectangle(
                    cornerRadius: min(cornerRadius, AntirotEditorialMetrics.maximumRadius),
                    style: .continuous
                )
            )
            .overlay {
                RoundedRectangle(
                    cornerRadius: min(cornerRadius, AntirotEditorialMetrics.maximumRadius),
                    style: .continuous
                )
                .stroke(Color.arBorder, lineWidth: AntirotEditorialMetrics.ruleWidth)
            }
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
            .frame(height: AntirotEditorialMetrics.ruleWidth)
    }
}

// MARK: - State Pill

struct StatePill: View {
    let label: String
    var isActive: Bool = false

    var body: some View {
        HStack(spacing: 8) {
            Rectangle()
                .fill(isActive ? Color.arAccent : Color.arBorder)
                .frame(width: 12, height: 2)
            Text(label.uppercased())
                .font(.caption.monospaced().weight(.semibold))
                .tracking(1.0)
                .foregroundStyle(isActive ? .arTextPrimary : .arTextSecondary)
        }
    }
}

// MARK: - Cinematic App System

enum AntirotCinematicMetrics {
    static let cardRadius: CGFloat = 4
    static let pillRadius: CGFloat = 12
    static let screenHorizontalPadding: CGFloat = AntirotEditorialMetrics.horizontalInset
    static let screenTopPadding: CGFloat = 32
    static let bottomContentPadding: CGFloat = 32
}

struct CinematicBackdrop: View {
    var body: some View {
        Color.arBg.ignoresSafeArea()
    }
}

struct CinematicScreen<Content: View>: View {
    var title: String
    var subtitle: String
    var icon: String
    @ViewBuilder var content: Content

    var body: some View {
        ScrollView(.vertical, showsIndicators: true) {
            VStack(alignment: .leading, spacing: 32) {
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
            VStack(alignment: .leading, spacing: 8) {
                Text(title)
                    .font(.largeTitle.weight(.medium))
                    .fontDesign(.serif)
                    .foregroundStyle(.arTextPrimary)
                Text(subtitle)
                    .font(.subheadline)
                    .foregroundStyle(.arTextSecondary)
                    .fixedSize(horizontal: false, vertical: true)
            }

            Spacer(minLength: 0)

            Image(systemName: icon)
                .font(.headline.weight(.medium))
                .foregroundStyle(icon == "waveform.path.ecg" ? .arAccent : .arTextSecondary)
                .frame(width: 44, height: 44, alignment: .topTrailing)
        }
        .padding(.bottom, 20)
        .overlay(alignment: .bottom) {
            SectionDivider()
        }
    }
}

struct CinematicGlassCard<Content: View>: View {
    var padding: CGFloat = 16
    var accent: Color = .arAccent
    @ViewBuilder var content: Content

    var body: some View {
        content
            .padding(padding)
            .background(Color.arSurface)
            .overlay(alignment: .leading) {
                Rectangle()
                    .fill(accent)
                    .frame(width: 3)
            }
            .clipShape(RoundedRectangle(cornerRadius: AntirotCinematicMetrics.cardRadius, style: .continuous))
            .overlay {
                RoundedRectangle(cornerRadius: AntirotCinematicMetrics.cardRadius, style: .continuous)
                    .stroke(Color.arBorder, lineWidth: AntirotEditorialMetrics.ruleWidth)
            }
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
                .font(.caption2.monospaced().weight(.semibold))
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
                    .font(.title2.weight(.bold))
                    .fontDesign(.rounded)
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
            .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))
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
            .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 4).stroke(Color.arBorder, lineWidth: 1))
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
            .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))
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
            .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))
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
    @Environment(\.colorSchemeContrast) private var colorSchemeContrast

    var cornerRadius: CGFloat
    var tint: Color
    var castsShadow: Bool

    func body(content: Content) -> some View {
        let resolvedRadius = min(cornerRadius, AntirotEditorialMetrics.maximumRadius)
        let shape = RoundedRectangle(cornerRadius: resolvedRadius, style: .continuous)

        content
            .background(shape.fill(tint))
            .overlay {
                shape.stroke(Color.arBorder, lineWidth: colorSchemeContrast == .increased ? 2 : 1)
                .allowsHitTesting(false)
            }
            .clipShape(shape)
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
        let resolvedRadius = min(cornerRadius, AntirotEditorialMetrics.maximumRadius)
        content
            .background(
                RoundedRectangle(cornerRadius: resolvedRadius, style: .continuous)
                    .fill(Color.arSurface)
            )
            .overlay(
                RoundedRectangle(cornerRadius: resolvedRadius, style: .continuous)
                    .stroke(Color.arBorder, lineWidth: max(borderWidth, 1))
                    .allowsHitTesting(false)
            )
            .clipShape(RoundedRectangle(cornerRadius: resolvedRadius, style: .continuous))
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
            .liquidGlass(cornerRadius: 12, borderWidth: 1, sheen: false)
    }
}
