use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

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

/// 元信息: 基础几何与通用属性
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(with = "ts_millis")]
    pub created_at: DateTime<Utc>,
}

/// 标注类型及特有属性
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnnotationKind {
    Rect {
        corner_radius: u8,
    },
    Arrow {
        head_size: u8,
        line_style: LineStyle,
    },
    Text {
        content: String,
        font_family: String,
        font_size: u32,
    },
    Highlight {
        mode: BlendMode,
    },
    Mosaic {
        level: u8,
    },
    Freehand {
        points: Vec<(f32, f32)>,
        smoothing: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LineStyle {
    Solid,
    Dashed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlendMode {
    Multiply,
    Screen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub meta: AnnotationMeta,
    pub kind: AnnotationKind,
}

/// 历史条目 (内存结构) – TODO: 未来若持久化可单独拆文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: Uuid,
    pub path: String,
    /// 可能是缩略图 PNG bytes（小尺寸）
    pub thumb: Option<Vec<u8>>,
    #[serde(with = "ts_millis")]
    pub created_at: DateTime<Utc>,
    pub title: Option<String>,
    /// 版本占位, 便于未来演进 (序列化向前兼容)
    pub version: u8,
}

/// 将新的 HistoryItem 推入集合，超过 capacity 自动裁剪最旧
pub fn push_history_trim(list: &mut Vec<HistoryItem>, item: HistoryItem, capacity: usize) {
    list.push(item);
    if list.len() > capacity {
        // 按创建时间倒序保留最近 capacity 个
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        list.truncate(capacity);
    }
}

#[cfg(test)]
mod history_tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn trim_history_capacity() {
        let mut v = Vec::new();
        let now = Utc::now();
        for i in 0..60u32 {
            push_history_trim(
                &mut v,
                HistoryItem {
                    id: Uuid::now_v7(),
                    path: format!("/tmp/{}.png", i),
                    thumb: None,
                    created_at: now + Duration::seconds(i as i64),
                    title: None,
                    version: 1,
                },
                50,
            );
        }
        assert_eq!(v.len(), 50);
        // 确保都是最新 50 条 (最大 created_at - 最小 created_at == 49s)
        let max = v.iter().map(|h| h.created_at).max().unwrap();
        let min = v.iter().map(|h| h.created_at).min().unwrap();
        assert_eq!((max - min).num_seconds(), 49);
    }
}

// serde helper for DateTime<Utc> as millis
mod ts_millis {
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};
    pub fn serialize<S>(dt: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_i64(dt.timestamp_millis())
    }
    pub fn deserialize<'de, D>(d: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let ms = i64::deserialize(d)?;
        Ok(Utc
            .timestamp_millis_opt(ms)
            .single()
            .ok_or_else(|| serde::de::Error::custom("invalid millis"))?)
    }
}
