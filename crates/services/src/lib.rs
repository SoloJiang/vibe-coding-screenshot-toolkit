use chrono::Utc;
use image::GenericImageView;
use infra::{metrics, start_timer};
use renderer::{ExportEncoder, PngEncoder, Renderer, SimpleRenderer};
use screenshot_core::{
    naming, undo, Annotation, HistoryItem, Result as CoreResult, Screenshot, UndoContext, UndoStack,
};
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

pub trait Capturer: Send + Sync {
    fn capture_full(&self) -> anyhow::Result<Screenshot>;
}

pub struct MockCapturer;
impl Capturer for MockCapturer {
    fn capture_full(&self) -> anyhow::Result<Screenshot> {
        anyhow::bail!("not implemented mock image")
    }
}

pub struct CaptureService<C: Capturer> {
    capturer: Arc<C>,
}
impl<C: Capturer> CaptureService<C> {
    pub fn new(capturer: Arc<C>) -> Self {
        Self { capturer }
    }
    pub fn capture_full(&self) -> anyhow::Result<Screenshot> {
        let _t = start_timer(
            "capture_full_us",
            &[100, 500, 1_000, 5_000, 20_000, 100_000],
        );
        let r = self.capturer.capture_full();
        if r.is_ok() {
            metrics::counter("capture_full_ok").inc();
        } else {
            metrics::counter("capture_full_err").inc();
        }
        r
    }
}

pub struct AnnotationService {
    pub annotations: Vec<Annotation>,
    pub undo: UndoStack,
}
impl AnnotationService {
    pub fn new() -> Self {
        Self {
            annotations: Vec::new(),
            undo: UndoStack::new(100),
        }
    }

    pub fn add(&mut self, ann: Annotation) {
        let ctx = UndoContext {
            annotations: self.annotations.clone(),
        };
        self.annotations.push(ann.clone());
        self.undo.push(undo::UndoOp {
            apply: Box::new(move |c: &mut UndoContext| {
                c.annotations.push(ann.clone());
            }),
            revert: Box::new(move |c: &mut UndoContext| {
                c.annotations = ctx.annotations.clone();
            }),
            merge_key: None,
        });
    }

    pub fn undo(&mut self) -> bool {
        let mut ctx = UndoContext {
            annotations: self.annotations.clone(),
        };
        if self.undo.undo(&mut ctx) {
            self.annotations = ctx.annotations;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        let mut ctx = UndoContext {
            annotations: self.annotations.clone(),
        };
        if self.undo.redo(&mut ctx) {
            self.annotations = ctx.annotations;
            true
        } else {
            false
        }
    }

    /// 根据 id 修改注解：
    /// 1. 闭包 f 返回 true 表示有实际修改 -> 记录 undo；false 则忽略。
    /// 2. merge_key 用于合并连续操作（例如拖拽）。
    pub fn update<F: FnOnce(&mut Annotation) -> bool>(
        &mut self,
        id: Uuid,
        merge_key: Option<&str>,
        f: F,
    ) {
        let idx = match self.annotations.iter().position(|a| a.meta.id == id) {
            Some(i) => i,
            None => return,
        };
        let before = self.annotations[idx].clone();
        let changed = {
            let ann = &mut self.annotations[idx];
            f(ann)
        };
        if !changed {
            return;
        }
        let after = self.annotations[idx].clone();
        self.undo.push(undo::UndoOp {
            apply: Box::new(move |c: &mut UndoContext| {
                if let Some(a) = c
                    .annotations
                    .iter_mut()
                    .find(|a| a.meta.id == after.meta.id)
                {
                    *a = after.clone();
                }
            }),
            revert: Box::new(move |c: &mut UndoContext| {
                if let Some(a) = c
                    .annotations
                    .iter_mut()
                    .find(|a| a.meta.id == before.meta.id)
                {
                    *a = before.clone();
                }
            }),
            merge_key: merge_key.map(|s| s.to_string()),
        });
    }

    /// 上移一层 (z++ 简单实现)
    pub fn move_up(&mut self, id: Uuid) {
        if let Some(idx) = self.annotations.iter().position(|a| a.meta.id == id) {
            let before = self.annotations[idx].meta.z;
            self.annotations[idx].meta.z += 1;
            let after = self.annotations[idx].meta.z;
            self.undo.push(undo::UndoOp {
                apply: Box::new(move |c: &mut UndoContext| {
                    if let Some(a) = c.annotations.iter_mut().find(|a| a.meta.id == id) {
                        a.meta.z = after;
                    }
                }),
                revert: Box::new(move |c: &mut UndoContext| {
                    if let Some(a) = c.annotations.iter_mut().find(|a| a.meta.id == id) {
                        a.meta.z = before;
                    }
                }),
                merge_key: None,
            });
        }
    }

    /// 下移一层 (z--)
    pub fn move_down(&mut self, id: Uuid) {
        if let Some(idx) = self.annotations.iter().position(|a| a.meta.id == id) {
            let before = self.annotations[idx].meta.z;
            self.annotations[idx].meta.z -= 1;
            let after = self.annotations[idx].meta.z;
            self.undo.push(undo::UndoOp {
                apply: Box::new(move |c: &mut UndoContext| {
                    if let Some(a) = c.annotations.iter_mut().find(|a| a.meta.id == id) {
                        a.meta.z = after;
                    }
                }),
                revert: Box::new(move |c: &mut UndoContext| {
                    if let Some(a) = c.annotations.iter_mut().find(|a| a.meta.id == id) {
                        a.meta.z = before;
                    }
                }),
                merge_key: None,
            });
        }
    }

    pub fn list(&self) -> &[Annotation] {
        &self.annotations
    }
}

impl Default for AnnotationService {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Clipboard: Send + Sync {
    fn write_image(&self, _bytes: &[u8]) -> CoreResult<()> {
        Ok(())
    }
}
pub struct StubClipboard;
impl Clipboard for StubClipboard {}

pub struct ExportService<CP: Clipboard> {
    clipboard: Arc<CP>,
    renderer: SimpleRenderer,
    encoder: PngEncoder,
    history: Option<Arc<std::sync::Mutex<HistoryService>>>,
}
impl<CP: Clipboard> ExportService<CP> {
    pub fn new(clipboard: Arc<CP>) -> Self {
        Self {
            clipboard,
            renderer: SimpleRenderer,
            encoder: PngEncoder,
            history: None,
        }
    }

