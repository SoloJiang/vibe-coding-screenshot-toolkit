use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use infra::metrics;
use infra::path_resolver::ensure_directories;
#[cfg(target_os = "macos")]
use platform_mac::MacCapturer;
use screenshot_core::{Frame, FrameSet, PixelFormat, Screenshot};
#[cfg(not(target_os = "macos"))]
use services::StubClipboard;
use services::{gen_file_name, ExportService, HistoryService};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Parser)]
#[command(author, version, about = "Screenshot Tool Dev CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Version,
    /// 全屏截图并保存
    Capture(CaptureArgs),
    /// 区域截图 (基于 mock 数据裁剪)
    CaptureRegion(CaptureRegionArgs),
    /// 输出当前进程 metrics 文本
    Metrics,
    /// 自研框选 UI 截图（原生选择器）
    CaptureInteractive(CaptureInteractiveArgs),
}

#[derive(Args)]
struct CaptureArgs {
    /// 输出目录 (-d / -o 兼容)
    #[arg(
        short = 'd',
        long = "out-dir",
        default_value = ".",
        visible_alias = "out",
        short_alias = 'o'
    )]
    out_dir: PathBuf,
    /// 命名模板（不含扩展名）
    #[arg(
        short = 't',
        long,
        default_value = "Screenshot-{date:yyyyMMdd-HHmmss}-{seq}"
    )]
    template: String,
    /// 是否导出所有显示器 (macOS)
    #[arg(long, default_value_t = false)]
    all: bool,
    /// 使用内置 mock 灰底图 (跳过真实捕获)
    #[arg(long, default_value_t = false)]
    mock: bool,
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
    /// x,y,w,h
    #[arg(long)]
    rect: String,
    /// 使用内置 mock 灰底图 (跳过真实捕获)
    #[arg(long, default_value_t = false)]
    mock: bool,
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
    /// 选择器类型：native(原生), gui(纯GUI)
    #[arg(
        long,
        default_value = "native",
        value_parser = ["native", "gui"]
    )]
    selector: String,
}

fn build_mock_screenshot() -> Screenshot {
    // 生成简单纯灰底图 (800x600)
    let w = 800u32;
    let h = 600u32;
    let mut bytes = vec![0u8; (w * h * 4) as usize];
    for p in bytes.chunks_exact_mut(4) {
        p[0] = 200;
        p[1] = 200;
        p[2] = 200;
        p[3] = 255;
    }
    let frame = Frame {
        width: w,
        height: h,
        pixel_format: PixelFormat::Rgba8,
        bytes: Arc::from(bytes.into_boxed_slice()),
    };
    let fs = FrameSet {
        primary: frame.clone(),
        all: vec![frame],
    };
    Screenshot {
        id: Uuid::now_v7(),
        raw: Arc::new(fs),
        scale: 1.0,
        created_at: Utc::now(),
    }
}

fn crop_screenshot(src: &Screenshot, x: u32, y: u32, w: u32, h: u32) -> Screenshot {
    let src_frame = &src.raw.primary;
    let w0 = src_frame.width;
    let h0 = src_frame.height;
    let x2 = (x + w).min(w0);
    let y2 = (y + h).min(h0);
    let w = x2 - x;
    let h = y2 - y;
    let mut bytes = vec![0u8; (w * h * 4) as usize];
    for row in 0..h {
        for col in 0..w {
            let src_i = (((y + row) * w0 + (x + col)) * 4) as usize;
            let dst_i = ((row * w + col) * 4) as usize;
            bytes[dst_i..dst_i + 4].copy_from_slice(&src_frame.bytes[src_i..src_i + 4]);
        }
    }
    let frame = Frame {
        width: w,
        height: h,
        pixel_format: PixelFormat::Rgba8,
        bytes: Arc::from(bytes.into_boxed_slice()),
    };
    let fs = FrameSet {
        primary: frame.clone(),
        all: vec![frame],
    };
    Screenshot {
        id: Uuid::now_v7(),
        raw: Arc::new(fs),
        scale: 1.0,
        created_at: Utc::now(),
    }
}

