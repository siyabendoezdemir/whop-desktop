# Whop (unofficial desktop wrapper)

A lightweight, native **macOS** desktop wrapper around [whop.com](https://whop.com),
built with [Tauri 2](https://tauri.app).

> **This is an unofficial, personal-use project.** It is **not affiliated with,
> endorsed by, or distributed by Whop.** It simply loads the public website
> `https://whop.com` in a native macOS window. It does not bundle or modify any
> Whop code. The app/site icon is Whop's brandmark (a trademark of Whop — see
> [License & trademarks](#license--trademarks)); you can swap it for your own or
> a neutral placeholder via `scripts/generate-icons.sh`.

---

## What it is

Whop loads `https://whop.com` as a **top-level external URL** inside a native
WKWebView window (not an iframe). The website authenticates and stores its own
session in the normal webview cookie store, so it behaves like a dedicated,
single-site browser:

- Persistent login/cookies/localStorage across launches
- Normal links, redirects, OAuth/email-login popups, and checkout flows stay in the app
- Downloads land in your `~/Downloads`
- Camera/microphone prompts work through the standard macOS permission system
- A native macOS menu with Back/Forward/Reload/Zoom and standard Edit shortcuts

It is intentionally minimal and grants the remote website **zero** access to any
native (Tauri) capability.

---

## Requirements

- macOS 11+ (developed/tested on macOS 26, Apple Silicon)
- [Node.js](https://nodejs.org) 18+ and [pnpm](https://pnpm.io) 9+
- [Rust](https://rustup.rs) (stable) + Cargo
- Xcode Command Line Tools: `xcode-select --install`

If any are missing:

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# pnpm (via corepack, ships with Node)
corepack enable && corepack prepare pnpm@latest --activate

# Xcode Command Line Tools
xcode-select --install
```

---

## Install

```bash
pnpm install
```

## Develop

Runs Vite + a debug build of the app with the Web Inspector enabled:

```bash
pnpm tauri dev
```

## Release build (`.app` + `.dmg`)

```bash
pnpm tauri build
```

### Output locations

After a successful `pnpm tauri build`, the artifacts are written to:

- App: `src-tauri/target/release/bundle/macos/Whop.app`
- DMG: `src-tauri/target/release/bundle/dmg/Whop_0.1.0_aarch64.dmg`

> The `aarch64` suffix reflects Apple Silicon. On an Intel Mac the DMG is named
> `Whop_0.1.0_x64.dmg`.

These are **unsigned local development builds** (no Apple Developer signing or
notarization). The first time you open the `.app` you may need to right-click →
**Open**, or run `xattr -dr com.apple.quarantine /path/to/Whop.app` if macOS
Gatekeeper blocks it after copying it from the DMG.

---

## Replacing the icon

A neutral placeholder icon is generated at build setup. To swap in your own:

1. Prepare a **1024 × 1024 PNG**.
2. Run:

```bash
./scripts/generate-icons.sh /path/to/your-icon-1024.png
```

This calls `pnpm tauri icon`, which regenerates every required size into
`src-tauri/icons/` (`.icns`, `.png`, `.ico`, plus iOS/Android sets). Rebuild with
`pnpm tauri build` afterward.

To regenerate the neutral placeholder instead:

```bash
./scripts/generate-icons.sh        # no argument → recreates the placeholder
```

The placeholder generator (`scripts/make-placeholder-icon.py`) uses only the
Python standard library and never downloads or copies any third-party artwork.

---

## Behavior details

### Camera & microphone

- The bundle declares `NSCameraUsageDescription` and `NSMicrophoneUsageDescription`
  (see `src-tauri/Info.plist`), which macOS shows in the permission prompt.
- Permission is **not** requested on startup. macOS prompts only the first time
  the web page actually calls `navigator.mediaDevices.getUserMedia()`.
- `navigator.mediaDevices.getUserMedia()` is available in the WKWebView. If you
  deny permission, the web page's promise rejects normally and the app keeps
  running — denial does not crash anything.
- `src-tauri/Entitlements.plist` contains the minimum camera / microphone /
  outbound-network entitlements. **These only take effect once the app is
  code-signed**; for the current unsigned builds, access is governed by the
  Info.plist strings plus the macOS TCC prompt.

### Downloads

- Downloads initiated by the website are saved to `~/Downloads`.
- The server-provided filename is preserved when safe; unsafe characters are
  sanitized, and a numeric suffix `(1)`, `(2)`, … is added to avoid overwriting
  an existing file.
- When a download finishes, a **native completion notification** is shown (only
  if you've granted the app notification permission).
- **File → Reveal Last Download in Finder** (`⌘⇧J`) reveals the most recent
  completed download; **File → Open Downloads Folder** opens `~/Downloads`.
- The remote page is **not** granted any general filesystem access.

### Notifications

There are two independent systems:

1. **Whop's own web notifications / Web Push** — these depend on Whop's backend
   and service-worker configuration. macOS WKWebView has historically limited
   support for the Web Push API, and we cannot verify this without a logged-in
   account and Whop's server cooperation. **This wrapper does not fake, inject,
   or work around web push.** If it works, it's because WKWebView + Whop support
   it; if it doesn't, that is a platform limitation (see below).
2. **Native wrapper notifications** — used only for download completion, driven
   from Rust via the official `tauri-plugin-notification`. This works
   independently of (1) and is never exposed to the web page.

### Window / Dock behavior

- Closing the window with the red traffic-light button **hides** it instead of
  quitting (standard macOS behavior), keeping your session alive.
- Clicking the Dock icon restores (or recreates) the window.
- Quit fully with **⌘Q** (App → Quit Whop).

---

## Architecture

### Why a top-level external URL (not an iframe)

The window is created in Rust with `WebviewUrl::External("https://whop.com")`.
Loading the site as the webview's own top-level document (rather than inside an
iframe) is what makes cookies, OAuth popups, `postMessage`, downloads, and
camera/mic permission prompts behave like a real browser. Iframing whop.com
would break third-party-cookie/OAuth flows and would likely be blocked by the
site's framing protections anyway.

### Navigation & popup handling

All handling lives in `src-tauri/src/lib.rs`:

- **`on_navigation`** decides whether each top-level navigation may proceed.
  `https`/`http`/`about`/`blob`/`data` stay in the app (so normal browsing,
  auth, checkout, upload, media, and payment redirects all work). `mailto:`,
  `tel:`, `sms:`, and FaceTime URLs are handed to the macOS default handler and
  the in-webview navigation is canceled. Everything else (e.g. `file://`,
  unknown custom schemes) is blocked.
- **`on_new_window`** handles `window.open` and `target="_blank"`. Web URLs are
  allowed to open as a real popup using WKWebView's default implementation,
  which **preserves the `window.opener` relationship** — this is what lets
  OAuth/email-login/checkout popups send their result back to the main page and
  close themselves. Popup requests are never silently discarded; non-web schemes
  are opened externally or blocked.

### How downloads work

`on_download` intercepts WKWebView download requests. On `Requested` it computes
a safe, de-duplicated destination in `~/Downloads`; on `Finished` it records the
path (for Reveal in Finder) and shows a native notification. No signed download
URL is ever logged.

### Why remote content is denied Tauri IPC (security)

This app loads remote content we don't control, so the threat model assumes the
page could be hostile. Mitigations:

- `capabilities/main-capability.json` has **no `remote` allowlist**, so the
  whop.com origin can never call Tauri IPC, plugins, or native commands.
- `withGlobalTauri` is `false`; no Tauri JS API is injected into the page.
- Native features (downloads, notifications, menu, mailto/tel, reveal-in-Finder)
  run entirely in Rust and are invisible to the web page.
- No TLS validation is disabled, no traffic is proxied/intercepted, no tokens
  are injected, no credentials are stored by the wrapper, no analytics/tracking
  is added, and pages are not modified.
- `csp` applies only to the (essentially empty) local frontend; it has no effect
  on the remote site, whose own server-sent CSP is respected. We do **not** use
  `csp: null`.

### Debug logging

Debug builds print developer logs for navigation, blocked URLs, popup requests,
and download start/finish. Logs deliberately include **only scheme + host** (and
download filenames), never full URLs, cookies, tokens, form contents, or
personal data. Release builds compile the logging out entirely.

---

## Manual test checklist

Some items require your Whop login and can't be automated here:

1. `pnpm tauri dev` (or open the built app) → window opens, `whop.com` loads.
2. Open the login page and sign in.
3. Quit (⌘Q) and reopen → you are still logged in.
4. Click around Whop → navigation stays inside the app.
5. Test Back/Forward/Reload (View menu) and Copy/Paste (Edit menu).
6. Click a `target="_blank"` link → it opens (as a popup) rather than vanishing.
7. Trigger a file download → it lands in `~/Downloads`; a notification appears;
   File → Reveal Last Download in Finder highlights it.
8. Use a Whop feature that needs the camera → macOS prompts; allow/deny both work.
9. Same for the microphone.
10. Deny a permission → the app keeps running (no crash).

---

## Troubleshooting

- **"Whop is damaged / can't be opened" after copying from the DMG:** Gatekeeper
  quarantine on an unsigned app. Run:
  `xattr -dr com.apple.quarantine /Applications/Whop.app` (adjust the path), or
  right-click the app → **Open** the first time.
- **Camera/mic never prompts:** confirm the feature actually calls
  `getUserMedia`, and check **System Settings → Privacy & Security → Camera /
  Microphone**. If you previously denied, re-enable it there.
- **No download notification:** grant notifications in **System Settings →
  Notifications → Whop**. Downloads still complete regardless.
- **Build fails on `pnpm tauri build`:** ensure Xcode CLT is installed
  (`xcode-select -p`) and Rust is up to date (`rustup update`).

### Clearing the app's browsing / session data

This logs you out and wipes cookies/localStorage for the wrapper. WKWebView
stores per-app web data under your user Library; remove the app's data
container and WebKit storage:

```bash
# Quit Whop first, then:
rm -rf ~/Library/WebKit/technologies.ciya.whop
rm -rf ~/Library/Caches/technologies.ciya.whop
rm -rf "~/Library/Containers/technologies.ciya.whop"
rm -rf "~/Library/Application Support/technologies.ciya.whop"
```

> Paths can vary slightly by macOS version. The bundle identifier is
> `technologies.ciya.whop`; searching `~/Library` for that string finds any
> remaining data. Removing it forces a fresh login next launch.

---

## Known WKWebView / authentication limitations

- **Web Push / browser notifications from Whop** may not work: macOS WKWebView's
  support for the Web Push API is limited and depends on Whop's own
  service-worker/back-end setup. This wrapper neither guarantees nor fakes it.
  Native download notifications are unaffected.
- **Hard "force reload" (cache bypass)** isn't exposed by WKWebView; Force Reload
  performs a normal reload.
- **Unsigned build** → Gatekeeper friction when distributing the `.app`/`.dmg`
  (see Troubleshooting). For personal use on your own machine this is usually a
  one-time right-click → Open.
- **Entitlements require signing** to take effect (see Camera & microphone).

---

## Repository layout

```
.                     Tauri 2 desktop app
├── src/              Minimal placeholder frontend (bundler requirement only)
├── src-tauri/        Rust backend, config, capabilities, Info.plist, icons
├── scripts/          Icon generation / replacement
└── landing/          Next.js download page (the marketing/share site)
```

## Landing page

`landing/` is a small Next.js site (a single hero) used to share the build with
others. It serves the `.dmg` from `landing/public/downloads/`.

```bash
cd landing
pnpm install
pnpm dev          # http://localhost:3000
pnpm build        # production build
```

Deploy it anywhere that hosts Next.js (e.g. Vercel). When you ship a new app
version, drop the new `.dmg` into `landing/public/downloads/` and bump the
version constant in `landing/app/page.tsx`.

## Contributing / building from a fresh clone

The raw Whop brand-kit files and the branded icon source are **not** committed
(see `.gitignore`). A fresh clone builds with the generated icons already in
`src-tauri/icons/`. To use your own icon, run `./scripts/generate-icons.sh
/path/to/icon-1024.png`; to produce a neutral placeholder, run it with no
argument.

## License & trademarks

Source code is released under the **MIT License** (see [`LICENSE`](./LICENSE)).

This is an **unofficial, personal-use** project and is **not affiliated with,
endorsed by, or distributed by Whop**. "Whop" and the Whop logo are trademarks
of their respective owner; the MIT license covers this project's own code only
and grants no rights to Whop's name or marks. Don't use this project to imply
any official affiliation with Whop.
