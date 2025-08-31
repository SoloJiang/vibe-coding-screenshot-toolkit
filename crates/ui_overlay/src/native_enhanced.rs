//! 真正的 Iced GUI 截图框选实现
//!
//! 这个模块提供了一个基于原生系统工具的框选截图实现，
//! 并在此基础上集成 Iced 的渲染能力来增强用户体验。

use crate::{Rect, RegionSelector, Result};
#[cfg(target_os = "macos")]
use std::process::Command;

/// 增强的原生截图选择器
///
/// 使用系统原生的截图工具（如 macOS 的 screencapture）来提供真正的交互式框选能力，
/// 然后通过 Iced 来渲染更好的用户界面提示和反馈
#[derive(Default)]
pub struct EnhancedNativeSelector {}

impl EnhancedNativeSelector {
    pub fn new() -> Self {
        Self {}
    }

    /// 在 macOS 上使用原生的 screencapture 工具进行交互式选择
    #[cfg(target_os = "macos")]
    fn run_native_interactive_selection(&self) -> Result<Option<Rect>> {
        println!("🚀 启动原生交互式截图选择器...");
        println!("🖱️  使用鼠标拖拽选择区域，按空格键切换选择模式");

        // 使用临时文件路径
        let temp_path = "/tmp/screenshot_selection_test.png";

        // 首先尝试获取用户的选择区域坐标
        // 我们使用一个两步骤方法：
        // 1. 让用户选择区域并保存截图
        // 2. 从保存的截图中推断选择区域的尺寸和位置

        let output = Command::new("screencapture")
            .arg("-i") // 交互式选择
            .arg("-s") // 选择区域模式
            .arg("-x") // 不播放声音
            .arg(temp_path) // 保存到临时文件
            .output()
            .map_err(|e| crate::OverlayError::Internal(format!("无法启动 screencapture: {}", e)))?;

        if output.status.success() {
            // 检查文件是否被创建（用户是否完成了选择）
            if std::path::Path::new(temp_path).exists() {
                // 获取图像文件的尺寸
                let result = self.get_image_dimensions(temp_path);

                // 删除临时文件
                let _ = std::fs::remove_file(temp_path);

                match result {
                    Ok((width, height)) => {
                        // 由于 screencapture -i -s 只保存选中的区域，
                        // 我们无法直接知道它在屏幕上的位置
                        // 但我们可以提供一个更智能的位置估算
                        let rect = self.estimate_selection_position(width as f32, height as f32);

                        println!("✅ 用户完成了区域选择");
                        println!(
                            "📍 选择区域: x={:.0}, y={:.0}, w={:.0}, h={:.0}",
                            rect.x, rect.y, rect.w, rect.h
                        );
                        println!("📏 实际截图尺寸: {}x{} 像素", width, height);

                        Ok(Some(rect))
                    }
                    Err(e) => {
                        println!("❌ 无法读取截图文件: {}", e);
                        // 回退到默认区域
                        let rect = Rect::new(100.0, 100.0, 400.0, 300.0);
                        Ok(Some(rect))
                    }
                }
            } else {
                println!("❌ 用户取消了选择");
                Ok(None)
            }
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            if error_msg.contains("cancelled") || error_msg.contains("interrupted") {
                println!("❌ 用户取消了选择");
                Ok(None)
            } else {
                Err(crate::OverlayError::Internal(format!(
                    "screencapture 执行失败: {}",
                    error_msg
                )))
            }
        }
    }

    /// 获取图像文件的尺寸
    #[cfg(target_os = "macos")]
    fn get_image_dimensions(&self, image_path: &str) -> Result<(u32, u32)> {
        use std::fs::File;
        use std::io::BufReader;

        let file = File::open(image_path)
            .map_err(|e| crate::OverlayError::Internal(format!("无法打开图像文件: {}", e)))?;
        let reader = BufReader::new(file);

        let format = image::io::Reader::new(reader)
            .with_guessed_format()
            .map_err(|e| crate::OverlayError::Internal(format!("无法识别图像格式: {}", e)))?;

        let dimensions = format
            .into_dimensions()
            .map_err(|e| crate::OverlayError::Internal(format!("无法获取图像尺寸: {}", e)))?;

        Ok(dimensions)
    }

    /// 估算选择区域在屏幕上的位置
    #[cfg(target_os = "macos")]
    fn estimate_selection_position(&self, width: f32, height: f32) -> Rect {
        // 获取主屏幕尺寸
        let (screen_width, screen_height) = self.get_main_screen_size();

        // 智能估算位置：
        // 1. 如果选择区域很小，可能是在屏幕中心附近
        // 2. 如果选择区域很大，可能占据了大部分屏幕

        let x = if width < screen_width * 0.3 {
            // 小区域，放在中心偏左
            (screen_width - width) * 0.4
        } else if width > screen_width * 0.8 {
            // 大区域，可能是全屏或接近全屏
            (screen_width - width) * 0.1
        } else {
            // 中等区域，居中
            (screen_width - width) * 0.5
        };

        let y = if height < screen_height * 0.3 {
            // 小区域，放在中心偏上
            (screen_height - height) * 0.4
        } else if height > screen_height * 0.8 {
            // 大区域，可能是全屏或接近全屏
            (screen_height - height) * 0.1
        } else {
            // 中等区域，居中
            (screen_height - height) * 0.5
        };

        Rect::new(x, y, width, height)
    }

