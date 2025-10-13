use crate::path_resolver::resolve_paths;
use serde::{Deserialize, Serialize};
use std::{fs, io};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UploadCfg {
    pub enabled: bool,
    pub endpoint: String,
    pub token: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrivacyCfg {
    pub enabled: bool,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OcrCfg {
    pub engine: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub version: u32,
    pub hotkeys: Hotkeys,
    pub default_save_path: String,
    pub naming_template: String,
    pub recent_colors: Vec<String>,
    pub upload: UploadCfg,
    pub privacy: PrivacyCfg,
    pub ocr: OcrCfg,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Hotkeys {
    pub capture_full: String,
    pub capture_window: String,
    pub capture_region: String,
    pub capture_delay: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            hotkeys: Hotkeys {
                capture_full: "Cmd+Shift+1".into(),
                capture_window: "Cmd+Shift+2".into(),
                capture_region: "Cmd+Shift+3".into(),
                capture_delay: "Cmd+Shift+4".into(),
            },
            default_save_path: "~/Pictures/Screenshots".into(),
            naming_template: "Screenshot-{date:yyyyMMdd-HHmmss}-{seq}".into(),
            recent_colors: vec![],
            upload: UploadCfg {
                enabled: false,
                endpoint: "".into(),
                token: "".into(),
            },
            privacy: PrivacyCfg { enabled: true },
            ocr: OcrCfg {
                engine: "tesseract".into(),
            },
        }
    }
}

pub fn load_config() -> io::Result<AppConfig> {
    let p = resolve_paths().config;
    match fs::read_to_string(&p) {
        Ok(s) => Ok(serde_json::from_str(&s).unwrap_or_default()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(AppConfig::default()),
        Err(e) => Err(e),
    }
}
pub fn save_config(cfg: &AppConfig) -> io::Result<()> {
    let p = resolve_paths().config;
    if let Some(dir) = p.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = p.with_extension("tmp");
    let json_bytes = serde_json::to_vec_pretty(cfg)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(&tmp, json_bytes)?;
    fs::rename(tmp, p)?;
    Ok(())
}