/// 初始化历史服务和序列号
fn init_history_and_sequence(out_dir: &std::path::Path) -> (Arc<std::sync::Mutex<HistoryService>>, std::path::PathBuf) {
    let hist_dir = out_dir.join(".history");
    let _ = ensure_directories(&[out_dir, &hist_dir]);
    let history = Arc::new(std::sync::Mutex::new(
        HistoryService::new(&hist_dir, 50).expect("history init"),
    ));
    {
        let mut h = history.lock().unwrap();
        let _ = h.load_from_disk();
    }

    // 读取序列 (seq.txt: YYYYMMDD last_value)
    let seq_file = hist_dir.join("seq.txt");
    if let Ok(txt) = std::fs::read_to_string(&seq_file) {
        let parts: Vec<&str> = txt.trim().split_whitespace().collect();
        if parts.len() == 2 {
            if let Ok(v) = parts[1].parse::<u32>() {
                screenshot_core::naming::set_sequence_for(parts[0], v);
            }
        }
    }

    (history, seq_file)
}

/// 写回序列号到文件
fn save_sequence(seq_file: &std::path::Path) {
    if let Some(parent) = seq_file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let day = chrono::Utc::now().format("%Y%m%d").to_string();
    let curr = screenshot_core::naming::current_sequence();
    let _ = std::fs::write(seq_file, format!("{} {}\n", day, curr));
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Version) | None => {
            println!("version: {}", env!("CARGO_PKG_VERSION"));
        }
        Some(Commands::Capture(args)) => {
            #[cfg(target_os = "macos")]
            {
                // 初始化历史和序列号
                let (history, seq_file) = init_history_and_sequence(&args.out_dir);

                #[cfg(target_os = "macos")]
                let export = {
                    use platform_mac::MacClipboard;
                    ExportService::new(Arc::new(MacClipboard)).with_history(history.clone())
                };
                #[cfg(not(target_os = "macos"))]
                let export =
                    ExportService::new(Arc::new(StubClipboard)).with_history(history.clone());
                let export_one = |shot: &Screenshot, idx: usize| {
                    let fname = gen_file_name(&args.template, idx) + ".png";
                    let path = args.out_dir.join(&fname);
                    match export.export_png_to_file(shot, &[], &path) {
                        Ok(_) => println!("{}", path.display()),
                        Err(e) => eprintln!("导出 {fname} 失败: {e}"),
                    }
                };
                if args.mock {
                    let shot = build_mock_screenshot();
                    export_one(&shot, 0);
                } else if args.all {
                    match MacCapturer::capture_all() {
                        Ok(list) => {
                            for (i, shot) in list.iter().enumerate() {
                                export_one(shot, i);
                            }
                        }
                        Err(e) => {
                            eprintln!("多显示器捕获失败，回退单屏: {e}");
                            let shot = MacCapturer::capture_full().unwrap_or_else(|e2| {
                                if format!("{e2}").contains("permission") {
                                    eprintln!("可能缺少屏幕录制权限，请在 系统设置->隐私与安全性->屏幕录制 授权终端");
                                }
                                eprintln!("单屏也失败, 使用 mock: {e2}");
                                build_mock_screenshot()
                            });
                            export_one(&shot, 0);
                        }
                    }
                } else {
                    let shot = MacCapturer::capture_full().unwrap_or_else(|e| {
                        if format!("{e}").contains("permission") {
                            eprintln!("可能缺少屏幕录制权限，请在 系统设置->隐私与安全性->屏幕录制 授权终端");
                        }
                        eprintln!("mac 捕获失败, 使用 mock: {e}");
                        build_mock_screenshot()
                    });
                    export_one(&shot, 0);
                }
                // 写回序列
                save_sequence(&seq_file);
            }
            #[cfg(not(target_os = "macos"))]
            {
                // 初始化历史和序列号
                let (history, seq_file) = init_history_and_sequence(&args.out_dir);

                let shot = build_mock_screenshot();
                let fname = gen_file_name(&args.template, 0) + ".png";
                let path = args.out_dir.join(fname);
                let export = ExportService::new(Arc::new(StubClipboard)).with_history(history.clone());
                export
                    .export_png_to_file(&shot, &[], &path)
                    .expect("export");

                // 写回序列
                save_sequence(&seq_file);

                println!("{}", path.display());
            }
        }
        Some(Commands::CaptureRegion(args)) => {
            let base = if args.mock {
                build_mock_screenshot()
            } else {
                #[cfg(target_os = "macos")]
                {
                    match MacCapturer::capture_full() {
                        Ok(s) => s,
                        Err(e) => {
                            if format!("{e}").contains("permission") {
                                eprintln!("可能缺少屏幕录制权限，请在 系统设置->隐私与安全性->屏幕录制 授权终端");
                            }
                            eprintln!("mac 捕获失败, 使用 mock: {e}");
                            build_mock_screenshot()
                        }
                    }
                }
                #[cfg(not(target_os = "macos"))]
                {
                    build_mock_screenshot()
                }
            };
            let parts: Vec<u32> = args
                .rect
                .split(',')
                .filter_map(|s| s.parse().ok())
                .collect();
            if parts.len() != 4 {
                eprintln!("--rect 需要 x,y,w,h 四个整数");
                std::process::exit(1);
            }
            if parts[2] == 0 || parts[3] == 0 {
                eprintln!("裁剪宽高必须 > 0");
                std::process::exit(1);
            }
            #[cfg(target_os = "macos")]
            let sub = match platform_mac::MacCapturer::capture_region(
                parts[0], parts[1], parts[2], parts[3],
            ) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("原生区域捕获失败，退回内存裁剪: {e}");
                    crop_screenshot(&base, parts[0], parts[1], parts[2], parts[3])
                }
            };
            #[cfg(not(target_os = "macos"))]
            let sub = crop_screenshot(&base, parts[0], parts[1], parts[2], parts[3]);
            let fname = gen_file_name(&args.template, 0) + ".png";
            let path = args.out_dir.join(fname);
            // 初始化历史和序列号
            let (history, seq_file) = init_history_and_sequence(&args.out_dir);

            #[cfg(target_os = "macos")]
            let export = {
                use platform_mac::MacClipboard;
                ExportService::new(Arc::new(MacClipboard)).with_history(history.clone())
            };
            #[cfg(not(target_os = "macos"))]
            let export = ExportService::new(Arc::new(StubClipboard)).with_history(history.clone());
            export.export_png_to_file(&sub, &[], &path).expect("export");

            // 写回序列
            save_sequence(&seq_file);
            println!("{}", path.display());
        }
        Some(Commands::Metrics) => {
            let text = metrics::export();
            println!("{}", text);
        }
        Some(Commands::CaptureInteractive(args)) => {
            #[cfg(target_os = "macos")]
            {
                use platform_mac::MacCapturer;

                // 根据参数选择不同的选择器
                let selector: Box<dyn ui_overlay::RegionSelector> = match args.selector.as_str() {
                    "native" => {
                        println!("🔧 使用增强原生选择器 (macOS screencapture)");
                        ui_overlay::create_enhanced_native_selector()
                    }
                    "gui" => {
                        println!("🎨 使用纯 GUI 选择器 (Iced 界面)");
                        ui_overlay::create_gui_region_selector()
                    }
                    _ => {
                        eprintln!(
                            "❌ 未知选择器类型: {}，支持的类型: native, gui",
                            args.selector
                        );
                        std::process::exit(1);
                    }
                };

                match MacCapturer::capture_region_interactive_custom(selector.as_ref()) {
                    Ok(shot) => {
                        // 导出文件
                        let filename = gen_file_name(&args.template, 1);
                        let out = args.out_dir.join(format!("{}.png", filename));

                        let export = {
                            use platform_mac::MacClipboard;
                            ExportService::new(Arc::new(MacClipboard))
                        };

                        if let Err(e) = export.export_png_to_file(&shot, &[], &out) {
                            eprintln!("❌ 导出失败: {e}");
                        } else {
                            println!("✅ 截图已保存: {}", out.display());
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ 交互框选失败/取消: {e}");
                        std::process::exit(3);
                    }
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                eprintln!("❌ 当前平台未实现自研框选");
                std::process::exit(2);
            }
        }
    }
}
