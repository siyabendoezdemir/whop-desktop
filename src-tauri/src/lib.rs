//! Whop — an unofficial, personal-use macOS desktop wrapper around whop.com.
//!
//! Architecture overview (see README.md for the long version):
//!
//! * The main window is created programmatically in Rust and points at
//!   `https://whop.com` as a TOP-LEVEL external URL via
//!   `WebviewUrl::External` — NOT an iframe. This lets WKWebView own cookies,
//!   localStorage, OAuth popups, and downloads exactly like a normal browser.
//! * The remote page is NEVER granted Tauri IPC access (see
//!   `capabilities/main-capability.json`). All native behaviour (downloads,
//!   notifications, menu actions, mailto/tel, reveal-in-Finder) is driven from
//!   this Rust code and is invisible to the web page.
//! * Navigation, popups (`window.open` / `target="_blank"`), downloads, and
//!   non-web schemes are handled by the builder callbacks below.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::menu::{
    AboutMetadataBuilder, Menu, MenuBuilder, MenuEvent, MenuItemBuilder, PredefinedMenuItem,
    SubmenuBuilder,
};
use tauri::utils::config::BackgroundThrottlingPolicy;
use tauri::webview::{DownloadEvent, NewWindowResponse, WebviewWindowBuilder};
use tauri::{AppHandle, Manager, RunEvent, Url, WebviewUrl, WebviewWindow, WindowEvent, Wry};
use tauri_plugin_notification::NotificationExt;

/// The single site this wrapper exists to host.
const WHOP_URL: &str = "https://whop.com";
/// Stable window label used everywhere we look the window up.
const MAIN_LABEL: &str = "main";

/// A fixed 16-byte WKWebView data-store identifier. Using a constant value
/// guarantees the SAME persistent cookie / localStorage / session store is
/// reused on every launch, so logins survive quitting and reopening the app.
/// (This is the persistence guarantee for requirement: "Persists login sessions
/// and cookies between app launches".)
const DATA_STORE_ID: [u8; 16] = [
    0x77, 0x68, 0x6f, 0x70, 0x77, 0x72, 0x61, 0x70, 0x70, 0x65, 0x72, 0x01, 0x00, 0x00, 0x00, 0x01,
];

/// Process-wide state. Kept tiny on purpose.
struct AppState {
    /// Current zoom factor applied to the main webview (menu-driven zoom).
    zoom: Mutex<f64>,
    /// Path of the most recently completed download, for "Reveal in Finder".
    last_download: Mutex<Option<PathBuf>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            zoom: Mutex::new(1.0),
            last_download: Mutex::new(None),
        }
    }
}

/// Debug-only developer log. Compiles to nothing in release builds.
///
/// SECURITY: callers must only pass non-sensitive data. We deliberately log
/// scheme + host only, never full URLs (which can contain signed tokens),
/// cookies, form contents, or personal data.
fn dlog(msg: &str) {
    #[cfg(debug_assertions)]
    eprintln!("[whop] {msg}");
    #[cfg(not(debug_assertions))]
    let _ = msg;
}

/// Returns `scheme://host` for logging, dropping path/query so we never leak
/// secrets embedded in URLs.
fn host_only(url: &Url) -> String {
    match url.host_str() {
        Some(h) => format!("{}://{}", url.scheme(), h),
        None => format!("{}://", url.scheme()),
    }
}

/// Opens a non-web URL (mailto:/tel:/etc.) with the system default handler.
///
/// SECURITY: only ever called for an allowlist of safe schemes (see callers).
/// We do NOT expose Tauri's opener plugin to the web page; this runs in native
/// Rust only.
fn open_with_system(url: &Url) {
    let _ = std::process::Command::new("open").arg(url.as_str()).spawn();
}

/// Reveals a file in Finder (native, not exposed to the web page).
fn reveal_in_finder(path: &Path) {
    let _ = std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .spawn();
}

// ---------------------------------------------------------------------------
// Navigation, popups, and scheme handling
// ---------------------------------------------------------------------------

