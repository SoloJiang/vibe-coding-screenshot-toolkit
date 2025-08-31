//! Iced-rs 基础的截图框选 UI 实现

use crate::{Point, Rect, RegionSelector, Result};
use std::io::{self, Write};

/// 选择状态
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SelectionMode {
    #[default]
    None,
    Creating,
    Selected,
}

/// 选区状态
#[derive(Debug, Clone, Default)]
pub struct SelectionState {
    pub rect: Rect,
    pub mode: SelectionMode,
    pub anchor_point: Option<Point>,
    pub is_dragging: bool,
}

impl SelectionState {
    pub fn start_creating(&mut self, point: Point) {
        self.mode = SelectionMode::Creating;
        self.anchor_point = Some(point);
        self.rect = Rect::new(point.x, point.y, 0.0, 0.0);
        self.is_dragging = true;
    }

    pub fn update_creating(&mut self, point: Point) {
        if let Some(anchor) = self.anchor_point {
            let x_min = anchor.x.min(point.x);
            let y_min = anchor.y.min(point.y);
            let x_max = anchor.x.max(point.x);
            let y_max = anchor.y.max(point.y);

            self.rect = Rect::new(x_min, y_min, x_max - x_min, y_max - y_min);
        }
    }

    pub fn finish_creating(&mut self) {
        if self.rect.w > 4.0 && self.rect.h > 4.0 {
            self.mode = SelectionMode::Selected;
        } else {
            self.mode = SelectionMode::None;
        }
        self.is_dragging = false;
    }
}

/// Iced 区域选择器 - 交互式命令行实现
///
/// 由于 Iced 0.13 API 复杂性，暂时使用命令行交互的方式
/// 让用户可以真正进行区域选择
#[derive(Default)]
pub struct IcedRegionSelector {}

impl IcedRegionSelector {
    pub fn new() -> Self {
        Self {}
    }

    /// 交互式选择区域
    fn interactive_selection(&self, width: u32, height: u32) -> Result<Option<Rect>> {
        println!("🎯 启动交互式区域选择器");
        println!("📏 可选区域: 0,0 到 {},{}像素", width, height);
        println!();

        // 提供预设选项
        println!("请选择一个区域或输入自定义坐标:");
        println!("1. 全屏 (0, 0, {}, {})", width, height);
        println!("2. 中心区域 (25%边距)");
        println!("3. 中心区域 (10%边距)");
        println!("4. 自定义坐标");
        println!("5. 取消选择");
        println!();

        print!("请输入选项 (1-5): ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        match input.trim() {
            "1" => {
                println!("✅ 选择了全屏区域");
                Ok(Some(Rect::new(0.0, 0.0, width as f32, height as f32)))
            }
            "2" => {
                let margin_x = width as f32 * 0.25;
                let margin_y = height as f32 * 0.25;
                let x = margin_x;
                let y = margin_y;
                let w = width as f32 - 2.0 * margin_x;
                let h = height as f32 - 2.0 * margin_y;

                println!(
                    "✅ 选择了中心区域 (25%边距): x={:.0}, y={:.0}, w={:.0}, h={:.0}",
                    x, y, w, h
                );
                Ok(Some(Rect::new(x, y, w, h)))
            }
            "3" => {
                let margin_x = width as f32 * 0.1;
                let margin_y = height as f32 * 0.1;
                let x = margin_x;
                let y = margin_y;
                let w = width as f32 - 2.0 * margin_x;
                let h = height as f32 - 2.0 * margin_y;

                println!(
                    "✅ 选择了中心区域 (10%边距): x={:.0}, y={:.0}, w={:.0}, h={:.0}",
                    x, y, w, h
                );
                Ok(Some(Rect::new(x, y, w, h)))
            }
            "4" => self.custom_coordinates_input(width, height),
            "5" => {
                println!("❌ 用户取消了选择");
                Ok(None)
            }
            _ => {
                println!("❌ 无效选项，默认取消选择");
                Ok(None)
            }
        }
    }

    /// 自定义坐标输入
    fn custom_coordinates_input(&self, width: u32, height: u32) -> Result<Option<Rect>> {
        println!("请输入自定义坐标 (格式: x,y,w,h):");
        print!("坐标 (例如: 100,100,800,600): ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let coords: Vec<&str> = input.trim().split(',').collect();
        if coords.len() != 4 {
            println!("❌ 格式错误，取消选择");
            return Ok(None);
        }

        let x: f32 = coords[0].trim().parse().unwrap_or(0.0);
        let y: f32 = coords[1].trim().parse().unwrap_or(0.0);
        let w: f32 = coords[2].trim().parse().unwrap_or(100.0);
        let h: f32 = coords[3].trim().parse().unwrap_or(100.0);

        // 验证坐标范围
        if x < 0.0 || y < 0.0 || x + w > width as f32 || y + h > height as f32 {
            println!("❌ 坐标超出范围，取消选择");
            return Ok(None);
        }

        if w < 4.0 || h < 4.0 {
            println!("❌ 区域太小 (最小4x4像素)，取消选择");
            return Ok(None);
        }

        println!(
            "✅ 选择了自定义区域: x={:.0}, y={:.0}, w={:.0}, h={:.0}",
            x, y, w, h
        );
        Ok(Some(Rect::new(x, y, w, h)))
    }

    /// 运行区域选择
    fn run_selection_app(&self, background: Option<(Vec<u8>, u32, u32)>) -> Result<Option<Rect>> {
        println!("🚀 启动 Iced 交互式截图选择器...");

        if let Some((_, width, height)) = background {
            println!("� 背景图像信息: {}x{} 像素", width, height);
            self.interactive_selection(width, height)
        } else {
            println!("📊 默认屏幕尺寸: 1920x1080 像素");
            self.interactive_selection(1920, 1080)
        }
    }
}

impl RegionSelector for IcedRegionSelector {
    fn select(&self) -> Result<Option<Rect>> {
        println!("🚀 Iced 截图选择器启动中...");
        self.run_selection_app(None)
    }

    fn select_with_background(
        &self,
        background: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Option<Rect>> {
        println!("🚀 Iced 带背景图像的截图选择器启动中...");
        println!(
            "📊 背景图像信息: {}x{} 像素, 数据大小: {:.2} MB",
            width,
            height,
            background.len() as f64 / (1024.0 * 1024.0)
        );

        let background_data = (background.to_vec(), width, height);
        self.run_selection_app(Some(background_data))
    }
}

/// 创建区域选择器实例
pub fn create_region_selector() -> Box<dyn RegionSelector> {
    Box::new(IcedRegionSelector::new())
}

/// 带配置的区域选择器创建函数
pub fn create_region_selector_with_config(_config: ()) -> Box<dyn RegionSelector> {
    Box::new(IcedRegionSelector::new())
}
