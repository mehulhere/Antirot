import SwiftUI

enum ChatSheetDetents {
    static let collapsedHeight: CGFloat = 118
    static let halfFraction: CGFloat = 0.5
    static let fullFraction: CGFloat = 0.96

    static func halfHeight(availableHeight: CGFloat) -> CGFloat {
        availableHeight * halfFraction
    }

    static func fullHeight(availableHeight: CGFloat) -> CGFloat {
        availableHeight * fullFraction
    }

    static func heights(availableHeight: CGFloat) -> [CGFloat] {
        [
            collapsedHeight,
            halfHeight(availableHeight: availableHeight),
            fullHeight(availableHeight: availableHeight)
        ]
    }

    static func nearestHeight(to value: CGFloat, availableHeight: CGFloat) -> CGFloat {
        let full = fullHeight(availableHeight: availableHeight)
        let clamped = min(max(value, collapsedHeight), full)
        return heights(availableHeight: availableHeight)
            .min(by: { abs($0 - clamped) < abs($1 - clamped) }) ?? collapsedHeight
    }

    static func nextExpandedHeight(from current: CGFloat, availableHeight: CGFloat) -> CGFloat {
        let half = halfHeight(availableHeight: availableHeight)
        let full = fullHeight(availableHeight: availableHeight)
        return current < half - 8 ? half : full
    }

    static func nextCollapsedHeight(from current: CGFloat, availableHeight: CGFloat) -> CGFloat {
        let half = halfHeight(availableHeight: availableHeight)
        return current > half + 8 ? half : collapsedHeight
    }

    static func liveHeight(
        from start: CGFloat,
        translationY: CGFloat,
        availableHeight: CGFloat
    ) -> CGFloat {
        let full = fullHeight(availableHeight: availableHeight)
        let next = start - translationY
        return min(max(next, collapsedHeight), full)
    }

    static func finalHeight(
        from start: CGFloat,
        predictedEndTranslationY: CGFloat,
        availableHeight: CGFloat
    ) -> CGFloat {
        let projected = start - predictedEndTranslationY * 0.18
        return nearestHeight(to: projected, availableHeight: availableHeight)
    }
}

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
    @FocusState private var isDraftFocused: Bool

    var body: some View {
        GeometryReader { proxy in
            let available = proxy.size.height
            let half = ChatSheetDetents.halfHeight(availableHeight: available)
            let full = ChatSheetDetents.fullHeight(availableHeight: available)
            let resolved = min(max(height, ChatSheetDetents.collapsedHeight), full)

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

    // MARK: - Sheet Content

    @ViewBuilder
    private func sheetContent(half: CGFloat, full: CGFloat, resolved: CGFloat) -> some View {
        let isCollapsed = resolved <= ChatSheetDetents.collapsedHeight + 14
        let isFull = resolved >= full - 8

        VStack(spacing: 0) {
            dragHandle(half: half, available: full / ChatSheetDetents.fullFraction)

            if isCollapsed {
                collapsedContent(available: full / ChatSheetDetents.fullFraction)
            } else {
                expandedContent(isFull: isFull)
            }
        }
        .contentShape(RoundedRectangle(cornerRadius: 30, style: .continuous))
        .liquidGlass(cornerRadius: 30, borderWidth: 0.7)
        .shadow(color: .black.opacity(0.38), radius: 24, y: -8)
    }

    private func sheetDragGesture(availableHeight: CGFloat) -> some Gesture {
        DragGesture(minimumDistance: 8)
            .onChanged { value in
                if dragStartHeight == 0 {
                    dragStartHeight = height
                }
                withAnimation(.interactiveSpring(response: 0.28, dampingFraction: 0.84)) {
                    height = ChatSheetDetents.liveHeight(
                        from: dragStartHeight,
                        translationY: value.translation.height,
                        availableHeight: availableHeight
                    )
                }
            }
            .onEnded { value in
                let start = dragStartHeight == 0 ? height : dragStartHeight
                withAnimation(.spring(response: 0.34, dampingFraction: 0.82)) {
                    height = ChatSheetDetents.finalHeight(
                        from: start,
                        predictedEndTranslationY: value.predictedEndTranslation.height,
                        availableHeight: availableHeight
                    )
                }
                dragStartHeight = 0
            }
    }

    private func dragHandle(half: CGFloat, available: CGFloat) -> some View {
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
        .frame(minHeight: 44)
        .contentShape(Rectangle())
        .simultaneousGesture(sheetDragGesture(availableHeight: available))
        .onTapGesture {
            withAnimation(.spring(response: 0.34, dampingFraction: 0.82)) {
                height = height <= ChatSheetDetents.collapsedHeight + 14
                    ? half
                    : ChatSheetDetents.collapsedHeight
            }
        }
        .accessibilityLabel("Coach chat")
        .accessibilityHint("Tap to expand or collapse")
    }
    // MARK: - Collapsed

    private func collapsedContent(available: CGFloat) -> some View {
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
            .contentShape(Rectangle())
            .onTapGesture {
                withAnimation(.spring(response: 0.34, dampingFraction: 0.82)) {
                    height = max(height, ChatSheetDetents.halfHeight(availableHeight: available))
                }
            }

            Spacer(minLength: 8)

            micButton(size: 48)
        }
        .padding(.horizontal, 18)
        .padding(.vertical, 13)
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    // MARK: - Expanded

    private func expandedContent(isFull: Bool) -> some View {
        VStack(spacing: 0) {
            chatList(isFull: isFull)
            composer
        }
    }

    private func chatList(isFull: Bool) -> some View {
        ScrollViewReader { proxy in
            ScrollView(.vertical, showsIndicators: isFull) {
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
            .scrollDisabled(!isFull)
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
                .focused($isDraftFocused)
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
                .onTapGesture {
                    isDraftFocused = true
                }
                .submitLabel(.send)
                .onSubmit(onSend)

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
        .contentShape(Rectangle())
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