/// Decides whether a top-level navigation inside the main webview may proceed.
/// Returning `false` cancels the navigation.
fn allow_navigation(url: &Url) -> bool {
    match url.scheme() {
        // Normal web traffic stays inside the app: whop.com, its subdomains,
        // and any auth/checkout/payment/media redirect chain it triggers.
        "https" | "http" | "about" | "blob" | "data" => {
            dlog(&format!("nav allow {}", host_only(url)));
            true
        }
        // Hand these off to the OS default handler, then cancel the in-webview
        // navigation so we don't end up on an error page.
        "mailto" | "tel" | "sms" | "facetime" | "facetime-audio" => {
            dlog(&format!("nav external scheme {}", url.scheme()));
            open_with_system(url);
            false
        }
        // Block everything else (file://, custom app schemes, etc.).
        other => {
            dlog(&format!("nav BLOCKED scheme {other}"));
            false
        }
    }
}

/// Decides how to handle a `window.open` / `target="_blank"` request.
fn handle_new_window(url: &Url) -> NewWindowResponse<Wry> {
    match url.scheme() {
        // Allow the popup with WKWebView's default implementation. This is the
        // robust choice for OAuth / email-login / checkout popups because it
        // preserves the `window.opener` relationship so the popup can
        // postMessage results back to the main page and close itself. We do not
        // silently discard the request.
        "https" | "http" | "about" | "blob" => {
            dlog(&format!("popup allow {}", host_only(url)));
            NewWindowResponse::Allow
        }
        // Non-web schemes: open externally, deny the popup window.
        "mailto" | "tel" | "sms" | "facetime" | "facetime-audio" => {
            dlog(&format!("popup external scheme {}", url.scheme()));
            open_with_system(url);
            NewWindowResponse::Deny
        }
        other => {
            dlog(&format!("popup BLOCKED scheme {other}"));
            NewWindowResponse::Deny
        }
    }
}

// ---------------------------------------------------------------------------
// Downloads
// ---------------------------------------------------------------------------

/// Strips path separators and unsafe characters from a server-provided name.
fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' | ' ' | '(' | ')' | '+' => c,
            _ => '_',
        })
        .collect();
    let trimmed = cleaned.trim_matches(|c| c == '.' || c == ' ').to_string();
    // Cap length (all chars are ASCII at this point, so byte slicing is safe).
    let limited = if trimmed.len() > 150 {
        trimmed[..150].to_string()
    } else {
        trimmed
    };
    if limited.is_empty() {
        "download".to_string()
    } else {
        limited
    }
}

/// Derives a safe filename from a download URL's last path segment.
fn filename_from_url(url: &Url) -> String {
    let raw = url
        .path_segments()
        .and_then(|mut s| s.next_back())
        .filter(|s| !s.is_empty())
        .unwrap_or("download");
    sanitize_filename(raw)
}

/// Returns a path in `dir` that does not collide with an existing file by
/// appending " (n)" before the extension if needed.
fn unique_path(dir: &Path, filename: &str) -> PathBuf {
    let candidate = dir.join(filename);
    if !candidate.exists() {
        return candidate;
    }
    let p = Path::new(filename);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("download");
    let ext = p.extension().and_then(|s| s.to_str());
    for i in 1..10_000 {
        let name = match ext {
            Some(e) => format!("{stem} ({i}).{e}"),
            None => format!("{stem} ({i})"),
        };
        let c = dir.join(name);
        if !c.exists() {
            return c;
        }
    }
    candidate
}

/// Computes where a requested download should be saved: the user's Downloads
/// directory, with a sanitized, de-duplicated filename.
fn download_target(app: &AppHandle, url: &Url) -> PathBuf {
    let dir = app.path().download_dir().unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join("Downloads")
    });
    unique_path(&dir, &filename_from_url(url))
}

// ---------------------------------------------------------------------------
// Window creation
// ---------------------------------------------------------------------------

