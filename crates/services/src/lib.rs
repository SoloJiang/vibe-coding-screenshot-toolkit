use screenshot_core::{Annotation, Screenshot, UndoContext, UndoStack, undo};
use renderer::{Renderer, ExportEncoder, PngEncoder, SimpleRenderer};
use std::path::Path;
use std::sync::Arc;

pub trait Capturer: Send + Sync {
	fn capture_full(&self) -> anyhow::Result<Screenshot>;
}

pub struct MockCapturer;
impl Capturer for MockCapturer {
	fn capture_full(&self) -> anyhow::Result<Screenshot> { anyhow::bail!("not implemented mock image") }
}

pub struct CaptureService<C: Capturer> {
	capturer: Arc<C>,
}
impl<C: Capturer> CaptureService<C> {
	pub fn new(capturer: Arc<C>) -> Self { Self { capturer } }
	pub fn capture_full(&self) -> anyhow::Result<Screenshot> { self.capturer.capture_full() }
}

pub struct AnnotationService {
	pub annotations: Vec<Annotation>,
	pub undo: UndoStack,
}
impl AnnotationService {
	pub fn new() -> Self { Self { annotations: Vec::new(), undo: UndoStack::new(100) } }

	pub fn add(&mut self, ann: Annotation) {
		let ctx = UndoContext { annotations: self.annotations.clone() };
		self.annotations.push(ann);
		self.undo.push(undo::UndoOp {
			apply: Box::new(|_| {}),
			revert: Box::new(move |c: &mut UndoContext| {
				c.annotations = ctx.annotations.clone();
			}),
			merge_key: None,
		});
	}

	pub fn list(&self) -> &[Annotation] { &self.annotations }
}

pub trait Clipboard: Send + Sync {
	fn write_image(&self, _bytes: &[u8]) -> anyhow::Result<()> { Ok(()) }
}
pub struct StubClipboard;
impl Clipboard for StubClipboard {}

pub struct ExportService<CP: Clipboard> {
	clipboard: Arc<CP>,
	renderer: SimpleRenderer,
	encoder: PngEncoder,
}
impl<CP: Clipboard> ExportService<CP> {
	pub fn new(clipboard: Arc<CP>) -> Self { Self { clipboard, renderer: SimpleRenderer, encoder: PngEncoder } }

	pub fn render_png_bytes(&self, screenshot: &Screenshot, annotations: &[Annotation]) -> anyhow::Result<Vec<u8>> {
		let frame = &screenshot.raw.primary;
		let img = self.renderer.render(frame, annotations);
		self.encoder.encode_png(&img).map_err(|e| e.into())
	}

	pub fn export_png_to_clipboard(&self, screenshot: &Screenshot, annotations: &[Annotation]) -> anyhow::Result<()> {
		let bytes = self.render_png_bytes(screenshot, annotations)?;
		self.clipboard.write_image(&bytes)
	}

	pub fn export_png_to_file<P: AsRef<Path>>(&self, screenshot: &Screenshot, annotations: &[Annotation], path: P) -> anyhow::Result<()> {
		let bytes = self.render_png_bytes(screenshot, annotations)?;
		std::fs::write(&path, &bytes)?;
		Ok(())
	}

	pub fn render_jpeg_bytes(&self, screenshot: &Screenshot, annotations: &[Annotation], quality: u8) -> anyhow::Result<Vec<u8>> {
		let frame = &screenshot.raw.primary;
		let img = self.renderer.render(frame, annotations);
		self.encoder.encode_jpeg(&img, quality).map_err(|e| e.into())
	}

	pub fn export_jpeg_to_file<P: AsRef<Path>>(&self, screenshot: &Screenshot, annotations: &[Annotation], path: P, quality: u8) -> anyhow::Result<()> {
		let bytes = self.render_jpeg_bytes(screenshot, annotations, quality)?;
		std::fs::write(&path, &bytes)?;
		Ok(())
	}
}
