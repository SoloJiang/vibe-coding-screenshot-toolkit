use crate::platform;
use crate::{OverlayError, Region, RegionSelector, Result as OverlayResult};
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize, Position};
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowLevel};

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

        struct SelectionApp {
            attrs: WindowAttributes,
            // 复用单个 Pixels，避免每帧创建导致“Context leak detected”日志
            // 注意 drop 顺序：字段按声明顺序 drop，需先 drop pixels 再 drop window
            pixels: Option<Pixels<'static>>,
            // 使用 Box<Window> 确保稳定地址；pixels 中保存对 Window 的引用
            window: Option<Box<Window>>,
            pres_guard: Option<platform::PresentationGuard>,
            size_px: PhysicalSize<u32>,
            bg: Option<Vec<u8>>,
            bg_w: u32,
            bg_h: u32,
            // 预计算后的变暗背景（与 bg 同尺寸）。若无 bg 则为 None
            bg_dim: Option<Vec<u8>>,
            dragging: bool,
            start: (f32, f32),
            curr: (f32, f32),
            result: Option<Region>,
            shift_down: bool,
            alt_down: bool,
            // 合并高频重绘请求，避免在同一帧内重复 request_redraw
            redraw_pending: bool,
        }

        impl SelectionApp {
            fn rect_logical(&self) -> (f32, f32, f32, f32) {
                let (sx, sy) = self.start;
                let (cx, cy) = self.curr;
                let dx = cx - sx;
                let dy = cy - sy;
                if self.alt_down && self.shift_down {
                    let side = dx.abs().max(dy.abs());
                    return (sx - side, sy - side, sx + side, sy + side);
                }
                if self.alt_down {
                    return (sx - dx.abs(), sy - dy.abs(), sx + dx.abs(), sy + dy.abs());
                }
                if self.shift_down {
                    let side = dx.abs().max(dy.abs());
                    let sxn = if dx >= 0.0 { sx } else { sx - side };
                    let syn = if dy >= 0.0 { sy } else { sy - side };
                    let exn = if dx >= 0.0 { sx + side } else { sx };
                    let eyn = if dy >= 0.0 { sy + side } else { sy };
                    return (sxn, syn, exn, eyn);
                }
                (sx.min(cx), sy.min(cy), sx.max(cx), sy.max(cy))
            }

            fn ensure_pixels(&mut self) -> bool {
                if self.pixels.is_some() {
                    return true;
                }
                let Some(win) = self.window.as_ref() else {
                    return false;
                };
                let size_px = self.size_px;
                if size_px.width == 0 || size_px.height == 0 {
                    return false;
                }
                // 安全性说明：
                // - 我们用 Box<Window> 持有窗口，获取其裸指针并临时转为 &'static Window 仅用于构建 Pixels。
                // - 本结构体中字段声明顺序保证 drop 顺序：先 drop pixels 再 drop window，引用始终有效。
                let window_ref: &'static Window = unsafe { &*(win.as_ref() as *const Window) };
                let surface = SurfaceTexture::new(size_px.width, size_px.height, window_ref);
                match Pixels::new(size_px.width, size_px.height, surface) {
                    Ok(p) => {
                        self.pixels = Some(p);
                        true
                    }
                    Err(_) => false,
                }
            }

            fn render_once(&mut self, event_loop: &ActiveEventLoop) {
                if !self.ensure_pixels() {
                    return;
                }
                let size_px = self.size_px;
                // 先计算缩放与矩形，避免持有对 pixels 的可变借用期间再次借用 self
                let scale = self
                    .window
                    .as_ref()
                    .map(|w| w.scale_factor() as f32)
                    .unwrap_or(1.0);
                let (x0l, y0l, x1l, y1l) = self.rect_logical();
                let x0 = (x0l * scale).floor().max(0.0) as usize;
                let y0 = (y0l * scale).floor().max(0.0) as usize;
                let x1 = (x1l * scale).ceil().min(size_px.width as f32) as usize;
                let y1 = (y1l * scale).ceil().min(size_px.height as f32) as usize;

                let pixels = self.pixels.as_mut().unwrap();
                let frame = pixels.frame_mut();
                let w = size_px.width as usize;
                let h = size_px.height as usize;
                // 1) 先绘制“变暗背景”：如果 bg_dim 尺寸与窗口一致则整体 memcpy，否则逐行裁剪
                if let Some(bg_dim) = &self.bg_dim {
                    let bw = self.bg_w as usize;
                    let bh = self.bg_h as usize;
                    if bw == w && bh == h {
                        frame.copy_from_slice(bg_dim);
                    } else {
                        let rw = w.min(bw);
                        let rh = h.min(bh);
                        for y in 0..rh {
                            let dst = y * w * 4;
                            let src = y * bw * 4;
                            frame[dst..dst + rw * 4].copy_from_slice(&bg_dim[src..src + rw * 4]);
                        }
                        // 多余区域补黑
                        for y in rh..h {
                            let row = &mut frame[y * w * 4..(y + 1) * w * 4];
                            // RGBA 黑色
                            for px in row.chunks_exact_mut(4) {
                                px.copy_from_slice(&[0, 0, 0, 255]);
                            }
                        }
                    }
                } else if let Some(bg) = &self.bg {
                    // 未预计算 dim（理论上不会发生），退化为直接拷贝 bg
                    let bw = self.bg_w as usize;
                    let bh = self.bg_h as usize;
                    let rw = w.min(bw);
                    let rh = h.min(bh);
                    for y in 0..rh {
                        let dst = y * w * 4;
                        let src = y * bw * 4;
                        frame[dst..dst + rw * 4].copy_from_slice(&bg[src..src + rw * 4]);
                    }
                    for y in rh..h {
                        let row = &mut frame[y * w * 4..(y + 1) * w * 4];
                        for px in row.chunks_exact_mut(4) {
                            px.copy_from_slice(&[0, 0, 0, 255]);
                        }
                    }
                } else {
                    // 无背景版本：全黑
                    for y in 0..h {
                        let row = &mut frame[y * w * 4..(y + 1) * w * 4];
                        for px in row.chunks_exact_mut(4) {
                            px.copy_from_slice(&[0, 0, 0, 255]);
                        }
                    }
                }

                // 2) 若有选区：从原始 bg 恢复选区内容（避免全屏逐像素变暗）
                if x1 > x0 && y1 > y0 {
                    if let Some(bg) = &self.bg {
                        let bw = self.bg_w as usize;
                        let bh = self.bg_h as usize;
                        let rx0 = x0.min(bw);
                        let ry0 = y0.min(bh);
                        let rx1 = x1.min(bw);
                        let ry1 = y1.min(bh);
                        if rx1 > rx0 && ry1 > ry0 {
                            let rw = rx1 - rx0;
                            for y in 0..(ry1 - ry0) {
                                let dst = ((ry0 + y) * w + rx0) * 4;
                                let src = ((ry0 + y) * bw + rx0) * 4;
                                frame[dst..dst + rw * 4].copy_from_slice(&bg[src..src + rw * 4]);
                            }
                        }
                    }
                }

                if x1 > x0 && y1 > y0 {
                    for x in x0..x1 {
                        let it = (y0 * w + x) * 4;
                        let ib = ((y1 - 1) * w + x) * 4;
                        frame[it..it + 4].copy_from_slice(&[255, 255, 255, 255]);
                        frame[ib..ib + 4].copy_from_slice(&[255, 255, 255, 255]);
                    }
                    for y in y0..y1 {
                        let il = (y * w + x0) * 4;
                        let ir = (y * w + x1 - 1) * 4;
                        frame[il..il + 4].copy_from_slice(&[255, 255, 255, 255]);
                        frame[ir..ir + 4].copy_from_slice(&[255, 255, 255, 255]);
                    }
                }
                if pixels.render().is_err() {
                    event_loop.exit();
                }
                // 完成一次有效绘制，允许后续 request_redraw
                self.redraw_pending = false;
            }
        }

        impl ApplicationHandler for SelectionApp {
            fn resumed(&mut self, event_loop: &ActiveEventLoop) {
                if self.window.is_some() {
                    return;
                }
                match event_loop.create_window(self.attrs.clone()) {
                    Ok(w) => {
                        // 平台呈现设置（如 macOS 隐藏菜单栏与 Dock）
                        self.pres_guard = platform::start_presentation();
                        // 铺满当前显示器（非全屏模式，避免标题栏/关闭按钮悬浮）
                        if let Some(mon) = w.current_monitor() {
                            let size_px = mon.size();
                            let pos = mon.position();
                            let _ = w.request_inner_size(size_px);
                            w.set_outer_position(Position::Physical(PhysicalPosition::new(
                                pos.x, pos.y,
                            )));
                        }
                        // 初始化尺寸并保存窗口句柄
                        self.size_px = w.inner_size();
                        self.window = Some(Box::new(w));
                        // 预计算变暗背景，避免每帧逐像素处理
                        if self.bg_dim.is_none() {
                            if let Some(bg) = &self.bg {
                                let mut dim = vec![0u8; bg.len()];
                                let a = 90u8 as u16; // 与原先 dim_alpha 保持一致
                                for (i, chunk) in bg.chunks_exact(4).enumerate() {
                                    let r = chunk[0] as u16;
                                    let g = chunk[1] as u16;
                                    let b = chunk[2] as u16;
                                    let base = i * 4;
                                    dim[base] = ((r * (255 - a)) / 255) as u8;
                                    dim[base + 1] = ((g * (255 - a)) / 255) as u8;
                                    dim[base + 2] = ((b * (255 - a)) / 255) as u8;
                                    dim[base + 3] = 255;
                                }
                                self.bg_dim = Some(dim);
                            }
                        }
                        // 预热：尽早创建并渲染一次 Pixels，避免首次输入触发时的初始化卡顿
                        let _ = self.ensure_pixels();
                        self.render_once(event_loop);
                        // 预热完成后再显示窗口
                        if let Some(win) = self.window.as_ref() {
                            win.set_visible(true);
                            if !self.redraw_pending {
                                self.redraw_pending = true;
                                win.request_redraw();
                            }
                        }
                    }
                    Err(_) => event_loop.exit(),
                }
            }

            fn window_event(
                &mut self,
                event_loop: &ActiveEventLoop,
                window_id: winit::window::WindowId,
                event: WindowEvent,
            ) {
                let Some(window) = self.window.as_ref() else {
                    return;
                };
                if window.id() != window_id {
                    return;
                }
                match event {
                    WindowEvent::CloseRequested => {
                        self.result = None;
                        // no-op: simple fullscreen已移除
                        if let Some(g) = self.pres_guard.take() {
                            platform::end_presentation(g);
                        }
                        event_loop.exit();
                    }
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                logical_key, state, ..
                            },
                        ..
                    } => {
                        if state == ElementState::Pressed && logical_key.to_text() == Some("\u{1b}")
                        {
                            self.result = None;
                            event_loop.exit();
                            return;
                        }
                        match logical_key {
                            Key::Named(NamedKey::Shift) => {
                                self.shift_down = state == ElementState::Pressed;
                                if let Some(w) = self.window.as_ref() {
                                    if !self.redraw_pending {
                                        self.redraw_pending = true;
                                        w.request_redraw();
                                    }
                                }
                            }
                            Key::Named(NamedKey::Alt) => {
                                self.alt_down = state == ElementState::Pressed;
                                if let Some(w) = self.window.as_ref() {
                                    if !self.redraw_pending {
                                        self.redraw_pending = true;
                                        w.request_redraw();
                                    }
                                }
                            }
                            Key::Named(NamedKey::ArrowLeft) if state == ElementState::Pressed => {
                                self.curr.0 -= 1.0;
                                if let Some(w) = self.window.as_ref() {
                                    if !self.redraw_pending {
                                        self.redraw_pending = true;
                                        w.request_redraw();
                                    }
                                }
                            }
                            Key::Named(NamedKey::ArrowRight) if state == ElementState::Pressed => {
                                self.curr.0 += 1.0;
                                if let Some(w) = self.window.as_ref() {
                                    if !self.redraw_pending {
                                        self.redraw_pending = true;
                                        w.request_redraw();
                                    }
                                }
                            }
                            Key::Named(NamedKey::ArrowUp) if state == ElementState::Pressed => {
                                self.curr.1 -= 1.0;
                                if let Some(w) = self.window.as_ref() {
                                    if !self.redraw_pending {
                                        self.redraw_pending = true;
                                        w.request_redraw();
                                    }
                                }
                            }
                            Key::Named(NamedKey::ArrowDown) if state == ElementState::Pressed => {
                                self.curr.1 += 1.0;
                                if let Some(w) = self.window.as_ref() {
                                    if !self.redraw_pending {
                                        self.redraw_pending = true;
                                        w.request_redraw();
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: MouseButton::Left,
                        ..
                    } => {
                        self.dragging = true;
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Released,
                        button: MouseButton::Left,
                        ..
                    } => {
                        if self.dragging {
                            self.dragging = false;
                        }
                        let (x0l, y0l, x1l, y1l) = self.rect_logical();
                        let w = (x1l - x0l).abs();
                        let h = (y1l - y0l).abs();
                        if w >= 1.0 && h >= 1.0 {
                            let scale = self
                                .window
                                .as_ref()
                                .map(|w| w.scale_factor() as f32)
                                .unwrap_or(1.0);
                            self.result = Some(Region::new(x0l, y0l, w, h, scale));
                        }
                        // no-op: simple fullscreen已移除
                        if let Some(g) = self.pres_guard.take() {
                            platform::end_presentation(g);
                        }
                        event_loop.exit();
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        let scale = self
                            .window
                            .as_ref()
                            .map(|w| w.scale_factor())
                            .unwrap_or(1.0);
                        let p = position.to_logical::<f64>(scale);
                        let (x, y) = (p.x as f32, p.y as f32);
                        if self.dragging {
                            self.curr = (x, y);
                        } else {
                            self.start = (x, y);
                            self.curr = (x, y);
                        }
                        if let Some(w) = self.window.as_ref() {
                            if !self.redraw_pending {
                                self.redraw_pending = true;
                                w.request_redraw();
                            }
                        }
                    }
                    WindowEvent::Resized(new_size) => {
                        self.size_px = new_size;
                        if let Some(p) = self.pixels.as_mut() {
                            let _ = p.resize_surface(new_size.width, new_size.height);
                        }
                        if let Some(w) = self.window.as_ref() {
                            if !self.redraw_pending {
                                self.redraw_pending = true;
                                w.request_redraw();
                            }
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        self.render_once(event_loop);
                    }
                    _ => {}
                }
            }

            fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
                // 空闲时不强制重绘，按需在输入或尺寸变化时 request_redraw
            }
        }

        let event_loop =
            EventLoop::new().map_err(|e| OverlayError::Internal(format!("event loop: {e}")))?;
        let attrs = WindowAttributes::default()
            // 置顶，防止被其他窗口遮挡
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(false)
            // 先隐藏窗口，预热渲染后再显示，避免首次交互卡顿
            .with_visible(false);
        let mut app = SelectionApp {
            attrs,
            window: None,
            pixels: None,
            pres_guard: None,
            size_px: PhysicalSize::new(1, 1),
            bg: bg_rgba,
            bg_w,
            bg_h,
            bg_dim: None,
            dragging: false,
            start: (0.0, 0.0),
            curr: (0.0, 0.0),
            result: None,
            shift_down: false,
            alt_down: false,
            redraw_pending: false,
        };

        if let Err(e) = event_loop.run_app(&mut app) {
            return Err(OverlayError::Internal(format!("event loop run: {e}")));
        }

        Ok(app.result)
    }
}
