use directories::ProjectDirs;
use std::path::PathBuf;

pub struct Paths {
    pub base: PathBuf,
    pub config: PathBuf,
    pub history: PathBuf,
    pub logs: PathBuf,
}

pub fn resolve_paths() -> Paths {
    let proj = ProjectDirs::from("com", "Example", "ScreenshotTool").expect("project dirs");
    let base = proj.data_dir().to_path_buf();
    Paths {
        base: base.clone(),
        config: base.join("config.json"),
        history: base.join("history"),
        logs: base.join("logs"),
    }
}
