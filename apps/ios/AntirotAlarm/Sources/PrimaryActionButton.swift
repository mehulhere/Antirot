import SwiftUI

// MARK: - Primary Action Button

/// The dominant state action: a substantial smoked-glass control with a
/// decisive red core, clear label, and restrained physical depth.
struct PrimaryActionButton: View {
    let title: String
    let systemImage: String
    var isBusy: Bool = false
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 14) {
                ZStack {
                    Circle()
                        .fill(LinearGradient.antirotAccent)

                    Circle()
                        .stroke(Color.white.opacity(0.22), lineWidth: 0.5)

                    Circle()
                        .fill(
                            LinearGradient(
                                colors: [Color.white.opacity(0.30), .clear],
                                startPoint: .topLeading,
                                endPoint: .bottomTrailing
                            )
                        )

                    if isBusy {
                        ProgressView()
                            .tint(.white)
                    } else {
                        Image(systemName: systemImage)
                            .font(.system(size: 22, weight: .semibold))
                            .foregroundStyle(.white)
                    }
                }
                .frame(width: 54, height: 54)
                .shadow(color: Color.arAccent.opacity(0.34), radius: 18, y: 8)

                Text(title)
                    .font(.title3.weight(.bold))
                    .fontDesign(.rounded)
                    .foregroundStyle(.arTextPrimary)
                    .fixedSize(horizontal: false, vertical: true)

                Spacer(minLength: 8)

                Image(systemName: "arrow.up.right")
                    .font(.subheadline.weight(.bold))
                    .foregroundStyle(.arTextPrimary)
                    .frame(width: 44, height: 44)
                    .background(Circle().fill(Color.white.opacity(0.08)))
            }
            .padding(10)
            .frame(maxWidth: .infinity, minHeight: 74)
            .background(Color.arAccent.opacity(0.08), in: RoundedRectangle(cornerRadius: 26, style: .continuous))
            .smokedGlass(cornerRadius: 26, tint: .arSurface)
            .overlay(
                RoundedRectangle(cornerRadius: 26, style: .continuous)
                    .stroke(Color.arAccent.opacity(0.22), lineWidth: 0.7)
            )
        }
        .buttonStyle(PrimaryActionButtonStyle())
        .disabled(isBusy)
    }
}

private struct PrimaryActionButtonStyle: ButtonStyle {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed ? 0.975 : 1.0)
            .animation(
                reduceMotion ? .easeOut(duration: 0.1) : .spring(response: 0.28, dampingFraction: 0.72),
                value: configuration.isPressed
            )
    }
}

// MARK: - Secondary Action Button

/// A visually quiet glass pill. Subordinate to the primary circle so the user
/// always knows which action is dominant in the current state.
struct SecondaryActionButton: View {
    let title: String
    let systemImage: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 6) {
                Image(systemName: systemImage)
                    .font(.caption2.weight(.bold))
                Text(title)
                    .font(.caption.weight(.semibold))
            }
            .foregroundStyle(.arTextSecondary)
            .padding(.horizontal, 14)
            .frame(minHeight: 44)
            .smokedGlass(cornerRadius: 22, tint: .arSurface, shadow: false)
        }
        .buttonStyle(.plain)
    }
}

// MARK: - Confetti Overlay

/// Shows a `ConfettiBurst` over any content while `trigger` is true, then
/// resets the binding so the burst can fire again on the next Start.
struct ConfettiOverlay: ViewModifier {
    @Binding var trigger: Bool

    func body(content: Content) -> some View {
        content.overlay {
            if trigger {
                ConfettiBurst {
                    withAnimation(.easeOut(duration: 0.2)) {
                        trigger = false
                    }
                }
            }
        }
    }
}

extension View {
    func confettiOverlay(trigger: Binding<Bool>) -> some View {
        modifier(ConfettiOverlay(trigger: trigger))
    }
}
