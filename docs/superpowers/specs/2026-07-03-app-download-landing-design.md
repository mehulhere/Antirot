# App Download Landing Page Design

## Goal

Update the public Antirot website so the first impression is the new native iOS and Android app, with downloads as the primary conversion path.

## Context

The current website still uses the older violet, gold, and cyan marketing system and presents GitHub and the simulator almost as strongly as app installation. The current native app has moved to a monochrome coach-room experience: near-black backgrounds, muted gray text, a single red accent, one dominant action button, a cinematic abstract coach stage, and a bottom chat sheet.

## Audience

The primary audience is someone deciding whether to install Antirot on their phone. Developers remain a secondary audience and should still find GitHub, README, and simulator links lower on the page.

## Experience

The hero should lead with downloading Antirot for iOS and Android. It should make the app the primary visual object by showing an app-style phone mockup inspired by the iOS coach room: status pill, abstract coach, one red circular action, and a bottom chat sheet.

The page should explain the app in phone-native terms:

- Voice-first coach interaction.
- One state-aware primary action.
- Real alarms and escalation.
- Current-task widget.
- Google sign-in and managed backend sync.
- Strict but useful behavioral memory.

Install notes should be honest:

- Android uses the direct APK.
- iOS uses the unsigned IPA through SideStore or AltStore.
- Some iOS capabilities depend on entitlements, signed builds, or iOS version.

## Visual Direction

Use the native app palette:

- Background: `#0A0A0A`
- Surface: `#141414`
- Elevated: `#1C1C1E`
- Accent: `#E63946`
- Secondary text: `#8E8E93`
- Muted text: `#48484A`

Reduce decorative gradients and remove the old focus dial from the landing page. Keep motion subtle and app-like.

## Scope

Modify:

- `website/index.html`
- `website/style.css`

Add:

- `scripts/test-website-landing.mjs`

Do not modify the iOS or Android app code for this task.

## Validation

Run:

```bash
node scripts/test-website-landing.mjs
npx eslint scripts/test-website-landing.mjs
```

Manual verification:

- Open `website/index.html` in a browser.
- Confirm the first viewport is downloads-first.
- Confirm Android and iOS download buttons point to the existing release files.
- Confirm GitHub and simulator links are still present but secondary.
