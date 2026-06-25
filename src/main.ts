// Intentionally minimal. This local frontend is only a placeholder so that the
// Tauri bundler has a `frontendDist` to package. The main application window
// is created in Rust and points directly at https://whop.com, so this code does
// not run in the user-facing window during normal use.
//
// IMPORTANT (security): we deliberately do NOT import or expose any Tauri APIs
// here, and the remote whop.com page is never granted access to Tauri IPC.
export {};
