use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub pixel_format: PixelFormat,
    pub bytes: Arc<[u8]>, // BGRA or RGBA depending on platform normalization
}

#[derive(Debug, Clone)]
pub enum PixelFormat {
    Bgra8,
    Rgba8,
}

#[derive(Debug, Clone)]
pub struct FrameSet {
    pub primary: Frame,
    pub all: Vec<Frame>,
    // layout info TODO: add DisplayLayout when defined
}

#[derive(Debug, Clone)]
pub struct Screenshot {
    pub id: Uuid,
    pub raw: Arc<FrameSet>,
    pub scale: f32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AnnotationMeta {
    pub id: Uuid,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub rotation: u16,
    pub opacity: f32,
    pub stroke_color: Option<String>,
    pub fill_color: Option<String>,
    pub stroke_width: Option<f32>,
    pub z: i32,
    pub locked: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum AnnotationKind {
    Rect { corner_radius: u8 },
    Arrow { head_size: u8, line_style: LineStyle },
    Text { content: String, font_family: String, font_size: u32 },
    Highlight { mode: BlendMode },
    Mosaic { level: u8 },
    Freehand { points: Vec<(f32, f32)>, smoothing: f32 },
}

#[derive(Debug, Clone)]
pub enum LineStyle {
    Solid,
    Dashed,
}

#[derive(Debug, Clone)]
pub enum BlendMode {
    Multiply,
    Screen,
}

#[derive(Debug, Clone)]
pub struct Annotation {
    pub meta: AnnotationMeta,
    pub kind: AnnotationKind,
}
