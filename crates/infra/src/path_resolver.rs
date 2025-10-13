use directories::ProjectDirs;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Paths {
    pub base: PathBuf,
    pub config: PathBuf,
    pub history: PathBuf,
    pub logs: PathBuf,
}

pub fn resolve_paths() -> Paths {
    let base = if let Some(proj) = ProjectDirs::from("com", "Example", "ScreenshotTool") {
        proj.data_dir().to_path_buf()
    } else {
        // Fallback: 在无法获取标准路径时使用当前目录下的 .screenshot-tool
        let fallback = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".screenshot-tool");
        eprintln!(
            "Warning: Could not determine system data directory, using fallback: {:?}",
            fallback
        );
        fallback
    };
    Paths {
        base: base.clone(),
        config: base.join("config.json"),
        history: base.join("history"),
        logs: base.join("logs"),
    }
}

/// 确保必要目录存在（幂等）。
pub fn ensure_directories<P: AsRef<Path>>(dirs: &[P]) -> std::io::Result<()> {
    for d in dirs {
        fs::create_dir_all(d.as_ref())?;
    }
    Ok(())
}
