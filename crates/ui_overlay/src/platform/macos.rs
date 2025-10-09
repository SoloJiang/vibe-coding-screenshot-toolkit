use objc2::runtime::{AnyObject, Bool};
use objc2::{class, msg_send};
use objc2_app_kit::NSApplicationPresentationOptions;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

pub type GuardInner = NSApplicationPresentationOptions;

pub fn start_presentation() -> Option<GuardInner> {
    // NSApplication *app = [NSApplication sharedApplication]; activate and hide menu/dock
    let app: *mut AnyObject = unsafe { msg_send![class!(NSApplication), sharedApplication] };
    unsafe {
        let _: () = msg_send![app, activateIgnoringOtherApps: true];
    }
    let prev: NSApplicationPresentationOptions = unsafe { msg_send![app, presentationOptions] };
    let opts = NSApplicationPresentationOptions::HideMenuBar
        | NSApplicationPresentationOptions::HideDock
        | NSApplicationPresentationOptions::DisableAppleMenu
        | NSApplicationPresentationOptions::DisableProcessSwitching
        | NSApplicationPresentationOptions::DisableForceQuit
        | NSApplicationPresentationOptions::DisableSessionTermination
        | NSApplicationPresentationOptions::DisableHideApplication;
    unsafe {
        let _: () = msg_send![app, setPresentationOptions: opts];
    }
    Some(prev)
}

pub fn end_presentation(guard: GuardInner) {
    let app: *mut AnyObject = unsafe { msg_send![class!(NSApplication), sharedApplication] };
    unsafe {
        let _: () = msg_send![app, setPresentationOptions: guard];
    }
}

pub fn apply_overlay_window_appearance(window: &Window, color: [u8; 4]) {
    let handle = match window.window_handle() {
        Ok(handle) => handle,
        Err(_) => return,
    };

    let ns_view_ptr = match handle.as_raw() {
        RawWindowHandle::AppKit(appkit_handle) => appkit_handle.ns_view.as_ptr(),
        _ => std::ptr::null_mut(),
    };

    if ns_view_ptr.is_null() {
        return;
    }

    let ns_view = ns_view_ptr.cast::<AnyObject>();

    let ns_window: *mut AnyObject = unsafe { msg_send![ns_view, window] };
    if ns_window.is_null() {
        return;
    }

    let r = color[0] as f64 / 255.0;
    let g = color[1] as f64 / 255.0;
    let b = color[2] as f64 / 255.0;
    let a = color[3] as f64 / 255.0;

    unsafe {
        let color_obj: *mut AnyObject =
            msg_send![class!(NSColor), colorWithSRGBRed: r, green: g, blue: b, alpha: a];
        let _: () = msg_send![ns_window, setOpaque: Bool::from(false)];
        let _: () = msg_send![ns_window, setHasShadow: Bool::from(false)];
        let _: () = msg_send![ns_window, setBackgroundColor: color_obj];
        let _: () = msg_send![ns_window, setAlphaValue: 1.0];
    }
}
