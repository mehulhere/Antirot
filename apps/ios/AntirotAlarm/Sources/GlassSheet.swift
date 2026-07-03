import SwiftUI

enum ChatSheetDetents {
    static let collapsedHeight: CGFloat = 118
    static let fullFraction: CGFloat = 0.96

    static func fullHeight(availableHeight: CGFloat) -> CGFloat {
        availableHeight * fullFraction
    }

    static func heights(availableHeight: CGFloat) -> [CGFloat] {
        [
            collapsedHeight,
            fullHeight(availableHeight: availableHeight)
        ]
    }

    static func nearestHeight(to value: CGFloat, availableHeight: CGFloat) -> CGFloat {
        let full = fullHeight(availableHeight: availableHeight)
        let clamped = min(max(value, collapsedHeight), full)
        return heights(availableHeight: availableHeight)
            .min(by: { abs($0 - clamped) < abs($1 - clamped) }) ?? collapsedHeight
    }

    static func nextExpandedHeight(from _: CGFloat, availableHeight: CGFloat) -> CGFloat {
        fullHeight(availableHeight: availableHeight)
    }

    static func nextCollapsedHeight(from _: CGFloat, availableHeight: CGFloat) -> CGFloat {
        collapsedHeight
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

    static func offsetY(for visibleHeight: CGFloat, availableHeight: CGFloat) -> CGFloat {
        let full = fullHeight(availableHeight: availableHeight)
        let resolved = min(max(visibleHeight, collapsedHeight), full)
        return full - resolved
    }

    static func visibleHeight(
        committedHeight: CGFloat,
        dragTranslationY: CGFloat,
        availableHeight: CGFloat
    ) -> CGFloat {
        liveHeight(
            from: committedHeight,
            translationY: dragTranslationY,
            availableHeight: availableHeight
        )
    }

    static func finalHeight(
        from start: CGFloat,
        predictedEndTranslationY: CGFloat,
        availableHeight: CGFloat
    ) -> CGFloat {
        if predictedEndTranslationY < -12 {
            return fullHeight(availableHeight: availableHeight)
        }
        if predictedEndTranslationY > 12 {
            return collapsedHeight
        }
        let projected = start - predictedEndTranslationY * 0.18
        return nearestHeight(to: projected, availableHeight: availableHeight)
    }

    static func isCollapsed(_ height: CGFloat) -> Bool {
        height <= collapsedHeight + 14
    }

    static func showsCollapsedContent(committedHeight: CGFloat, dragTranslationY: CGFloat) -> Bool {
        isCollapsed(committedHeight) && dragTranslationY >= 0
    }
}

// MARK: - Glass Chat Sheet

/// A bottom-anchored, draggable, translucent glass chat sheet with two snap
/// points (collapsed and full). The strong blur keeps the coach scene
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
    var onPlayVoiceMessage: (URL) -> Void

    @GestureState private var dragTranslationY: CGFloat = 0
    @FocusState private var isDraftFocused: Bool

    var body: some View {
        GeometryReader { proxy in
            let available = proxy.size.height
            let full = ChatSheetDetents.fullHeight(availableHeight: available)
            let committed = min(max(height, ChatSheetDetents.collapsedHeight), full)
            let resolved = ChatSheetDetents.visibleHeight(
                committedHeight: committed,
                dragTranslationY: dragTranslationY,
                availableHeight: available
            )
            let offsetY = ChatSheetDetents.offsetY(
                for: resolved,
                availableHeight: available
            )
            let showCollapsedContent = ChatSheetDetents.showsCollapsedContent(
                committedHeight: committed,
                dragTranslationY: dragTranslationY
            )

            VStack(spacing: 0) {
                Spacer()
                sheetContent(
                    full: full,
                    showCollapsedContent: showCollapsedContent
                )
                    .frame(height: full)
                    .padding(.horizontal, 10)
                    .padding(.bottom, 10)
                    .offset(y: offsetY)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)
    }

    // MARK: - Snap Helpers

    // MARK: - Sheet Content

    @ViewBuilder
    private func sheetContent(full: CGFloat, showCollapsedContent: Bool) -> some View {
        VStack(spacing: 0) {
            dragHandle(
                full: full,
                available: full / ChatSheetDetents.fullFraction
            )

            if showCollapsedContent {
                collapsedContent(available: full / ChatSheetDetents.fullFraction)
            } else {
                expandedContent
            }
        }
        .contentShape(RoundedRectangle(cornerRadius: 30, style: .continuous))
        .liquidGlass(cornerRadius: 30, borderWidth: 0.7)
        .shadow(color: .black.opacity(0.38), radius: 24, y: -8)
    }

    private func sheetDragGesture(availableHeight: CGFloat) -> some Gesture {
        DragGesture(minimumDistance: 8)
            .updating($dragTranslationY) { value, state, transaction in
                transaction.disablesAnimations = true
                transaction.animation = nil
                state = value.translation.height
            }
            .onEnded { value in
                withAnimation(.spring(response: 0.22, dampingFraction: 0.86)) {
                    height = ChatSheetDetents.finalHeight(
                        from: height,
                        predictedEndTranslationY: value.predictedEndTranslation.height,
                        availableHeight: availableHeight
                    )
                }
            }
    }

    private func dragHandle(full: CGFloat, available: CGFloat) -> some View {
        VStack(spacing: 0) {
            Capsule(style: .continuous)
                .fill(Color.white.opacity(0.28))
                .frame(width: 38, height: 5)
        }
        .frame(maxWidth: .infinity)
        .frame(minHeight: 44)
        .contentShape(Rectangle())
        .gesture(sheetDragGesture(availableHeight: available))
        .simultaneousGesture(
            TapGesture().onEnded {
                withAnimation(.spring(response: 0.22, dampingFraction: 0.86)) {
                    height = ChatSheetDetents.isCollapsed(height)
                        ? full
                        : ChatSheetDetents.collapsedHeight
                }
            }
        )
        .accessibilityLabel("Coach chat")
        .accessibilityHint("Tap to open or collapse. Drag up to open or drag down to collapse")
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
                withAnimation(.spring(response: 0.22, dampingFraction: 0.86)) {
                    height = ChatSheetDetents.fullHeight(availableHeight: available)
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

    private var expandedContent: some View {
        VStack(spacing: 0) {
            chatList
            composer
        }
    }

    private var chatList: some View {
        ScrollViewReader { proxy in
            ScrollView(.vertical, showsIndicators: true) {
                LazyVStack(spacing: 10) {
                    ForEach(messages) { message in
                        GlassChatRow(
                            message: message,
                            onPlayVoiceMessage: onPlayVoiceMessage
                        )
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
    var onPlayVoiceMessage: (URL) -> Void

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
                if let audioFileURL = message.audioFileURL {
                    Button {
                        onPlayVoiceMessage(audioFileURL)
                    } label: {
                        Label("Voice message", systemImage: "play.circle.fill")
                            .font(.body.weight(.medium))
                            .foregroundStyle(.arTextPrimary)
                    }
                    .buttonStyle(.plain)
                    .contentShape(Rectangle())
                    .accessibilityLabel("Play voice message")
                    .accessibilityHint("Plays the recorded voice check-in")
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
            onSend: {},
            onPlayVoiceMessage: { _ in }
        )
    }
}
