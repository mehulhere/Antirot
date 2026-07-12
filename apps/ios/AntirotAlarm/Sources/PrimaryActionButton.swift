import SwiftUI

// MARK: - Primary Action Button

/// The dominant state action: one clear editorial block with no ornamental
/// glass, glow, or competing controls.
struct PrimaryActionButton: View {
    let title: String
    let systemImage: String
    var isBusy: Bool = false
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 16) {
                if isBusy {
                    ProgressView()
                        .tint(.arDeepBg)
                } else {
                    Image(systemName: systemImage)
                        .font(.system(size: 18, weight: .bold))
                }

                Text(title)
                    .font(.system(.title2, design: .serif, weight: .semibold))
                    .fixedSize(horizontal: false, vertical: true)

                Spacer(minLength: 8)

                Image(systemName: "arrow.up.right")
                    .font(.system(size: 18, weight: .bold))
            }
            .foregroundStyle(.arDeepBg)
            .padding(.horizontal, 20)
            .frame(maxWidth: .infinity, minHeight: 76)
            .background(Color.arAccent, in: RoundedRectangle(cornerRadius: 4, style: .continuous))
        }
        .buttonStyle(PrimaryActionButtonStyle())
        .disabled(isBusy)
    }
}

private struct PrimaryActionButtonStyle: ButtonStyle {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .opacity(configuration.isPressed ? 0.78 : 1.0)
            .animation(
                reduceMotion ? .easeOut(duration: 0.1) : .spring(response: 0.28, dampingFraction: 0.72),
                value: configuration.isPressed
            )
    }
}

// MARK: - Secondary Action Button

/// A quiet text action subordinate to the primary block.
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
                    .font(.system(size: 13, weight: .semibold, design: .monospaced))
                    .textCase(.uppercase)
            }
            .foregroundStyle(.arTextPrimary)
            .padding(.horizontal, 10)
            .frame(minHeight: 44)
            .overlay(alignment: .bottom) {
                Rectangle()
                    .fill(Color.arBorder)
                    .frame(height: 1)
            }
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
