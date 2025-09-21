use chrono::{DateTime, Utc};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Region {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// 逻辑坐标对应的缩放因子 (HiDPI 下为屏幕 scale)
    pub scale: f32,
}

impl Region {
    pub fn new(x: f32, y: f32, w: f32, h: f32, scale: f32) -> Self {
        Self { x, y, w, h, scale }
    }

    pub fn norm(&self) -> Self {
        let (mut x, mut y, mut w, mut h) = (self.x, self.y, self.w, self.h);
        if w < 0.0 {
            x += w;
            w = -w;
        }
        if h < 0.0 {
            y += h;
            h = -h;
        }
        Self {
            x,
            y,
            w,
            h,
            scale: self.scale,
        }
    }
}

#[derive(Debug, Error)]
pub enum OverlayError {
    #[error("user cancelled")]
    Cancelled,
    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, OverlayError>;
pub type MaybeRegion = std::result::Result<Option<Region>, OverlayError>;

/// 选择器契约：阻塞选择一个矩形区域；取消则返回 Err(Cancelled)
pub trait RegionSelector {
    fn select(&self) -> Result<Region>;
    /// 可选：携带背景 RGB 缓冲（width*height*3）。默认忽略背景。
    /// 语义：
    /// - Ok(Some(r)) -> 用户确认区域
    /// - Ok(None) -> 用户取消
    /// - Err(e) -> 内部错误
    fn select_with_background(&self, _rgb: &[u8], _width: u32, _height: u32) -> MaybeRegion {
        match self.select() {
            Ok(r) => Ok(Some(r)),
            Err(OverlayError::Cancelled) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// 新增：支持虚拟桌面背景的区域选择
    /// virtual_bounds: 虚拟桌面的边界信息 (min_x, min_y, width, height)
    /// display_offset: 当前交互显示器在虚拟桌面中的偏移 (x, y)
    /// 返回的Region坐标是相对于虚拟桌面的全局坐标
    fn select_with_virtual_background(
        &self,
        _rgb: &[u8],
        _width: u32,
        _height: u32,
        _virtual_bounds: (i32, i32, u32, u32), // (min_x, min_y, width, height)
        _display_offset: (i32, i32),           // (x, y)
    ) -> MaybeRegion {
        // 默认实现：对于虚拟桌面模式，直接使用虚拟桌面背景
        // 坐标不需要转换，因为背景就是完整的虚拟桌面
        self.select_with_background(_rgb, _width, _height)
    }
}

/// 一个 mock：立即返回给定区域；用于非图形环境测试
pub struct MockSelector {
    pub region: parking_lot::Mutex<Option<Region>>,
}

impl MockSelector {
    pub fn new(region: Option<Region>) -> Self {
        Self {
            region: parking_lot::Mutex::new(region),
        }
    }
}

impl RegionSelector for MockSelector {
    fn select(&self) -> Result<Region> {
        let mut g = self.region.lock();
        match g.take() {
            Some(r) => Ok(r),
            None => Err(OverlayError::Cancelled),
        }
    }
}

mod coordinate_utils;
mod event_handler;
pub mod platform;
mod renderer;
mod selector;
mod window_manager;

pub fn create_gui_region_selector() -> Box<dyn RegionSelector> {
    Box::new(selector::WinitRegionSelector::new())
}

/// 选择结果的审计信息（预留）
#[derive(Debug, Clone)]
pub struct SelectionAudit {
    pub id: uuid::Uuid,
    pub at: DateTime<Utc>,
    pub region: Option<Region>,
}

impl SelectionAudit {
    pub fn cancelled() -> Self {
        let ts = uuid::Timestamp::now(uuid::NoContext);
        Self {
            id: uuid::Uuid::new_v7(ts),
            at: Utc::now(),
            region: None,
        }
    }
    pub fn finished(r: Region) -> Self {
        let ts = uuid::Timestamp::now(uuid::NoContext);
        Self {
            id: uuid::Uuid::new_v7(ts),
            at: Utc::now(),
            region: Some(r),
        }
    }
}