/// Creates the main window if it does not exist, otherwise shows/focuses it.
/// Used at startup and on Dock-icon reopen.
fn show_or_create_main(app: &AppHandle) -> tauri::Result<()> {
    if let Some(win) = app.get_webview_window(MAIN_LABEL) {
        let _ = win.show();
        let _ = win.set_focus();
        return Ok(());
    }

    let url = WHOP_URL
        .parse::<Url>()
        .expect("WHOP_URL is a valid constant URL");

    WebviewWindowBuilder::new(app, MAIN_LABEL, WebviewUrl::External(url))
        .title("Whop")
        .inner_size(1440.0, 900.0)
        .min_inner_size(1000.0, 700.0)
        .center()
        .resizable(true)
        // Persistent (non-incognito) store keeps cookies + login across launches.
        .incognito(false)
        .data_store_identifier(DATA_STORE_ID)
        // Cmd +/-/0 zoom shortcuts handled natively by the webview.
        .zoom_hotkeys_enabled(true)
        // Keep timers/media alive in the background so calls and any
        // notification logic are not throttled when the window is not focused.
        .background_throttling(BackgroundThrottlingPolicy::Disabled)
        // Web Inspector is only compiled in for debug builds.
        .devtools(cfg!(debug_assertions))
        .on_navigation(allow_navigation)
        .on_new_window(|url, _features| handle_new_window(&url))
        .on_download(|webview, event| {
            let app = webview.app_handle();
            match event {
                DownloadEvent::Requested { url, destination } => {
                    let target = download_target(app, &url);
                    dlog(&format!(
                        "download start {} -> {}",
                        host_only(&url),
                        target
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("download")
                    ));
                    *destination = target;
                    true
                }
                DownloadEvent::Finished { url, path, success } => {
                    let name = path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .and_then(|s| s.to_str())
                        .unwrap_or("download")
                        .to_string();
                    // Log host + filename + success only — never the signed URL.
                    dlog(&format!(
                        "download finished {} file={name} success={success}",
                        host_only(&url)
                    ));
                    if success {
                        if let Some(p) = path.clone() {
                            if let Some(state) = app.try_state::<AppState>() {
                                if let Ok(mut last) = state.last_download.lock() {
                                    *last = Some(p);
                                }
                            }
                        }
                        // Native completion notification (best-effort; only
                        // shows if the user has granted notification permission).
                        let _ = app
                            .notification()
                            .builder()
                            .title("Download complete")
                            .body(format!("Saved \u{201c}{name}\u{201d} to Downloads"))
                            .show();
                    }
                    true
                }
                // DownloadEvent is #[non_exhaustive].
                _ => true,
            }
        })
        .build()?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Menu
// ---------------------------------------------------------------------------

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    // App menu (becomes the application menu on macOS as the first submenu).
    let about_meta = AboutMetadataBuilder::new()
        .name(Some("Whop"))
        .version(Some(env!("CARGO_PKG_VERSION")))
        .comments(Some(
            "Unofficial personal-use desktop wrapper for whop.com. Not affiliated with Whop.",
        ))
        .build();

    let app_menu = SubmenuBuilder::new(app, "Whop")
        .item(&PredefinedMenuItem::about(
            app,
            Some("About Whop"),
            Some(about_meta),
        )?)
        .separator()
        .item(&PredefinedMenuItem::hide(app, Some("Hide Whop"))?)
        .item(&PredefinedMenuItem::hide_others(app, Some("Hide Others"))?)
        .item(&PredefinedMenuItem::show_all(app, None)?)
        .separator()
        .item(&PredefinedMenuItem::quit(app, Some("Quit Whop"))?)
        .build()?;

    // File menu — small extra so completed downloads are reachable.
    let reveal = MenuItemBuilder::with_id("reveal_download", "Reveal Last Download in Finder")
        .accelerator("CmdOrCtrl+Shift+J")
        .build(app)?;
    let open_downloads =
        MenuItemBuilder::with_id("open_downloads", "Open Downloads Folder").build(app)?;
    let file_menu = SubmenuBuilder::new(app, "File")
        .item(&reveal)
        .item(&open_downloads)
        .build()?;

    // Edit menu — standard predefined items (handled natively by the OS).
    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .item(&PredefinedMenuItem::undo(app, None)?)
        .item(&PredefinedMenuItem::redo(app, None)?)
        .separator()
        .item(&PredefinedMenuItem::cut(app, None)?)
        .item(&PredefinedMenuItem::copy(app, None)?)
        .item(&PredefinedMenuItem::paste(app, None)?)
        .item(&PredefinedMenuItem::select_all(app, None)?)
        .build()?;

    // View menu — custom items routed to the current webview (not new windows).
    let back = MenuItemBuilder::with_id("back", "Back")
        .accelerator("CmdOrCtrl+[")
        .build(app)?;
    let forward = MenuItemBuilder::with_id("forward", "Forward")
        .accelerator("CmdOrCtrl+]")
        .build(app)?;
    let reload = MenuItemBuilder::with_id("reload", "Reload")
        .accelerator("CmdOrCtrl+R")
        .build(app)?;
    let force_reload = MenuItemBuilder::with_id("force_reload", "Force Reload")
        .accelerator("CmdOrCtrl+Shift+R")
        .build(app)?;
    let actual_size = MenuItemBuilder::with_id("actual_size", "Actual Size")
        .accelerator("CmdOrCtrl+0")
        .build(app)?;
    let zoom_in = MenuItemBuilder::with_id("zoom_in", "Zoom In")
        .accelerator("CmdOrCtrl+Plus")
        .build(app)?;
    let zoom_out = MenuItemBuilder::with_id("zoom_out", "Zoom Out")
        .accelerator("CmdOrCtrl+-")
        .build(app)?;

    // `mut` is only needed for the debug-only Web Inspector item below.
    #[allow(unused_mut)]
    let mut view_builder = SubmenuBuilder::new(app, "View")
        .item(&back)
        .item(&forward)
        .item(&reload)
        .item(&force_reload)
        .separator()
        .item(&actual_size)
        .item(&zoom_in)
        .item(&zoom_out)
        .separator()
        .item(&PredefinedMenuItem::fullscreen(app, None)?);

    // Web Inspector only exists in debug builds (devtools is disabled in release).
    #[cfg(debug_assertions)]
    {
        let inspector = MenuItemBuilder::with_id("inspector", "Open Web Inspector")
            .accelerator("CmdOrCtrl+Alt+I")
            .build(app)?;
        view_builder = view_builder.separator().item(&inspector);
    }

    let view_menu = view_builder.build()?;

    MenuBuilder::new(app)
        .item(&app_menu)
        .item(&file_menu)
        .item(&edit_menu)
        .item(&view_menu)
        .build()
}

