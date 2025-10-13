use chrono::Utc;
use image::GenericImageView;
use infra::{metrics, start_timer};
use parking_lot::Mutex;
use renderer::{ExportEncoder, PngEncoder, Renderer, SimpleRenderer};
use screenshot_core::{
    naming, undo, Annotation, HistoryItem, Result as CoreResult, Screenshot, UndoContext, UndoStack,
};
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

// 缩略图生成常量
const THUMBNAIL_MAX_SIZE: u32 = 240; // 缩略图最长边像素数

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
    history: Option<Arc<Mutex<HistoryService>>>,
}

// 手动实现Clone
impl<CP: Clipboard> Clone for ExportService<CP> {
    fn clone(&self) -> Self {
        Self {
            clipboard: self.clipboard.clone(),
            renderer: SimpleRenderer, // SimpleRenderer是零大小类型，直接创建新实例
            encoder: PngEncoder,      // PngEncoder是零大小类型，直接创建新实例
            history: self.history.clone(),
        }
    }
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

    pub fn with_history(mut self, history: Arc<Mutex<HistoryService>>) -> Self {
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

    /// 导出PNG到文件（同步版本）
    ///
    /// 注意：这个方法会阻塞当前线程。如果在异步上下文中，建议使用 `export_png_to_file_async`
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
                let mut history_lock = h.lock();
                let _ = history_lock.append(path.as_ref(), Some(thumb));
            }
        }
        Ok(())
    }

    /// 导出PNG到文件（异步版本）
    ///
    /// 性能优化：
    /// - 文件写入使用 tokio 异步I/O
    /// - 缩略图生成在 spawn_blocking 中执行
    pub async fn export_png_to_file_async<P: AsRef<Path> + Send>(
        &self,
        screenshot: &Screenshot,
        annotations: &[Annotation],
        path: P,
    ) -> anyhow::Result<()> {
        // 在当前线程渲染（CPU密集）
        let bytes = self.render_png_bytes(screenshot, annotations)?;

        // 异步写入文件
        let path_ref = path.as_ref();
        tokio::fs::write(path_ref, &bytes).await?;
        metrics::counter("export_png_file_ok").inc();

        // 异步处理历史记录和缩略图
        if let Some(h) = &self.history {
            let path_buf = path_ref.to_path_buf();
            let bytes_clone = bytes.clone();
            let history = h.clone();

            // 在后台线程生成缩略图并更新历史
            tokio::task::spawn_blocking(move || {
                if let Ok(thumb) = Self::generate_thumbnail_static(&bytes_clone) {
                    let mut history_lock = history.lock();
                    let _ = history_lock.append(&path_buf, Some(thumb));
                }
            });
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
        Self::generate_thumbnail_static(png_bytes)
    }

    /// 生成缩略图（静态方法，可在异步任务中使用）
    fn generate_thumbnail_static(png_bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
        let img = image::load_from_memory(png_bytes)?;
        let (w, h) = img.dimensions();
        let max_side = THUMBNAIL_MAX_SIZE;
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

    /// 从磁盘加载历史记录（同步版本）
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

    /// 从磁盘加载历史记录（异步版本）
    ///
    /// 性能优化：
    /// - 文件读取使用 tokio 异步I/O
    /// - JSON解析在 spawn_blocking 中执行
    pub async fn load_from_disk_async(&mut self) -> anyhow::Result<()> {
        let index = self.base_dir.join("history.jsonl");
        if !tokio::fs::try_exists(&index).await? {
            return Ok(());
        }

        // 异步读取文件
        let text = tokio::fs::read_to_string(&index).await?;
        let capacity = self.capacity;

        // 在 blocking pool 中解析 JSON（CPU密集）
        let items = tokio::task::spawn_blocking(move || {
            let mut items = Vec::new();
            for line in text.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<HistoryItem>(line) {
                    Ok(mut item) => {
                        if item.version == 0 {
                            item.version = 1;
                        }
                        items.push(item);
                    }
                    Err(_e) => { /* ignore broken line */ }
                }
            }

            // 裁剪
            if items.len() > capacity {
                items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                items.truncate(capacity);
            }

            items
        })
        .await?;

        self.items = items;
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

/// OCR 请求消息
pub struct OcrRequest {
    /// 图像的 PNG 字节
    pub image_bytes: Vec<u8>,
    /// 用于回传 OCR 结果的通道
    pub response_tx: tokio::sync::oneshot::Sender<CoreResult<Vec<String>>>,
}

/// OCR 服务（使用 tokio 异步模型）
///
/// 性能优化：
/// - 使用 tokio channel 替代 std::mpsc
/// - 使用 tokio task 替代 std::thread
/// - 保留并行处理能力
pub struct OcrService {
    tx: tokio::sync::mpsc::UnboundedSender<OcrRequest>,
}

impl OcrService {
    /// 创建新的OCR服务
    ///
    /// worker_threads: 并发处理的任务数（使用tokio任务，而非OS线程）
    pub fn new(worker_threads: usize) -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<OcrRequest>();
        let workers = worker_threads.max(1);

        // 使用Arc<Mutex>共享接收端
        let rx_shared = Arc::new(tokio::sync::Mutex::new(rx));

        // 启动worker任务池
        for _worker_id in 0..workers {
            let rx_clone = rx_shared.clone();
            tokio::spawn(async move {
                #[cfg(debug_assertions)]
                tracing::debug!("OCR worker {} 启动", _worker_id);

                loop {
                    let msg = {
                        let mut rx_guard = rx_clone.lock().await;
                        rx_guard.recv().await
                    };

                    match msg {
                        Some(OcrRequest {
                            image_bytes,
                            response_tx,
                        }) => {
                            // 在 blocking pool 中执行CPU密集型OCR操作
                            let result = tokio::task::spawn_blocking(move || {
                                // 占位实现：返回未实现错误
                                Err(screenshot_core::Error::new(
                                    screenshot_core::ErrorKind::Unsupported,
                                    format!("ocr not implemented ({} bytes)", image_bytes.len()),
                                ))
                            })
                            .await;

                            let ocr_result = match result {
                                Ok(r) => r,
                                Err(e) => Err(screenshot_core::Error::new(
                                    screenshot_core::ErrorKind::Unknown,
                                    format!("OCR task failed: {}", e),
                                )),
                            };

                            // 忽略发送失败（接收方可能已关闭）
                            let _ = response_tx.send(ocr_result);
                        }
                        None => break,
                    }
                }

                #[cfg(debug_assertions)]
                tracing::debug!("OCR worker {} 停止", _worker_id);
            });
        }

        Self { tx }
    }

    /// 异步识别图像中的文本
    ///
    /// 返回 oneshot receiver 用于接收OCR结果
    pub async fn recognize_async(
        &self,
        bytes: Vec<u8>,
    ) -> CoreResult<tokio::sync::oneshot::Receiver<CoreResult<Vec<String>>>> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        let req = OcrRequest {
            image_bytes: bytes,
            response_tx,
        };
        self.tx.send(req).map_err(|e| {
            screenshot_core::Error::new(screenshot_core::ErrorKind::Unknown, e.to_string())
        })?;
        Ok(response_rx)
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
        // 这些正则表达式是硬编码的，编译失败表明代码错误，应该 panic
        // 但使用 expect 提供更好的错误信息
        Self {
            email_re: regex::Regex::new(r#"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}"#)
                .expect("Invalid email regex pattern"),
            phone_re: regex::Regex::new(r#"\b\d{3}[- ]?\d{3,4}[- ]?\d{4}\b"#)
                .expect("Invalid phone regex pattern"),
            url_re: regex::Regex::new(r#"https?://[A-Za-z0-9._~:/?#%\\\[\\\]@!$&'()*+,;=-]+"#)
                .expect("Invalid URL regex pattern"),
            ipv4_re: regex::Regex::new(r#"\b((?:[0-9]{1,3}\.){3}[0-9]{1,3})\b"#)
                .expect("Invalid IPv4 regex pattern"),
            cn_mobile_re: regex::Regex::new(r#"\b1[3-9]\d{9}\b"#)
                .expect("Invalid CN mobile regex pattern"),
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

impl Default for PrivacyService {
    fn default() -> Self {
        Self::new()
    }
}