    pub fn with_history(mut self, history: Arc<std::sync::Mutex<HistoryService>>) -> Self {
        self.history = Some(history);
        self
    }

    pub fn render_png_bytes(
        &self,
        screenshot: &Screenshot,
        annotations: &[Annotation],
    ) -> anyhow::Result<Vec<u8>> {
        let _t = start_timer("render_png_us", &[100, 500, 1_000, 5_000, 20_000, 100_000]);
        let frame = &screenshot.raw.primary;
        let img = self.renderer.render(frame, annotations);
        let r = self.encoder.encode_png(&img);
        if r.is_ok() {
            metrics::counter("render_png_ok").inc();
        } else {
            metrics::counter("render_png_err").inc();
        }
        r
    }

    pub fn export_png_to_clipboard(
        &self,
        screenshot: &Screenshot,
        annotations: &[Annotation],
    ) -> anyhow::Result<()> {
        metrics::counter("clipboard_write_attempt").inc();
        let bytes = self.render_png_bytes(screenshot, annotations)?;
        match self.clipboard.write_image(&bytes) {
            Ok(_) => {
                metrics::counter("clipboard_write_success_first").inc();
                Ok(())
            }
            Err(e1) => {
                metrics::counter("clipboard_write_retry").inc();
                // 重试一次
                match self.clipboard.write_image(&bytes) {
                    Ok(_) => {
                        metrics::counter("clipboard_write_retry_success").inc();
                        Ok(())
                    }
                    Err(e2) => {
                        metrics::counter("clipboard_write_retry_fail").inc();
                        Err(anyhow::anyhow!(format!(
                            "clipboard write failed twice: first={:?} second={:?}",
                            e1, e2
                        )))
                    }
                }
            }
        }
    }

    pub fn export_png_to_file<P: AsRef<Path>>(
        &self,
        screenshot: &Screenshot,
        annotations: &[Annotation],
        path: P,
    ) -> anyhow::Result<()> {
        let bytes = self.render_png_bytes(screenshot, annotations)?;
        let write_res = std::fs::write(&path, &bytes);
        if write_res.is_ok() {
            metrics::counter("export_png_file_ok").inc();
        } else {
            metrics::counter("export_png_file_err").inc();
        }
        write_res?;
        if let Some(h) = &self.history {
            // 生成缩略图（最长边 240）
            if let Ok(thumb) = self.generate_thumbnail(&bytes) {
                let mut history_lock = h.lock().unwrap();
                let _ = history_lock.append(path.as_ref(), Some(thumb));
            }
        }
        Ok(())
    }

