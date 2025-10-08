/// 图像缓存模块
///
/// 缓存 Skia Image 对象，避免每帧重新创建
use skia_safe::Image;
use std::sync::Arc;

/// 图像缓存
pub struct ImageCache {
    /// 暗化后的背景图像
    tinted_image: Option<Arc<Image>>,
    /// 原始背景图像
    original_image: Option<Arc<Image>>,
    /// 缓存的数据哈希（用于检测变化）
    data_hash: u64,
}

impl ImageCache {
    pub fn new() -> Self {
        Self {
            tinted_image: None,
            original_image: None,
            data_hash: 0,
        }
    }

    /// 获取或创建暗化背景图像
    pub fn get_or_create_tinted_image(
        &mut self,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Option<Arc<Image>> {
        let hash = Self::calculate_hash(data);

        // 如果数据没变，直接返回缓存
        if self.data_hash == hash && self.tinted_image.is_some() {
            return self.tinted_image.clone();
        }

        // 创建新图像
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

        let arc_image = Arc::new(image);
        self.tinted_image = Some(arc_image.clone());
        self.data_hash = hash;

        Some(arc_image)
    }

    /// 获取或创建原始背景图像
    pub fn get_or_create_original_image(
        &mut self,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Option<Arc<Image>> {
        let hash = Self::calculate_hash(data);

        // 如果数据没变，直接返回缓存
        if self.data_hash == hash && self.original_image.is_some() {
            return self.original_image.clone();
        }

        // 创建新图像
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

        let arc_image = Arc::new(image);
        self.original_image = Some(arc_image.clone());
        self.data_hash = hash;

        Some(arc_image)
    }

    /// 计算数据哈希（简单快速的哈希）
    fn calculate_hash(data: &[u8]) -> u64 {
        // 使用 FNV-1a 哈希算法（快速且足够好）
        const FNV_OFFSET: u64 = 14695981039346656037;
        const FNV_PRIME: u64 = 1099511628211;

        let mut hash = FNV_OFFSET;

        // 采样策略：只哈希部分数据以提高性能
        // 对于大图像，每隔 N 个字节采样一次
        let step = if data.len() > 1024 * 1024 {
            1024 // 对于 > 1MB 的数据，每隔 1KB 采样
        } else {
            1 // 小数据全部哈希
        };

        for byte in data.iter().step_by(step) {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }

        hash
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
    fn test_hash_calculation() {
        let data1 = vec![1u8, 2, 3, 4, 5];
        let data2 = vec![1u8, 2, 3, 4, 5];
        let data3 = vec![1u8, 2, 3, 4, 6];

        let hash1 = ImageCache::calculate_hash(&data1);
        let hash2 = ImageCache::calculate_hash(&data2);
        let hash3 = ImageCache::calculate_hash(&data3);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
