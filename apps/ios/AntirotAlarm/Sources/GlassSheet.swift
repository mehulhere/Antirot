import SwiftUI
import UIKit

enum ChatSheetDetents {
    static let collapsedHeight: CGFloat = 72
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

    static func finalHeight(
        from start: CGFloat,
        translationY: CGFloat,
        velocityY: CGFloat,
        availableHeight: CGFloat
    ) -> CGFloat {
        if velocityY < -80 || translationY < -12 {
            return fullHeight(availableHeight: availableHeight)
        }
        if velocityY > 80 || translationY > 12 {
            return collapsedHeight
        }
        return nearestHeight(to: start - translationY, availableHeight: availableHeight)
    }

    static func isCollapsed(_ height: CGFloat) -> Bool {
        height <= collapsedHeight + 14
    }

    static func showsCollapsedContent(
        committedHeight: CGFloat,
        isDragging: Bool,
        dragBeganCollapsed: Bool
    ) -> Bool {
        if isDragging {
            return dragBeganCollapsed
        }
        return isCollapsed(committedHeight)
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
    var bottomInset: CGFloat = 0

    var onMic: () -> Void
    var onSend: () -> Void
    var onPlayVoiceMessage: (URL) -> Void

    @State private var isHandleDragging = false
    @State private var handleDragBeganCollapsed = false
    @FocusState private var isDraftFocused: Bool

    var body: some View {
        GeometryReader { proxy in
            let available = max(1, proxy.size.height - bottomInset)
            let full = ChatSheetDetents.fullHeight(availableHeight: available)
            let resolved = min(max(height, ChatSheetDetents.collapsedHeight), full)
            let showCollapsedContent = ChatSheetDetents.showsCollapsedContent(
                committedHeight: resolved,
                isDragging: isHandleDragging,
                dragBeganCollapsed: handleDragBeganCollapsed
            )

            VStack(spacing: 0) {
                Spacer()
                sheetContent(
                    full: full,
                    showCollapsedContent: showCollapsedContent
                )
                    .frame(maxWidth: .infinity, alignment: .top)
                    .frame(height: resolved, alignment: .top)
                    .padding(.horizontal, 16)
                    .padding(.bottom, bottomInset + 10)
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
                available: full / ChatSheetDetents.fullFraction,
                isCompact: showCollapsedContent
            )

            if showCollapsedContent {
                collapsedContent(available: full / ChatSheetDetents.fullFraction)
            } else {
                expandedContent
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .contentShape(RoundedRectangle(cornerRadius: showCollapsedContent ? 22 : 28, style: .continuous))
        .background(
            RoundedRectangle(cornerRadius: showCollapsedContent ? 22 : 28, style: .continuous)
                .fill(Color(red: 0.070, green: 0.078, blue: 0.090).opacity(0.94))
        )
        .overlay(
            RoundedRectangle(cornerRadius: showCollapsedContent ? 22 : 28, style: .continuous)
                .stroke(Color.white.opacity(0.07), lineWidth: 0.7)
        )
        .clipShape(RoundedRectangle(cornerRadius: showCollapsedContent ? 22 : 28, style: .continuous))
        .shadow(color: .black.opacity(0.42), radius: 18, y: -6)
    }

    private func dragHandle(full: CGFloat, available: CGFloat, isCompact: Bool) -> some View {
        let handleHeight: CGFloat = isCompact ? 0 : 44

        return ZStack {
            if !isCompact {
                Capsule(style: .continuous)
                    .fill(Color.white.opacity(0.28))
                    .frame(width: 38, height: 5)
                    .allowsHitTesting(false)
            }

            ChatSheetHandleInput(
                currentHeight: $height,
                availableHeight: available,
                fullHeight: full,
                onBegan: {
                    isHandleDragging = true
                    handleDragBeganCollapsed = ChatSheetDetents.isCollapsed(height)
                },
                onChanged: { nextHeight in
                    var transaction = Transaction()
                    transaction.disablesAnimations = true
                    transaction.animation = nil
                    withTransaction(transaction) {
                        height = nextHeight
                    }
                },
                onEnded: { nextHeight in
                    isHandleDragging = false
                    withAnimation(.spring(response: 0.22, dampingFraction: 0.86)) {
                        height = nextHeight
                    }
                },
                onCancelled: {
                    isHandleDragging = false
                }
            )
            .frame(maxWidth: .infinity)
            .frame(height: handleHeight)
        }
        .frame(maxWidth: .infinity)
        .frame(height: handleHeight)
        .accessibilityLabel("Coach chat")
        .accessibilityHint("Tap to open or collapse. Drag up to open or drag down to collapse")
    }
    // MARK: - Collapsed

    private func collapsedContent(available: CGFloat) -> some View {
        HStack(spacing: 12) {
            micButton(size: 46)

            TextField("Say it or type a command...", text: $draft, axis: .vertical)
                .focused($isDraftFocused)
                .lineLimit(1...2)
                .textInputAutocapitalization(.sentences)
                .font(.subheadline.weight(.medium))
                .foregroundStyle(.arTextPrimary)
                .submitLabel(.send)
                .onSubmit(onSend)
                .padding(.horizontal, 2)

            Button(action: onSend) {
                Image(systemName: "arrow.up")
                    .font(.subheadline.weight(.bold))
                    .foregroundStyle(.arTextSecondary)
                    .frame(width: 38, height: 38)
                    .background(Circle().fill(Color.white.opacity(0.06)))
            }
            .buttonStyle(.plain)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(Rectangle())
        .onTapGesture {
            isDraftFocused = true
        }
    }

    // MARK: - Expanded

    private var expandedContent: some View {
        VStack(spacing: 0) {
            expandedHeader
            chatList
            composer
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
    }

    private var expandedHeader: some View {
        HStack(spacing: 10) {
            Circle()
                .fill(Color.white.opacity(0.36))
                .frame(width: 7, height: 7)

            Text(statusText)
                .font(.system(size: 13, weight: .semibold))
                .foregroundStyle(.arTextSecondary)
                .lineLimit(1)

            Spacer(minLength: 12)
        }
        .padding(.horizontal, 20)
        .padding(.bottom, 6)
        .contentShape(Rectangle())
        .onTapGesture(perform: dismissDraftKeyboard)
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
                .padding(.top, 6)
                .padding(.bottom, 16)
                .frame(maxWidth: .infinity, alignment: .top)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
            .contentShape(Rectangle())
            .onTapGesture(perform: dismissDraftKeyboard)
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

    private func dismissDraftKeyboard() {
        isDraftFocused = false
    }
}

// MARK: - UIKit Handle Input

private struct ChatSheetHandleInput: UIViewRepresentable {
    @Binding var currentHeight: CGFloat

    var availableHeight: CGFloat
    var fullHeight: CGFloat
    var onBegan: () -> Void
    var onChanged: (CGFloat) -> Void
    var onEnded: (CGFloat) -> Void
    var onCancelled: () -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }

    func makeUIView(context: Context) -> UIView {
        let view = UIView(frame: .zero)
        view.backgroundColor = .clear
        view.isMultipleTouchEnabled = false

        let pan = UIPanGestureRecognizer(
            target: context.coordinator,
            action: #selector(Coordinator.handlePan(_:))
        )
        pan.maximumNumberOfTouches = 1
        pan.cancelsTouchesInView = true
        pan.delegate = context.coordinator

        let tap = UITapGestureRecognizer(
            target: context.coordinator,
            action: #selector(Coordinator.handleTap(_:))
        )
        tap.cancelsTouchesInView = true
        tap.delegate = context.coordinator
        tap.require(toFail: pan)

        view.addGestureRecognizer(pan)
        view.addGestureRecognizer(tap)
        return view
    }

    func updateUIView(_ uiView: UIView, context: Context) {
        context.coordinator.parent = self
    }

    final class Coordinator: NSObject, UIGestureRecognizerDelegate {
        var parent: ChatSheetHandleInput
        private var startHeight: CGFloat = ChatSheetDetents.collapsedHeight

        init(_ parent: ChatSheetHandleInput) {
            self.parent = parent
        }

        @objc func handleTap(_ recognizer: UITapGestureRecognizer) {
            guard recognizer.state == .ended else { return }
            let nextHeight = ChatSheetDetents.isCollapsed(parent.currentHeight)
                ? parent.fullHeight
                : ChatSheetDetents.collapsedHeight
            parent.onEnded(nextHeight)
        }

        @objc func handlePan(_ recognizer: UIPanGestureRecognizer) {
            switch recognizer.state {
            case .began:
                startHeight = parent.currentHeight
                parent.onBegan()
            case .changed:
                let translation = recognizer.translation(in: recognizer.view)
                let nextHeight = ChatSheetDetents.liveHeight(
                    from: startHeight,
                    translationY: translation.y,
                    availableHeight: parent.availableHeight
                )
                parent.onChanged(nextHeight)
            case .ended:
                let translation = recognizer.translation(in: recognizer.view)
                let velocity = recognizer.velocity(in: recognizer.view)
                let nextHeight = ChatSheetDetents.finalHeight(
                    from: startHeight,
                    translationY: translation.y,
                    velocityY: velocity.y,
                    availableHeight: parent.availableHeight
                )
                parent.onEnded(nextHeight)
            case .cancelled, .failed:
                parent.onCancelled()
            default:
                break
            }
        }

        func gestureRecognizer(
            _ gestureRecognizer: UIGestureRecognizer,
            shouldRecognizeSimultaneouslyWith otherGestureRecognizer: UIGestureRecognizer
        ) -> Bool {
            false
        }
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
                            .font(.system(size: 15, weight: .medium))
                            .foregroundStyle(.arTextPrimary)
                    }
                    .buttonStyle(.plain)
                    .contentShape(Rectangle())
                    .accessibilityLabel("Play voice message")
                    .accessibilityHint("Plays the recorded voice check-in")
                } else {
                    Text(message.text)
                        .font(.system(size: 16, weight: .regular))
                        .foregroundStyle(.arTextPrimary)
                        .lineSpacing(2)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            .padding(.horizontal, 13)
            .padding(.vertical, 9)
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
            height: .constant(ChatSheetDetents.collapsedHeight),
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
