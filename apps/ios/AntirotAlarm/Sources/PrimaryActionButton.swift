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
                .shadow(color: Color.arAccent.opacity(0.32), radius: 22, y: 10)
                .shadow(color: .black.opacity(0.40), radius: 16, y: 8)

                Text(title)
                    .font(.subheadline.weight(.semibold))
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
                    .font(.caption.weight(.semibold))
                Text(title)
                    .font(.subheadline.weight(.medium))
            }
            .foregroundStyle(.arTextSecondary)
            .padding(.horizontal, 18)
            .padding(.vertical, 11)
            .glassCapsule()
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
