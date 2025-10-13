use clap::{Args, Parser, Subcommand};
#[cfg(target_os = "macos")]
use platform_mac::MacCapturer;
#[cfg(target_os = "windows")]
use platform_win::WinCapturer;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
use services::StubClipboard;
use services::{gen_file_name, ExportService};
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Parser)]
#[command(
    author,
    version,
    about = "跨平台交互式截图工具 - 专注多显示器环境",
    long_about = "Screenshot Toolkit v0.1 MVP\n\n专注于交互式截图的跨平台工具，支持多显示器环境和跨显示器区域选择。\n\n特性：\n  • 交互式区域选择（鼠标拖拽）\n  • 多显示器自动检测和跨屏选择\n  • PNG 导出和剪贴板集成\n  • 智能文件命名（时间模板）\n  • 友好的权限和错误提示\n\n使用提示：\n  macOS 首次使用需要在\"系统偏好设置\"→\"安全性与隐私\"→\"隐私\"→\"屏幕录制\"中授权。"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 显示版本信息
    Version,
    /// 交互式框选截图 - 支持多显示器环境和跨显示器区域选择
    ///
    /// 启动交互式选择界面，支持鼠标拖拽选择任意矩形区域。
    /// 操作说明：
    ///   - 鼠标左键拖拽选择区域
    ///   - Enter/Space 确认截图
    ///   - Esc 取消操作
    ///   - 支持跨多个显示器的区域选择
    CaptureInteractive(CaptureInteractiveArgs),
}

#[derive(Args)]
struct CaptureInteractiveArgs {
    #[arg(
        short = 'd',
        long = "out-dir",
        default_value = ".",
        visible_alias = "out",
        short_alias = 'o',
        help = "输出目录路径"
    )]
    out_dir: PathBuf,
    #[arg(
        short = 't',
        long,
        default_value = "Screenshot-{date:yyyyMMdd-HHmmss}-{seq}",
        help = "文件名模板。支持变量：{date:format} 时间格式, {seq} 当日序列号"
    )]
    template: String,
    /// 截图后同时复制到系统剪贴板
    #[arg(long, help = "将截图同时复制到系统剪贴板")]
    clipboard: bool,
}

#[tokio::main]
async fn main() {
    // 初始化日志
    let _ = fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .try_init();

    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Version) | None => {
            println!(
                "Cross-platform Screenshot Capture Tool v{}",
                env!("CARGO_PKG_VERSION")
            );
        }
        Some(Commands::CaptureInteractive(args)) => {
            handle_interactive_capture_async(args).await;
        }
    }
}

