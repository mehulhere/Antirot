import SwiftUI

// MARK: - Primary Action Button

/// One large, circular, thumb-friendly action. Designed to feel physical and
/// important: an accent gradient fill, a specular highlight, a double shadow
/// for depth, and a springy press. The title sits as a quiet caption beneath
/// the circle so the icon stays the hero.
struct PrimaryActionButton: View {
    let title: String
    let systemImage: String
    var isBusy: Bool = false
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(spacing: 11) {
                ZStack {
                    Circle()
                        .fill(LinearGradient.antirotAccent)

                    Circle()
                        .stroke(Color.white.opacity(0.22), lineWidth: 0.5)

                                        // Specular highlight — reads as glassy/metallic, not flat.
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
                            .font(.system(size: 30, weight: .semibold))
                            .foregroundStyle(.white)
                    }
                }
                .frame(width: 88, height: 88)
                .shadow(color: Color.arAccent.opacity(0.42), radius: 28, y: 12)
                .shadow(color: .black.opacity(0.40), radius: 16, y: 8)

                Text(title)
                    .font(.system(size: 24, weight: .bold, design: .rounded))
                    .foregroundStyle(.arTextPrimary)
            }
        }
        .buttonStyle(PrimaryActionButtonStyle())
        .disabled(isBusy)
    }
}

private struct PrimaryActionButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed ? 0.93 : 1.0)
            .animation(.spring(response: 0.28, dampingFraction: 0.6), value: configuration.isPressed)
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
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background(.ultraThinMaterial, in: Capsule(style: .continuous))
            .background(Color.white.opacity(0.045), in: Capsule(style: .continuous))
            .overlay(Capsule(style: .continuous).stroke(Color.white.opacity(0.10), lineWidth: 0.6))
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
