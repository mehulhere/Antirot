# App Download Landing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert the public Antirot website into a downloads-first landing page for the new native iOS and Android app.

**Architecture:** This is a static-site update. `website/index.html` owns the page content and `website/style.css` owns the page visual system. A small Node smoke test checks for the required install CTAs, new app positioning, and the absence of the old focus-dial hero.

**Tech Stack:** Static HTML, CSS, and Node.js ESM.

## Global Constraints

- Do not modify iOS or Android app source files.
- Do not add emojis to code, commands, file contents, or identifiers.
- Preserve existing release asset paths: `releases/antirot.apk` and `releases/Antirot-unsigned.ipa`.
- Keep GitHub, README, simulator, and sign-in links available, but secondary to app downloads.
- Use the app-inspired monochrome palette with red accent.

---

### Task 1: Static Landing Requirements Test

**Files:**
- Create: `scripts/test-website-landing.mjs`

**Interfaces:**
- Consumes: `website/index.html`, `website/style.css`
- Produces: A command `node scripts/test-website-landing.mjs` that exits with code 0 only when the landing page matches the app-download requirements.

- [ ] **Step 1: Write the failing test**

Create `scripts/test-website-landing.mjs` with checks for:

- Hero title contains `Download Antirot`.
- Android APK link points to `releases/antirot.apk`.
- iOS IPA link points to `releases/Antirot-unsigned.ipa`.
- The app mockup class `phone-mockup` exists.
- The install notes section id `install-notes` exists.
- The old `focus-dial-hero` class is absent.
- CSS includes native app palette values `#0A0A0A`, `#141414`, `#1C1C1E`, and `#E63946`.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
node scripts/test-website-landing.mjs
```

Expected: FAIL because the current page still uses the old focus dial and does not contain the new downloads-first hero.

### Task 2: App Download Landing Page

**Files:**
- Modify: `website/index.html`
- Modify: `website/style.css`

**Interfaces:**
- Consumes: The requirements in `scripts/test-website-landing.mjs`.
- Produces: A static landing page with the new app-download content and app-aligned styling.

- [ ] **Step 1: Replace landing content**

Update `website/index.html` so the first viewport leads with app downloads, includes app-focused feature sections, keeps install notes, and moves developer links lower.

- [ ] **Step 2: Replace landing styling**

Update `website/style.css` so the landing page uses the native app monochrome palette, phone mockup, app-stage visual, and responsive layout.

- [ ] **Step 3: Run smoke test**

Run:

```bash
node scripts/test-website-landing.mjs
```

Expected: PASS with all landing requirements satisfied.

- [ ] **Step 4: Run targeted lint**

Run:

```bash
npx eslint scripts/test-website-landing.mjs
```

Expected: PASS.

- [ ] **Step 5: Manually inspect**

Open `website/index.html` in a browser and confirm:

- The first viewport is downloads-first.
- The app mockup is visible on desktop.
- Mobile layout keeps CTA buttons readable.
- GitHub and simulator links remain available as secondary actions.
