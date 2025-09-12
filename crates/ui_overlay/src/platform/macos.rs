use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use objc2_app_kit::NSApplicationPresentationOptions;

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
