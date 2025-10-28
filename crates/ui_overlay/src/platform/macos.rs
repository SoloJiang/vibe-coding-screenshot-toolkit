use objc2::runtime::{AnyObject, Bool};
use objc2::{class, msg_send};
use objc2_app_kit::NSApplicationPresentationOptions;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

pub type GuardInner = NSApplicationPresentationOptions;

/// macOS 窗口层级常量
///
/// 主菜单窗口层级
#[allow(dead_code)]
const NS_MAIN_MENU_WINDOW_LEVEL: i64 = 24;
/// 屏幕保护程序窗口层级（覆盖所有内容包括菜单栏）
const NS_SCREEN_SAVER_WINDOW_LEVEL: i64 = 1000;
/// 工具栏窗口层级（高于主覆盖窗口）- 预留给未来的工具栏使用
#[allow(dead_code)]
const TOOLBAR_WINDOW_LEVEL: i64 = 1100;

/// macOS 窗口集合行为标志
///
/// 窗口可以加入所有空间
const NS_WINDOW_COLLECTION_BEHAVIOR_CAN_JOIN_ALL_SPACES: u64 = 1 << 0;
/// 全屏主窗口
const NS_WINDOW_COLLECTION_BEHAVIOR_FULLSCREEN_PRIMARY: u64 = 1 << 7;
/// 固定窗口（不随空间切换移动）
const NS_WINDOW_COLLECTION_BEHAVIOR_STATIONARY: u64 = 1 << 4;

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

        // 设置窗口层级（使用屏幕保护程序级别，覆盖菜单栏）
        let _: () = msg_send![ns_window, setLevel: NS_SCREEN_SAVER_WINDOW_LEVEL];

        // 设置窗口集合行为：可加入所有空间 + 全屏主窗口 + 固定
        let behavior = NS_WINDOW_COLLECTION_BEHAVIOR_CAN_JOIN_ALL_SPACES
            | NS_WINDOW_COLLECTION_BEHAVIOR_FULLSCREEN_PRIMARY
            | NS_WINDOW_COLLECTION_BEHAVIOR_STATIONARY;
        let _: () = msg_send![ns_window, setCollectionBehavior: behavior];

        // 确保窗口接受鼠标事件（不忽略鼠标事件）
        let _: () = msg_send![ns_window, setIgnoresMouseEvents: Bool::from(false)];

        #[cfg(debug_assertions)]
        {
            let current_level: i64 = msg_send![ns_window, level];
            let current_behavior: u64 = msg_send![ns_window, collectionBehavior];
            tracing::debug!(
                "[macOS Window] 窗口层级 level={}, 集合行为 behavior={:#b}",
                current_level,
                current_behavior
            );
            if current_level < NS_MAIN_MENU_WINDOW_LEVEL {
                tracing::warn!(
                    "[macOS Window] ⚠️  窗口层级 {} 低于菜单栏层级 {}，菜单栏可能显示!",
                    current_level,
                    NS_MAIN_MENU_WINDOW_LEVEL
                );
            }
        }
    }
}
