/// 图标加载和管理模块
///
/// 负责加载 SVG 图标并转换为 egui 可用的纹理
use std::collections::HashMap;

/// SVG 图标内容（编译时嵌入）
pub struct SvgIcons {
    pub rectangle: &'static str,
    pub arrow: &'static str,
    pub brush: &'static str,
    pub check: &'static str,
    pub close: &'static str,
}

impl SvgIcons {
    /// 获取内嵌的 SVG 图标
    pub fn load() -> Self {
        Self {
            rectangle: include_str!("../../../../assets/icons/rectangle.svg"),
            arrow: include_str!("../../../../assets/icons/arrow.svg"),
            brush: include_str!("../../../../assets/icons/brush.svg"),
            check: include_str!("../../../../assets/icons/check.svg"),
            close: include_str!("../../../../assets/icons/close.svg"),
        }
    }
}

/// 图标缓存
///
/// 用于缓存渲染后的图标纹理，避免重复加载
pub struct IconCache {
    textures: HashMap<String, egui::TextureHandle>,
    svgs: SvgIcons,
}

impl IconCache {
    /// 创建新的图标缓存
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
            svgs: SvgIcons::load(),
        }
    }

    /// 获取图标的 SVG 内容
    pub fn get_svg(&self, name: &str) -> Option<&str> {
        match name {
            "rectangle" => Some(self.svgs.rectangle),
            "arrow" => Some(self.svgs.arrow),
            "brush" => Some(self.svgs.brush),
            "check" => Some(self.svgs.check),
            "close" => Some(self.svgs.close),
            _ => None,
        }
    }

    /// 将 SVG 渲染为 egui 纹理（简化版本，使用 Unicode 字符作为替代）
    ///
    /// 注意：完整的 SVG 渲染需要额外的 resvg 或类似库，这里使用文字图标作为临时方案
    pub fn get_or_create_texture(
        &mut self,
        ctx: &egui::Context,
        name: &str,
    ) -> Option<egui::TextureHandle> {
        // 如果已缓存，直接返回
        if let Some(texture) = self.textures.get(name) {
            return Some(texture.clone());
        }

        // 创建简单的图标纹理（使用单色方块作为占位符）
        // TODO: 使用 resvg 等库将 SVG 转换为位图
        let size = [24, 24];
        let color = egui::Color32::WHITE;
        let pixels: Vec<egui::Color32> = vec![color; size[0] * size[1]];

        let image = egui::ColorImage {
            size,
            pixels,
            source_size: egui::Vec2::new(size[0] as f32, size[1] as f32), // egui 0.33 新增字段
        };

        let texture = ctx.load_texture(name, image, egui::TextureOptions::default());

        self.textures.insert(name.to_string(), texture.clone());
        Some(texture)
    }
}

impl Default for IconCache {
    fn default() -> Self {
        Self::new()
    }
}