/// Runs a closure with the main webview window if it exists.
fn with_main<F: FnOnce(&WebviewWindow)>(app: &AppHandle, f: F) {
    if let Some(win) = app.get_webview_window(MAIN_LABEL) {
        f(&win);
    }
}

/// Applies an absolute zoom factor and records it in state.
fn set_zoom_abs(app: &AppHandle, value: f64) {
    let clamped = value.clamp(0.3, 3.0);
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut z) = state.zoom.lock() {
            *z = clamped;
        }
    }
    with_main(app, |w| {
        let _ = w.set_zoom(clamped);
    });
}

/// Adjusts zoom by a delta relative to the current factor.
fn adjust_zoom(app: &AppHandle, delta: f64) {
    let current = app
        .try_state::<AppState>()
        .and_then(|s| s.zoom.lock().ok().map(|z| *z))
        .unwrap_or(1.0);
    set_zoom_abs(app, current + delta);
}

fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        "back" => with_main(app, |w| {
            let _ = w.eval("window.history.back()");
        }),
        "forward" => with_main(app, |w| {
            let _ = w.eval("window.history.forward()");
        }),
        "reload" => with_main(app, |w| {
            let _ = w.reload();
        }),
        // WKWebView has no public hard-bypass-cache reload; a normal reload is
        // the safe equivalent here.
        "force_reload" => with_main(app, |w| {
            let _ = w.eval("window.location.reload()");
        }),
        "actual_size" => set_zoom_abs(app, 1.0),
        "zoom_in" => adjust_zoom(app, 0.1),
        "zoom_out" => adjust_zoom(app, -0.1),
        "reveal_download" => {
            if let Some(state) = app.try_state::<AppState>() {
                if let Ok(guard) = state.last_download.lock() {
                    if let Some(p) = guard.as_ref() {
                        reveal_in_finder(p);
                    }
                }
            }
        }
        "open_downloads" => {
            if let Ok(dir) = app.path().download_dir() {
                let _ = std::process::Command::new("open").arg(dir).spawn();
            }
        }
        #[cfg(debug_assertions)]
        "inspector" => with_main(app, |w| w.open_devtools()),
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Native notification plugin — used ONLY by this wrapper for download
        // completion. It is never exposed to the remote whop.com page.
        .plugin(tauri_plugin_notification::init())
        .manage(AppState::default())
        .menu(build_menu)
        .on_menu_event(handle_menu_event)
        .setup(|app| {
            show_or_create_main(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Standard macOS behaviour: closing the window with the red button
            // hides it (keeping the app and the in-memory session alive) instead
            // of quitting. Reopen via the Dock icon (RunEvent::Reopen below).
            if window.label() == MAIN_LABEL {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building the Whop application")
        .run(|app, event| {
            if let RunEvent::Reopen { .. } = event {
                let _ = show_or_create_main(app);
            }
        });
}
