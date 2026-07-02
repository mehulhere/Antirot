import SwiftUI

// MARK: - Glass Chat Sheet

/// A bottom-anchored, draggable, translucent glass chat sheet with three snap
/// points (collapsed, half, full). The strong blur keeps the coach scene
/// visible behind it while text stays readable. Voice-first composer at the
/// bottom; the latest coach one-liner is shown when collapsed.
struct GlassSheet: View {
    @Binding var height: CGFloat

    var messages: [CoachMessage]
    @Binding var draft: String
    var isRecording: Bool
    var isSending: Bool
    var statusText: String
    var latestOneLiner: String

    var onMic: () -> Void
    var onSend: () -> Void

    @State private var dragStartHeight: CGFloat = 0

    private let collapsedHeight: CGFloat = 118
    private let halfFraction: CGFloat = 0.5
    private let fullFraction: CGFloat = 0.9

    var body: some View {
        GeometryReader { proxy in
            let available = proxy.size.height
            let half = available * halfFraction
            let full = available * fullFraction
            let resolved = min(max(height, collapsedHeight), full)

            VStack(spacing: 0) {
                Spacer()
                sheetContent(half: half, full: full, resolved: resolved)
                    .frame(height: resolved)
                    .padding(.horizontal, 10)
                    .padding(.bottom, 10)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)
    }

    // MARK: - Snap Helpers

    private func detents(half: CGFloat, full: CGFloat) -> [CGFloat] {
        [collapsedHeight, half, full]
    }

    private func nearestDetent(to value: CGFloat, half: CGFloat, full: CGFloat) -> CGFloat {
        let detents = detents(half: half, full: full)
        let clamped = min(max(value, collapsedHeight), full)
        return detents.min(by: { abs($0 - clamped) < abs($1 - clamped) }) ?? collapsedHeight
    }

    // MARK: - Sheet Content

    @ViewBuilder
    private func sheetContent(half: CGFloat, full: CGFloat, resolved: CGFloat) -> some View {
        let isCollapsed = resolved <= collapsedHeight + 14

        VStack(spacing: 0) {
            dragHandle(half: half, full: full)

            if isCollapsed {
                collapsedContent
            } else {
                expandedContent
            }
        }
        .liquidGlass(cornerRadius: 30, borderWidth: 0.7)
        .shadow(color: .black.opacity(0.38), radius: 24, y: -8)
    }

    private func dragHandle(half: CGFloat, full: CGFloat) -> some View {
        VStack(spacing: 6) {
            Capsule(style: .continuous)
                .fill(Color.white.opacity(0.28))
                .frame(width: 38, height: 5)
                .padding(.top, 8)
                .padding(.bottom, 2)

            if !statusText.isEmpty {
                Text(statusText)
                    .font(.caption2)
                    .foregroundStyle(.arTextSecondary)
                    .lineLimit(1)
            }
        }
        .frame(maxWidth: .infinity)
        .contentShape(Rectangle())
        .gesture(
            DragGesture(minimumDistance: 2)
                .onChanged { value in
                    let next = dragStartHeight - value.translation.height
                    withAnimation(.spring()) {
                        height = min(max(next, collapsedHeight), full)
                    }
                }
                .onEnded { value in
                    let projected = height - value.predictedEndTranslation.height * 0.18
                    withAnimation(.spring(response: 0.34, dampingFraction: 0.82)) {
                        height = nearestDetent(to: projected, half: half, full: full)
                    }
                }
        )
        .onAppear { dragStartHeight = height }
        .onChange(of: height) { _, newValue in dragStartHeight = newValue }
    }
    // MARK: - Collapsed

    private var collapsedContent: some View {
        HStack(spacing: 12) {
            VStack(alignment: .leading, spacing: 4) {
                Text("Coach")
                    .font(.caption2.weight(.bold))
                    .tracking(1.0)
                    .foregroundStyle(.arTextSecondary)

                Text(latestOneLiner)
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(.arTextPrimary)
                    .lineLimit(2)
                    .multilineTextAlignment(.leading)
            }
            .frame(maxWidth: .infinity, alignment: .leading)

            Spacer(minLength: 8)

            micButton(size: 48)
        }
        .padding(.horizontal, 18)
        .padding(.vertical, 13)
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    // MARK: - Expanded

    private var expandedContent: some View {
        VStack(spacing: 0) {
            chatList
            composer
        }
    }

    private var chatList: some View {
        ScrollViewReader { proxy in
            ScrollView(.vertical, showsIndicators: false) {
                LazyVStack(spacing: 10) {
                    ForEach(messages) { message in
                        GlassChatRow(message: message)
                            .id(message.id)
                    }

                    if isSending {
                        HStack(spacing: 8) {
                            ProgressView()
                                .tint(.arTextSecondary)
                            Text(statusText)
                                .font(.caption)
                                .foregroundStyle(.arTextSecondary)
                            Spacer()
                        }
                        .padding(.horizontal, 14)
                        .padding(.vertical, 10)
                        .id("thinking-indicator")
                    }
                }
                .padding(.horizontal, 16)
                .padding(.top, 10)
                .padding(.bottom, 16)
            }
            .onChange(of: messages.count) { _, _ in
                if let last = messages.last?.id {
                    withAnimation(.easeOut(duration: 0.25)) {
                        proxy.scrollTo(last, anchor: .bottom)
                    }
                }
            }
        }
    }

    // MARK: - Composer (voice-first)

    private var composer: some View {
        let hasDraft = !draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty

        return HStack(spacing: 10) {
            micButton(size: 52)

            TextField("Type...", text: $draft, axis: .vertical)
                .lineLimit(1...4)
                .textInputAutocapitalization(.sentences)
                .font(.body)
                .foregroundStyle(.arTextPrimary)
                .padding(.horizontal, 14)
                .padding(.vertical, 11)
                .background(
                    RoundedRectangle(cornerRadius: 18, style: .continuous)
                        .fill(Color.black.opacity(0.22))
                )
                .overlay(
                    RoundedRectangle(cornerRadius: 18, style: .continuous)
                        .stroke(Color.white.opacity(0.08), lineWidth: 0.5)
                )

            if hasDraft {
                Button(action: onSend) {
                    Image(systemName: "arrow.up")
                        .font(.subheadline.weight(.bold))
                        .foregroundStyle(.white)
                        .frame(width: 42, height: 42)
                        .background(Circle().fill(Color.arAccent))
                }
                .buttonStyle(.plain)
                .disabled(isSending)
                .transition(.scale.combined(with: .opacity))
            }
        }
        .padding(.horizontal, 14)
        .padding(.top, 8)
        .padding(.bottom, 14)
        .animation(.spring(duration: 0.3), value: hasDraft)
    }

    @ViewBuilder
    private func micButton(size: CGFloat) -> some View {
        Button(action: onMic) {
            Image(systemName: isRecording ? "stop.fill" : "mic.fill")
                .font(.system(size: size * 0.42, weight: .semibold))
                .foregroundStyle(.white)
                .frame(width: size, height: size)
                .background(
                    Circle()
                        .fill(isRecording ? Color.arDanger : Color.arAccent)
                )
                .scaleEffect(isRecording ? 1.06 : 1.0)
                .animation(
                    isRecording ? .easeInOut(duration: 0.8).repeatForever(autoreverses: true) : .default,
                    value: isRecording
                )
        }
        .buttonStyle(.plain)
        .disabled(isSending)
    }
}
// MARK: - Glass Chat Row

private struct GlassChatRow: View {
    let message: CoachMessage

    var body: some View {
        if message.role == .system {
            Text(message.text)
                .font(.caption2)
                .foregroundStyle(.arTextSecondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: .infinity)
                .padding(.vertical, 3)
        } else {
            chatBubble
        }
    }

    private var chatBubble: some View {
        let isUser = message.role == .user

        return HStack {
            if isUser { Spacer(minLength: 48) }

            VStack(alignment: isUser ? .trailing : .leading, spacing: 4) {
                if message.audioFileURL != nil {
                    Label("Voice message", systemImage: "play.circle.fill")
                        .font(.body.weight(.medium))
                        .foregroundStyle(.arTextPrimary)
                } else {
                    Text(message.text)
                        .font(.body)
                        .foregroundStyle(.arTextPrimary)
                        .lineSpacing(3)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 10)
            .background(
                RoundedRectangle(cornerRadius: 16, style: .continuous)
                    .fill(isUser ? Color.arAccent.opacity(0.20) : Color.white.opacity(0.08))
            )

            if !isUser { Spacer(minLength: 48) }
        }
    }
}

// MARK: - Preview

#Preview {
    ZStack {
        Color.arBg.ignoresSafeArea()
        GlassSheet(
            height: .constant(108),
            messages: [
                CoachMessage(role: .coach, text: "I'm watching. What's the move?"),
                CoachMessage(role: .user, text: "Starting now.")
            ],
            draft: .constant(""),
            isRecording: false,
            isSending: false,
            statusText: "Ready",
            latestOneLiner: "Watching.",
            onMic: {},
            onSend: {}
        )
    }
}