    pub fn render_jpeg_bytes(
        &self,
        screenshot: &Screenshot,
        annotations: &[Annotation],
        quality: u8,
    ) -> anyhow::Result<Vec<u8>> {
        let frame = &screenshot.raw.primary;
        let _t = start_timer("render_jpeg_us", &[100, 500, 1_000, 5_000, 20_000, 100_000]);
        let img = self.renderer.render(frame, annotations);
        let r = self.encoder.encode_jpeg(&img, quality);
        if r.is_ok() {
            metrics::counter("render_jpeg_ok").inc();
        } else {
            metrics::counter("render_jpeg_err").inc();
        }
        r
    }

    pub fn export_jpeg_to_file<P: AsRef<Path>>(
        &self,
        screenshot: &Screenshot,
        annotations: &[Annotation],
        path: P,
        quality: u8,
    ) -> anyhow::Result<()> {
        let bytes = self.render_jpeg_bytes(screenshot, annotations, quality)?;
        let write_res = std::fs::write(&path, &bytes);
        if write_res.is_ok() {
            metrics::counter("export_jpeg_file_ok").inc();
        } else {
            metrics::counter("export_jpeg_file_err").inc();
        }
        write_res?;
        Ok(())
    }
}

impl<CP: Clipboard> ExportService<CP> {
    fn generate_thumbnail(&self, png_bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
        let img = image::load_from_memory(png_bytes)?; // DynamicImage
        let (w, h) = img.dimensions();
        let max_side = 240u32;
        let scale = (max_side as f32 / w.max(h) as f32).min(1.0);
        let nw = (w as f32 * scale).round() as u32;
        let nh = (h as f32 * scale).round() as u32;
        let resized = if scale < 1.0 {
            img.resize_exact(nw, nh, image::imageops::FilterType::Triangle)
        } else {
            img
        };
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, resized.width(), resized.height());
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut writer = enc.write_header()?;
            let rgba = resized.to_rgba8();
            writer.write_image_data(&rgba)?;
        }
        Ok(out)
    }
}

pub struct HistoryService {
    items: Vec<HistoryItem>,
    capacity: usize,
    base_dir: PathBuf,
}

impl HistoryService {
    pub fn new<P: AsRef<Path>>(base: P, capacity: usize) -> anyhow::Result<Self> {
        let dir = base.as_ref().to_path_buf();
        create_dir_all(&dir)?;
        Ok(Self {
            items: Vec::new(),
            capacity,
            base_dir: dir,
        })
    }

    pub fn load_from_disk(&mut self) -> anyhow::Result<()> {
        let index = self.base_dir.join("history.jsonl");
        if !index.exists() {
            return Ok(());
        }
        let text = std::fs::read_to_string(&index)?;
        self.items.clear();
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<HistoryItem>(line) {
                Ok(mut item) => {
                    if item.version == 0 {
                        item.version = 1;
                    }
                    self.items.push(item);
                }
                Err(_e) => { /* ignore broken line */ }
            }
        }
        // 裁剪
        if self.items.len() > self.capacity {
            self.items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            self.items.truncate(self.capacity);
        }
        Ok(())
    }

    pub fn append(&mut self, path: &Path, thumb: Option<Vec<u8>>) -> anyhow::Result<()> {
        let item = HistoryItem {
            id: Uuid::now_v7(),
            path: path.to_string_lossy().into_owned(),
            thumb,
            created_at: Utc::now(),
            title: None,
            version: 1,
        };
        screenshot_core::model::push_history_trim(&mut self.items, item.clone(), self.capacity);
        let mut f = File::options()
            .create(true)
            .append(true)
            .open(self.base_dir.join("history.jsonl"))?;
        let line = serde_json::to_string(&item)?;
        f.write_all(line.as_bytes())?;
        f.write_all(b"\n")?;
        Ok(())
    }

    pub fn list(&self) -> &[HistoryItem] {
        &self.items
    }
}

/// 依据模板生成文件名（不含扩展名）。模板支持 {date},{seq},{screen}
pub fn gen_file_name(template: &str, screen_index: usize) -> String {
    let now = Utc::now();
    naming::parse_naming_template(template, screen_index, now)
}

