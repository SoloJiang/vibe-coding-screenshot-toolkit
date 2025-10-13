/// 图像缓存模块
///
/// 缓存 Skia Image 对象，避免每帧重新创建
use skia_safe::Image;
use std::sync::Arc;

/// 图像缓存
///
/// 性能优化：背景图片在整个选择过程中不会变化，
/// 因此只需在初始化时创建一次，后续直接复用
pub struct ImageCache {
    /// 暗化后的背景图像
    tinted_image: Option<Arc<Image>>,
    /// 原始背景图像
    original_image: Option<Arc<Image>>,
    /// 是否已初始化
    initialized: bool,
}

impl ImageCache {
    pub fn new() -> Self {
        Self {
            tinted_image: None,
            original_image: None,
            initialized: false,
        }
    }

    /// 一次性初始化所有图像缓存
    ///
    /// 背景图片在整个选择过程中不会变化，因此只需初始化一次
    pub fn ensure_images_cached(
        &mut self,
        original_bg: &[u8],
        tinted_bg: &[u8],
        width: u32,
        height: u32,
    ) {
        if self.initialized {
            return;
        }

        self.original_image = Self::create_image(original_bg, width, height);
        self.tinted_image = Self::create_image(tinted_bg, width, height);
        self.initialized = true;
    }

    /// 获取暗化背景图像（返回 Arc 引用，零拷贝）
    pub fn get_tinted_image(&self) -> Option<Arc<Image>> {
        self.tinted_image.clone()
    }

    /// 获取原始背景图像（返回 Arc 引用，零拷贝）
    pub fn get_original_image(&self) -> Option<Arc<Image>> {
        self.original_image.clone()
    }

    /// 创建 Skia Image
    fn create_image(data: &[u8], width: u32, height: u32) -> Option<Arc<Image>> {
        let info = skia_safe::ImageInfo::new(
            (width as i32, height as i32),
            skia_safe::ColorType::RGBA8888,
            skia_safe::AlphaType::Premul,
            None,
        );

        let image = skia_safe::images::raster_from_data(
            &info,
            skia_safe::Data::new_copy(data),
            width as usize * 4,
        )?;

        Some(Arc::new(image))
    }
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_cache_initialization() {
        let cache = ImageCache::new();
        assert!(!cache.initialized);
        assert!(cache.tinted_image.is_none());
        assert!(cache.original_image.is_none());
    }
}