/// 异步版本的交互式截图处理
///
/// 注意：在 macOS 上，GUI 事件循环必须在主线程运行，
/// 所以我们使用 block_in_place 在主线程上同步执行截图，
/// 然后异步处理导出操作
async fn handle_interactive_capture_async(args: CaptureInteractiveArgs) {
    #[cfg(target_os = "macos")]
    {
        // 在主线程上执行截图（macOS 的 EventLoop 必须在主线程）
        // 使用 tokio::task::block_in_place 允许在异步上下文中运行同步代码
        let shot_result = tokio::task::block_in_place(|| {
            let selector: Box<dyn ui_overlay::RegionSelector> =
                ui_overlay::create_gui_region_selector();
            MacCapturer::capture_region_interactive_custom(selector.as_ref())
        });

        match shot_result {
            Ok(shot) => {
                // 异步导出截图
                export_screenshot_async(
                    shot,
                    args.template,
                    args.out_dir,
                    "交互式截图",
                    args.clipboard,
                )
                .await;
            }
            Err(e) => {
                // 根据错误类型提供更友好的提示
                match e.to_string().as_str() {
                    s if s.contains("permission") => {
                        eprintln!("❌ 权限不足：请在\"系统偏好设置\" → \"安全性与隐私\" → \"隐私\" → \"屏幕录制\"中，勾选本应用的权限。");
                        eprintln!("💡 提示：权限设置后可能需要重启应用程序。");
                    }
                    s if s.contains("Cancelled") => {
                        eprintln!("⚠️  操作已取消");
                        std::process::exit(0); // 用户主动取消，正常退出
                    }
                    s if s.contains("display") => {
                        eprintln!("❌ 显示器检测失败：{e}");
                        eprintln!("💡 提示：请确认显示器连接正常，或尝试重新启动应用。");
                    }
                    _ => {
                        eprintln!("❌ 交互框选失败: {e}");
                        eprintln!("💡 提示：如果问题持续存在，请检查系统权限设置。");
                    }
                }
                std::process::exit(2);
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        let selector: Box<dyn ui_overlay::RegionSelector> =
            ui_overlay::create_gui_region_selector();

        match WinCapturer::capture_region_interactive_custom(selector.as_ref()) {
            Ok(shot) => {
                export_screenshot(
                    shot,
                    args.template,
                    args.out_dir,
                    "交互式截图",
                    args.clipboard,
                );
            }
            Err(e) => {
                // 根据错误类型提供更友好的提示
                match e.to_string().as_str() {
                    s if s.contains("permission") => {
                        eprintln!("❌ 权限不足：请确认应用具有屏幕捕获权限。");
                        eprintln!("💡 提示：权限设置后可能需要重启应用程序。");
                    }
                    s if s.contains("Cancelled") => {
                        eprintln!("⚠️  操作已取消");
                        std::process::exit(0); // 用户主动取消，正常退出
                    }
                    s if s.contains("display") => {
                        eprintln!("❌ 显示器检测失败：{e}");
                        eprintln!("💡 提示：请确认显示器连接正常，或尝试重新启动应用。");
                    }
                    _ => {
                        eprintln!("❌ 交互框选失败: {e}");
                        eprintln!("💡 提示：如果问题持续存在，请检查系统权限设置。");
                    }
                }
                std::process::exit(2);
            }
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        eprintln!("❌ 当前平台暂不支持交互式截图");
        std::process::exit(3);
    }
}

/// 异步版本的导出截图
///
/// 性能优化：并行执行文件导出和剪贴板复制
async fn export_screenshot_async(
    shot: screenshot_core::Screenshot,
    template: String,
    out_dir: PathBuf,
    desc: &'static str,
    clipboard: bool,
) {
    let filename = gen_file_name(&template, 1);
    let out = out_dir.join(format!("{}.png", filename));

    let export = {
        #[cfg(target_os = "macos")]
        {
            use platform_mac::MacClipboard;
            ExportService::new(Arc::new(MacClipboard))
        }
        #[cfg(target_os = "windows")]
        {
            use platform_win::WinClipboard;
            ExportService::new(Arc::new(WinClipboard))
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            use services::StubClipboard;
            ExportService::new(Arc::new(StubClipboard))
        }
    };

    // 并行执行文件导出和剪贴板复制（如果需要）
    let file_task = export.export_png_to_file_async(&shot, &[], &out);

    let clipboard_task = if clipboard {
        // 在 blocking pool 中执行剪贴板操作
        let shot_clone = shot.clone();
        let export_clone = export.clone();
        Some(tokio::task::spawn_blocking(move || {
            export_clone.export_png_to_clipboard(&shot_clone, &[])
        }))
    } else {
        None
    };

    // 等待文件导出完成
    match file_task.await {
        Ok(_) => {
            println!("✅ {}已保存: {}", desc, out.display());
        }
        Err(e) => {
            match e.to_string().as_str() {
                s if s.contains("permission") || s.contains("Permission") => {
                    eprintln!("❌ {}导出失败: 文件写入权限不足", desc);
                    eprintln!(
                        "💡 提示：请检查输出目录的写入权限：{}",
                        out.parent().unwrap_or(&out).display()
                    );
                }
                s if s.contains("No such file") || s.contains("not found") => {
                    eprintln!("❌ {}导出失败: 输出目录不存在", desc);
                    eprintln!(
                        "💡 提示：请确认目录路径正确：{}",
                        out.parent().unwrap_or(&out).display()
                    );
                }
                s if s.contains("disk") || s.contains("space") => {
                    eprintln!("❌ {}导出失败: 磁盘空间不足", desc);
                    eprintln!("💡 提示：请检查可用磁盘空间。");
                }
                _ => {
                    eprintln!("❌ {}导出失败: {e}", desc);
                    eprintln!("💡 提示：请检查输出路径和权限设置。");
                }
            }
            std::process::exit(1);
        }
    }

    // 等待剪贴板操作完成（如果有）
    if let Some(task) = clipboard_task {
        match task.await {
            Ok(Ok(_)) => {
                println!("📋 已复制到剪贴板");
            }
            Ok(Err(e)) => {
                eprintln!("⚠️  剪贴板复制失败: {e}");
            }
            Err(e) => {
                eprintln!("⚠️  剪贴板任务失败: {e}");
            }
        }
    }
}
