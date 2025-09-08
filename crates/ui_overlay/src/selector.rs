use crate::{OverlayError, Region, RegionSelector, Result as OverlayResult};
#[cfg(target_os = "macos")]
use cocoa::{
    appkit::{NSApp, NSApplication, NSApplicationPresentationOptions, NSWindow, NSWindowStyleMask},
    base::{id, NO, YES},
    foundation::{NSRect, NSSize},
};
#[cfg(target_os = "macos")]
use objc::{msg_send, sel, sel_impl};
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, Event, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
#[cfg(target_os = "macos")]
use winit::platform::macos::WindowExtMacOS;
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::window::WindowBuilder;

pub struct WinitRegionSelector;

impl WinitRegionSelector {
    pub fn new() -> Self {
        Self
    }
}

impl RegionSelector for WinitRegionSelector {
    fn select(&self) -> OverlayResult<Region> {
        // 走无背景版本（不建议用），保持兼容
        match self.run_selector(None, 0, 0)? {
            Some(r) => Ok(r),
            None => Err(OverlayError::Cancelled),
        }
    }

    fn select_with_background(&self, rgb: &[u8], width: u32, height: u32) -> crate::MaybeRegion {
        self.run_selector(Some(rgb), width, height)
    }
}

impl WinitRegionSelector {
    fn run_selector(&self, bg_rgb: Option<&[u8]>, bg_w: u32, bg_h: u32) -> crate::MaybeRegion {
        // 创建全屏窗口（无边框/无阴影），使用 Pixels 渲染
        let mut event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_decorations(false)
            .with_resizable(false)
            // 不依赖窗口透明，直接绘制背景截图，避免纯黑问题
            .with_transparent(false)
            .build(&event_loop)
            .map_err(|e| OverlayError::Internal(format!("window build: {e}")))?;

        #[cfg(target_os = "macos")]
        {
            // 去掉阴影，避免看起来像是有窗口边框
            window.set_has_shadow(false);
            // 强制隐藏系统菜单栏与 Dock，并激活到前台，确保覆盖
            unsafe {
                let app = NSApp();
                let opts = NSApplicationPresentationOptions::NSApplicationPresentationHideDock
                    | NSApplicationPresentationOptions::NSApplicationPresentationHideMenuBar;
                app.setPresentationOptions_(opts);
                app.activateIgnoringOtherApps_(true);

                let ns_win: id = window.ns_window() as id;
                // 强制无边框样式并禁止拖动
                let _: () =
                    msg_send![ns_win, setStyleMask: NSWindowStyleMask::NSBorderlessWindowMask];
                let _: () = msg_send![ns_win, setMovable: NO];
                let _: () = msg_send![ns_win, setMovableByWindowBackground: NO];

                // 将窗口 frame 设置为当前屏幕完整区域（非 visibleFrame），确保覆盖菜单栏高度
                let screen: id = msg_send![ns_win, screen];
                if !screen.is_null() {
                    let frame: NSRect = msg_send![screen, frame];
                    let _: () = msg_send![ns_win, setFrame: frame display: YES];
                    // 内容尺寸与 frame 对齐
                    let _: () = msg_send![ns_win, setContentSize: NSSize { width: frame.size.width, height: frame.size.height }];
                }
                // 加入所有 Spaces，并作为全屏辅助窗口，避免空间切换与菜单栏区域缺口
                // CanJoinAllSpaces = 1<<0, FullScreenAuxiliary = 1<<8
                let behavior: u64 = (1 << 0) | (1 << 8);
                let _: () = msg_send![ns_win, setCollectionBehavior: behavior];
                // 提升到屏保级别，位于菜单栏/ Dock 之上
                ns_win.setLevel_(1000);
            }
            window.set_visible(true);
            window.request_redraw();
        }

        let mut size_px: PhysicalSize<u32> = window.inner_size();
        let surface_texture = SurfaceTexture::new(size_px.width, size_px.height, &window);
        let mut pixels = Pixels::new(size_px.width, size_px.height, surface_texture)
            .map_err(|e| OverlayError::Internal(format!("pixels: {e}")))?;

        #[cfg(target_os = "macos")]
        unsafe {
            // 进一步确保窗口内容尺寸与屏幕匹配
            let ns_win: id = window.ns_window() as id;
            let _: () = msg_send![ns_win, setContentSize: cocoa::foundation::NSSize{ width: size_px.width as f64, height: size_px.height as f64 }];
        }

        // 预处理背景（RGB -> RGBA），若提供
        let bg_rgba: Option<Vec<u8>> = bg_rgb.map(|rgb| {
            let mut out = vec![0u8; (bg_w as usize) * (bg_h as usize) * 4];
            for y in 0..bg_h as usize {
                for x in 0..bg_w as usize {
                    let si = (y * (bg_w as usize) + x) * 3;
                    let di = (y * (bg_w as usize) + x) * 4;
                    out[di] = rgb[si];
                    out[di + 1] = rgb[si + 1];
                    out[di + 2] = rgb[si + 2];
                    out[di + 3] = 255;
                }
            }
            out
        });

        // 交互状态
        let mut dragging = false;
        let mut start: (f32, f32) = (0.0, 0.0);
        let mut curr: (f32, f32) = (0.0, 0.0);
        let mut result: Option<Region> = None;

        event_loop.run_return(|event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        result = None;
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                state: ElementState::Pressed,
                                ..
                            },
                        ..
                    } => {
                        result = None;
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: MouseButton::Left,
                        ..
                    } => {
                        dragging = true;
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Released,
                        button: MouseButton::Left,
                        ..
                    } => {
                        if dragging {
                            dragging = false;
                        }
                        // 规范化并返回（逻辑坐标）
                        let (mut x, mut y) = start;
                        let mut w = curr.0 - start.0;
                        let mut h = curr.1 - start.1;
                        if w < 0.0 {
                            x += w;
                            w = -w;
                        }
                        if h < 0.0 {
                            y += h;
                            h = -h;
                        }
                        if w >= 1.0 && h >= 1.0 {
                            result = Some(Region::new(x, y, w, h, window.scale_factor() as f32));
                        }
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        // 跟踪逻辑坐标，绘制时再乘以 scale 转为物理像素
                        let p = position.to_logical::<f64>(window.scale_factor());
                        let (x, y) = (p.x as f32, p.y as f32);
                        if dragging {
                            curr = (x, y);
                        } else {
                            start = (x, y);
                            curr = (x, y);
                        }
                        window.request_redraw();
                    }
                    WindowEvent::Resized(new_size) => {
                        size_px = new_size;
                        pixels.resize_surface(new_size.width, new_size.height).ok();
                        pixels.resize_buffer(new_size.width, new_size.height).ok();
                        window.request_redraw();
                    }
                    _ => {}
                },
                Event::RedrawRequested(_wid) => {
                    // 绘制：先背景（若有），再对非选区做暗化，最后描边
                    let frame = pixels.frame_mut();
                    let w = size_px.width as usize;
                    let h = size_px.height as usize;

                    // 1) 背景
                    if let Some(bg) = &bg_rgba {
                        // 假设尺寸一致；如不一致则按交集绘制
                        let rw = w.min(bg_w as usize);
                        let rh = h.min(bg_h as usize);
                        for y in 0..rh {
                            let dst_row = y * w * 4;
                            let src_row = y * (bg_w as usize) * 4;
                            frame[dst_row..dst_row + rw * 4]
                                .copy_from_slice(&bg[src_row..src_row + rw * 4]);
                        }
                        // 超出区域用黑色填充（理论不应发生）
                        for y in rh..h {
                            for x in 0..w {
                                let idx = (y * w + x) * 4;
                                frame[idx..idx + 4].copy_from_slice(&[0, 0, 0, 255]);
                            }
                        }
                    } else {
                        // 无背景：填充全黑
                        for y in 0..h {
                            for x in 0..w {
                                let idx = (y * w + x) * 4;
                                frame[idx..idx + 4].copy_from_slice(&[0, 0, 0, 255]);
                            }
                        }
                    }

                    // 2) 暗化非选区
                    let scale = window.scale_factor() as f32;
                    let (x0l, y0l) = (start.0.min(curr.0), start.1.min(curr.1));
                    let (x1l, y1l) = (start.0.max(curr.0), start.1.max(curr.1));
                    let x0 = (x0l * scale).floor().max(0.0) as usize;
                    let y0 = (y0l * scale).floor().max(0.0) as usize;
                    let x1 = (x1l * scale).ceil().min(size_px.width as f32) as usize;
                    let y1 = (y1l * scale).ceil().min(size_px.height as f32) as usize;

                    let dim_alpha = 90u8; // 暗化强度
                    for y in 0..h {
                        for x in 0..w {
                            let inside = x >= x0 && x < x1 && y >= y0 && y < y1;
                            if inside {
                                continue; // 选区不暗化
                            }
                            let idx = (y * w + x) * 4;
                            // 简单线性插值：dst = src * (1 - a)
                            // a = dim_alpha / 255
                            let r = frame[idx] as u16;
                            let g = frame[idx + 1] as u16;
                            let b = frame[idx + 2] as u16;
                            let a = dim_alpha as u16;
                            let nr = ((r * (255 - a)) / 255) as u8;
                            let ng = ((g * (255 - a)) / 255) as u8;
                            let nb = ((b * (255 - a)) / 255) as u8;
                            frame[idx] = nr;
                            frame[idx + 1] = ng;
                            frame[idx + 2] = nb;
                            frame[idx + 3] = 255;
                        }
                    }

                    // 3) 白色描边
                    if x1 > x0 && y1 > y0 {
                        for x in x0..x1 {
                            let idx_top = (y0 * w + x) * 4;
                            let idx_bottom = ((y1 - 1) * w + x) * 4;
                            frame[idx_top..idx_top + 4].copy_from_slice(&[255, 255, 255, 255]);
                            frame[idx_bottom..idx_bottom + 4]
                                .copy_from_slice(&[255, 255, 255, 255]);
                        }
                        for y in y0..y1 {
                            let idx_left = (y * w + x0) * 4;
                            let idx_right = (y * w + x1 - 1) * 4;
                            frame[idx_left..idx_left + 4].copy_from_slice(&[255, 255, 255, 255]);
                            frame[idx_right..idx_right + 4].copy_from_slice(&[255, 255, 255, 255]);
                        }
                    }

                    if pixels.render().is_err() {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                _ => {}
            }
        });

        Ok(result)
    }
}
