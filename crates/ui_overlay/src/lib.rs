//! ui_overlay: 基于 Iced 的截图框选 UI
//!
//! 提供跨平台区域选择 (Region Selection) 能力，使用 Iced 实现轻量级、高性能的截图框选界面。
//! 支持鼠标拖拽、控制点调整、键盘快捷键等交互方式。

use thiserror::Error;

#[derive(Debug, Error)]
pub enum OverlayError {
    #[error("unsupported platform overlay")]
    Unsupported,
    #[error("user canceled selection")]
    Canceled,
    #[error("internal error: {0}")]
    Internal(String),
    #[error("Iced initialization failed: {0}")]
    IcedInitializationFailed(String),
    #[error("Image creation failed: {0}")]
    ImageCreationFailed(String),
    #[error("Event handling failed: {0}")]
    EventHandlingFailed(String),
    #[error("Invalid selection bounds")]
    InvalidBounds,
}

pub type Result<T> = std::result::Result<T, OverlayError>;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn normalized(self) -> Option<Self> {
        if self.w > 0.0 && self.h > 0.0 {
            Some(self)
        } else {
            None
        }
    }

    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.w && y >= self.y && y <= self.y + self.h
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        !(self.x + self.w < other.x
            || other.x + other.w < self.x
            || self.y + self.h < other.y
            || other.y + other.h < self.y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub w: f32,
    pub h: f32,
}

impl Size {
    pub fn new(w: f32, h: f32) -> Self {
        Self { w, h }
    }
}

/// 区域选择器 Trait
pub trait RegionSelector: Send + Sync {
    /// 触发一次交互式框选
    /// 成功：Ok(Some(Rect))；用户取消：Ok(None)；不支持：Err(Unsupported)
    fn select(&self) -> Result<Option<Rect>>;

    /// 带背景图像的框选（可选实现）
    fn select_with_background(
        &self,
        _background: &[u8],
        _width: u32,
        _height: u32,
    ) -> Result<Option<Rect>> {
        self.select()
    }
}

/// 占位实现：返回 Unsupported
pub struct StubRegionSelector;

impl RegionSelector for StubRegionSelector {
    fn select(&self) -> Result<Option<Rect>> {
        Err(OverlayError::Unsupported)
    }
}

// Iced 实现
pub mod iced_overlay;
pub mod native_enhanced;
pub use iced_overlay::{
    create_region_selector, create_region_selector_with_config, IcedRegionSelector,
};
pub use native_enhanced::{
    create_enhanced_native_selector, create_gui_region_selector, EnhancedNativeSelector,
    IcedGuiRegionSelector,
};