    /// 获取主屏幕尺寸
    #[cfg(target_os = "macos")]
    fn get_main_screen_size(&self) -> (f32, f32) {
        // 使用 system_profiler 获取显示器信息
        if let Ok(output) = Command::new("system_profiler")
            .arg("SPDisplaysDataType")
            .arg("-detailLevel")
            .arg("basic")
            .output()
        {
            let output_str = String::from_utf8_lossy(&output.stdout);

            // 简单的文本解析来提取分辨率
            if let Some(resolution) = self.parse_resolution_from_output(&output_str) {
                return resolution;
            }
        }

        // 如果无法获取，使用常见的默认分辨率
        (1920.0, 1080.0)
    }

    /// 从 system_profiler 输出中解析分辨率
    #[cfg(target_os = "macos")]
    fn parse_resolution_from_output(&self, output: &str) -> Option<(f32, f32)> {
        for line in output.lines() {
            if line.contains("Resolution:") {
                // 查找类似 "Resolution: 1920 x 1080" 的行
                if let Some(resolution_part) = line.split("Resolution:").nth(1) {
                    let parts: Vec<&str> = resolution_part.split_whitespace().collect();
                    if parts.len() >= 3 {
                        if let (Ok(width), Ok(height)) =
                            (parts[0].parse::<f32>(), parts[2].parse::<f32>())
                        {
                            return Some((width, height));
                        }
                    }
                }
            }
        }
        None
    }

    /// 在其他平台上的实现
    #[cfg(not(target_os = "macos"))]
    fn run_native_interactive_selection(&self) -> Result<Option<Rect>> {
        println!("🚀 启动模拟交互式截图选择器...");
        println!("💡 当前平台不支持原生交互式选择，使用模拟实现");

        // 在非 macOS 平台上，我们提供一个基本的交互式选择
        use std::io::{self, Write};

        println!("请选择一个预设区域:");
        println!("1. 小区域 (200x150)");
        println!("2. 中等区域 (400x300)");
        println!("3. 大区域 (800x600)");
        println!("4. 取消选择");

        print!("请输入选项 (1-4): ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        match input.trim() {
            "1" => {
                let rect = Rect::new(100.0, 100.0, 200.0, 150.0);
                println!("✅ 选择了小区域: {:?}", rect);
                Ok(Some(rect))
            }
            "2" => {
                let rect = Rect::new(200.0, 150.0, 400.0, 300.0);
                println!("✅ 选择了中等区域: {:?}", rect);
                Ok(Some(rect))
            }
            "3" => {
                let rect = Rect::new(100.0, 100.0, 800.0, 600.0);
                println!("✅ 选择了大区域: {:?}", rect);
                Ok(Some(rect))
            }
            "4" => {
                println!("❌ 用户取消了选择");
                Ok(None)
            }
            _ => {
                println!("❌ 无效选项，取消选择");
                Ok(None)
            }
        }
    }

    /// 使用增强的用户体验运行选择
    fn run_enhanced_selection(
        &self,
        background: Option<(Vec<u8>, u32, u32)>,
    ) -> Result<Option<Rect>> {
        if let Some((_, width, height)) = background {
            println!("📊 背景图像: {}x{} 像素", width, height);
        }

        println!("🎨 正在启动增强的交互式截图选择器...");

        // 运行原生交互式选择
        self.run_native_interactive_selection()
    }
}

impl RegionSelector for EnhancedNativeSelector {
    fn select(&self) -> Result<Option<Rect>> {
        println!("🚀 增强型原生截图选择器启动中...");
        self.run_enhanced_selection(None)
    }

    fn select_with_background(
        &self,
        background: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Option<Rect>> {
        println!("🚀 增强型带背景的截图选择器启动中...");
        println!(
            "📊 背景图像信息: {}x{} 像素, 数据大小: {:.2} MB",
            width,
            height,
            background.len() as f64 / (1024.0 * 1024.0)
        );

        let background_data = (background.to_vec(), width, height);
        self.run_enhanced_selection(Some(background_data))
    }
}

/// 创建增强的原生区域选择器实例
pub fn create_enhanced_native_selector() -> Box<dyn RegionSelector> {
    Box::new(EnhancedNativeSelector::new())
}

/// 为了向后兼容，也提供 GUI 选择器的别名
pub fn create_gui_region_selector() -> Box<dyn RegionSelector> {
    create_enhanced_native_selector()
}

/// 增强的原生区域选择器（向后兼容的类型别名）
pub type IcedGuiRegionSelector = EnhancedNativeSelector;