// ---- Future Expansion Stubs (OCR / Privacy) ----
/// 占位：OCR 服务（后续实现线程池 + tesseract 适配器）
pub struct OcrService {
    // 简单线程执行器 (占位)：mpsc 任务队列 + 工作线程
    tx: std::sync::mpsc::Sender<(Vec<u8>, std::sync::mpsc::Sender<CoreResult<Vec<String>>>)>,
}
impl OcrService {
    pub fn new(worker_threads: usize) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<(
            Vec<u8>,
            std::sync::mpsc::Sender<CoreResult<Vec<String>>>,
        )>();
        let shared = std::sync::Arc::new(std::sync::Mutex::new(rx));
        let threads = worker_threads.max(1);
        for _ in 0..threads {
            let shared_rx = shared.clone();
            std::thread::spawn(move || loop {
                let msg = {
                    let guard = shared_rx.lock().unwrap();
                    guard.recv()
                };
                match msg {
                    Ok((data, reply_tx)) => {
                        let _ = reply_tx.send(Err(screenshot_core::Error::new(
                            screenshot_core::ErrorKind::Unsupported,
                            format!("ocr not implemented ({} bytes)", data.len()),
                        )));
                    }
                    Err(_) => break,
                }
            });
        }
        Self { tx }
    }

    pub fn recognize_async(
        &self,
        bytes: Vec<u8>,
    ) -> CoreResult<std::sync::mpsc::Receiver<CoreResult<Vec<String>>>> {
        let (rtx, rrx) = std::sync::mpsc::channel();
        self.tx.send((bytes, rtx)).map_err(|e| {
            screenshot_core::Error::new(screenshot_core::ErrorKind::Unknown, e.to_string())
        })?;
        Ok(rrx)
    }
}

/// 占位：隐私扫描服务（检测敏感文本并返回命中区域）
pub struct PrivacyService {
    email_re: regex::Regex,
    phone_re: regex::Regex,
    url_re: regex::Regex,
    ipv4_re: regex::Regex,
    cn_mobile_re: regex::Regex,
}
impl PrivacyService {
    pub fn new() -> Self {
        Self {
            email_re: regex::Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").unwrap(),
            phone_re: regex::Regex::new(r"\b\d{3}[- ]?\d{3,4}[- ]?\d{4}\b").unwrap(),
            url_re: regex::Regex::new(r"https?://[A-Za-z0-9._~:/?#%[\\]@!$&'()*+,;=-]+").unwrap(),
            ipv4_re: regex::Regex::new(r"\b((?:[0-9]{1,3}\.){3}[0-9]{1,3})\b").unwrap(),
            cn_mobile_re: regex::Regex::new(r"\b1[3-9]\\d{9}\b").unwrap(),
        }
    }
    /// 返回命中区间 (start,end)
    pub fn scan(&self, text: &str) -> CoreResult<Vec<(usize, usize)>> {
        let mut hits = Vec::new();
        for m in self.email_re.find_iter(text) {
            hits.push((m.start(), m.end()));
        }
        for m in self.phone_re.find_iter(text) {
            hits.push((m.start(), m.end()));
        }
        for m in self.url_re.find_iter(text) {
            hits.push((m.start(), m.end()));
        }
        for m in self.ipv4_re.find_iter(text) {
            hits.push((m.start(), m.end()));
        }
        for m in self.cn_mobile_re.find_iter(text) {
            hits.push((m.start(), m.end()));
        }
        Ok(hits)
    }

    /// 调试：返回 (start,end,kind)
    pub fn scan_detailed(&self, text: &str) -> CoreResult<Vec<(usize, usize, &'static str)>> {
        let mut hits = Vec::new();
        for m in self.email_re.find_iter(text) {
            hits.push((m.start(), m.end(), "email"));
        }
        for m in self.phone_re.find_iter(text) {
            hits.push((m.start(), m.end(), "phone"));
        }
        for m in self.url_re.find_iter(text) {
            hits.push((m.start(), m.end(), "url"));
        }
        for m in self.ipv4_re.find_iter(text) {
            hits.push((m.start(), m.end(), "ipv4"));
        }
        for m in self.cn_mobile_re.find_iter(text) {
            hits.push((m.start(), m.end(), "cn_mobile"));
        }
        Ok(hits)
    }

    /// 简单 mask：返回将命中区域替换为同长度 * 的字符串（不改变长度）。
    pub fn mask(&self, text: &str) -> CoreResult<String> {
        let mut chars: Vec<char> = text.chars().collect();
        let hits = self.scan(text)?;
        for (s, e) in hits {
            for i in s..e {
                if i < chars.len() {
                    chars[i] = '*';
                }
            }
        }
        Ok(chars.into_iter().collect())
    }
}
