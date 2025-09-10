// Windows-specific presentation helper (placeholder).
// Future: set window always-on-top/z-order, disable taskbar flashing, etc.

#[derive(Clone, Copy, Debug, Default)]
pub struct GuardInner;

pub fn start_presentation() -> Option<GuardInner> {
    // Currently no special presentation handling required on Windows for the overlay.
    None
}

pub fn end_presentation(_guard: GuardInner) {
    // no-op
}
