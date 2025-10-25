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
        // 基础外观设置
        let color_obj: *mut AnyObject =
            msg_send![class!(NSColor), colorWithSRGBRed: r, green: g, blue: b, alpha: a];
        let _: () = msg_send![ns_window, setOpaque: Bool::from(false)];
        let _: () = msg_send![ns_window, setHasShadow: Bool::from(false)];
        let _: () = msg_send![ns_window, setBackgroundColor: color_obj];
        let _: () = msg_send![ns_window, setAlphaValue: 1.0];

        // 设置窗口层级（高于菜单栏）
        // NSMainMenuWindowLevel = 24
        // NSScreenSaverWindowLevel = 1000
        // 使用 ScreenSaver 层级确保完全覆盖菜单栏
        let level: i64 = 1000;
        let _: () = msg_send![ns_window, setLevel: level];

        // 设置窗口集合行为
        // NSWindowCollectionBehaviorCanJoinAllSpaces (1 << 0) = 1
        // NSWindowCollectionBehaviorFullScreenPrimary (1 << 7) = 128
        // NSWindowCollectionBehaviorStationary (1 << 4) = 16
        let behavior: u64 = (1 << 0) | (1 << 7) | (1 << 4);
        let _: () = msg_send![ns_window, setCollectionBehavior: behavior];

        // 确保窗口接受鼠标事件（不忽略鼠标事件）
        let _: () = msg_send![ns_window, setIgnoresMouseEvents: Bool::from(false)];

        #[cfg(debug_assertions)]
        {
            let current_level: i64 = msg_send![ns_window, level];
            let current_behavior: u64 = msg_send![ns_window, collectionBehavior];
            tracing::debug!(
                "[macOS Window] 窗口层级 level={}, 集合行为 collectionBehavior={:#b} ({})",
                current_level,
                current_behavior,
                current_behavior
            );
            tracing::debug!(
                "[macOS Window] 层级说明: NSMainMenuWindowLevel=24, NSScreenSaverWindowLevel=1000"
            );
            if current_level < 24 {
                tracing::warn!(
                    "[macOS Window] ⚠️  窗口层级 {} 低于菜单栏层级 24，菜单栏可能显示!",
                    current_level
                );
            }
        }
    }
}
