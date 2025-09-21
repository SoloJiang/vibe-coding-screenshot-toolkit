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
#[command(author, version, about = "Cross-platform Screenshot Capture Tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Version,
    /// 交互式框选截图（支持多显示器和跨显示器）
    CaptureInteractive(CaptureInteractiveArgs),
    /// 全屏截图
    Capture(CaptureArgs),
    /// 区域截图
    CaptureRegion(CaptureRegionArgs),
}

#[derive(Args)]
struct CaptureInteractiveArgs {
    #[arg(
        short = 'd',
        long = "out-dir",
        default_value = ".",
        visible_alias = "out",
        short_alias = 'o'
    )]
    out_dir: PathBuf,
    #[arg(
        short = 't',
        long,
        default_value = "Screenshot-{date:yyyyMMdd-HHmmss}-{seq}"
    )]
    template: String,
}

#[derive(Args)]
struct CaptureArgs {
    #[arg(
        short = 'd',
        long = "out-dir",
        default_value = ".",
        visible_alias = "out",
        short_alias = 'o'
    )]
    out_dir: PathBuf,
    #[arg(
        short = 't',
        long,
        default_value = "Screenshot-{date:yyyyMMdd-HHmmss}-{seq}"
    )]
    template: String,
    /// 捕获所有显示器
    #[arg(long)]
    all: bool,
}

#[derive(Args)]
struct CaptureRegionArgs {
    #[arg(
        short = 'd',
        long = "out-dir",
        default_value = ".",
        visible_alias = "out",
        short_alias = 'o'
    )]
    out_dir: PathBuf,
    #[arg(
        short = 't',
        long,
        default_value = "Screenshot-{date:yyyyMMdd-HHmmss}-{seq}"
    )]
    template: String,
    /// 区域：x,y,width,height
    #[arg(long, value_parser = parse_rect)]
    rect: (u32, u32, u32, u32),
}

fn parse_rect(s: &str) -> Result<(u32, u32, u32, u32), String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 4 {
        return Err("格式应为 x,y,width,height".to_string());
    }

    let x = parts[0].parse::<u32>().map_err(|_| "x 必须是数字")?;
    let y = parts[1].parse::<u32>().map_err(|_| "y 必须是数字")?;
    let w = parts[2].parse::<u32>().map_err(|_| "width 必须是数字")?;
    let h = parts[3].parse::<u32>().map_err(|_| "height 必须是数字")?;

    Ok((x, y, w, h))
}

fn main() {
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
            handle_interactive_capture(args);
        }
        Some(Commands::Capture(args)) => {
            handle_full_capture(args);
        }
        Some(Commands::CaptureRegion(args)) => {
            handle_region_capture(args);
        }
    }
}

fn handle_interactive_capture(args: CaptureInteractiveArgs) {
    #[cfg(target_os = "macos")]
    {
        let selector: Box<dyn ui_overlay::RegionSelector> =
            ui_overlay::create_gui_region_selector();

        match MacCapturer::capture_region_interactive_custom(selector.as_ref()) {
            Ok(shot) => {
                export_screenshot(shot, args.template, args.out_dir, "交互式截图");
            }
            Err(e) => {
                eprintln!("❌ 交互框选失败/取消: {e}");
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
                export_screenshot(shot, args.template, args.out_dir, "交互式截图");
            }
            Err(e) => {
                eprintln!("❌ 交互框选失败/取消: {e}");
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

fn handle_full_capture(args: CaptureArgs) {
    #[cfg(target_os = "macos")]
    {
        let result = if args.all {
            MacCapturer::capture_all()
        } else {
            MacCapturer::capture_full()
        };

        match result {
            Ok(shot) => {
                let desc = if args.all {
                    "全屏截图(所有显示器)"
                } else {
                    "全屏截图"
                };
                export_screenshot(shot, args.template, args.out_dir, desc);
            }
            Err(e) => {
                eprintln!("❌ 全屏截图失败: {e}");
                std::process::exit(2);
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        let result = if args.all {
            WinCapturer::capture_all()
        } else {
            WinCapturer::capture_full()
        };

        match result {
            Ok(shot) => {
                let desc = if args.all {
                    "全屏截图(所有显示器)"
                } else {
                    "全屏截图"
                };
                export_screenshot(shot, args.template, args.out_dir, desc);
            }
            Err(e) => {
                eprintln!("❌ 全屏截图失败: {e}");
                std::process::exit(2);
            }
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        eprintln!("❌ 当前平台暂不支持全屏截图");
        std::process::exit(3);
    }
}

fn handle_region_capture(args: CaptureRegionArgs) {
    let (x, y, w, h) = args.rect;

    #[cfg(target_os = "macos")]
    {
        match MacCapturer::capture_region(x, y, w, h) {
            Ok(shot) => {
                export_screenshot(shot, args.template, args.out_dir, "区域截图");
            }
            Err(e) => {
                eprintln!("❌ 区域截图失败: {e}");
                std::process::exit(2);
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        match WinCapturer::capture_region(x, y, w, h) {
            Ok(shot) => {
                export_screenshot(shot, args.template, args.out_dir, "区域截图");
            }
            Err(e) => {
                eprintln!("❌ 区域截图失败: {e}");
                std::process::exit(2);
            }
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        eprintln!("❌ 当前平台暂不支持区域截图");
        std::process::exit(3);
    }
}

fn export_screenshot(
    shot: screenshot_core::Screenshot,
    template: String,
    out_dir: PathBuf,
    desc: &str,
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

    if let Err(e) = export.export_png_to_file(&shot, &[], &out) {
        eprintln!("❌ {}导出失败: {e}", desc);
        std::process::exit(1);
    } else {
        println!("✅ {}已保存: {}", desc, out.display());
    }
}
