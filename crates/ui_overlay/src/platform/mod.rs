//! Platform-specific helpers for overlay window presentation.
//! Keep OS glue isolated to improve cross-platform maintainability.

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

/// Guard object for temporary presentation changes (opaque across OS).
pub struct PresentationGuard(
    #[cfg(target_os = "macos")] pub(crate) macos::GuardInner,
    #[cfg(target_os = "windows")] pub(crate) windows::GuardInner,
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))] pub(crate) (),
);

/// Apply platform-specific presentation tweaks for full-screen overlay UX.
/// Returns a guard that will restore previous presentation state when passed to `end_presentation`.
pub fn start_presentation() -> Option<PresentationGuard> {
    #[cfg(target_os = "macos")]
    {
        macos::start_presentation().map(PresentationGuard)
    }
    #[cfg(target_os = "windows")]
    {
        windows::start_presentation().map(PresentationGuard)
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        None
    }
}

/// Restore platform presentation using the guard returned by `start_presentation`.
pub fn end_presentation(guard: PresentationGuard) {
    #[cfg(target_os = "macos")]
    {
        macos::end_presentation(guard.0);
    }
    #[cfg(target_os = "windows")]
    {
        windows::end_presentation(guard.0);
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let _ = guard; // no-op
    }
}
