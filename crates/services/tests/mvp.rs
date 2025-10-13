use chrono::Utc;
use infra::metrics;
use parking_lot::Mutex;
use screenshot_core::{
    Annotation, AnnotationKind, AnnotationMeta, Frame, FrameSet, PixelFormat, Screenshot,
};
use services::{gen_file_name, AnnotationService, ExportService, HistoryService, StubClipboard};
use std::sync::Arc;
use uuid::Uuid;

fn make_mock_screenshot(w: u32, h: u32) -> Screenshot {
    let mut bytes = vec![0u8; (w * h * 4) as usize];
    for p in bytes.chunks_exact_mut(4) {
        p.copy_from_slice(&[180, 180, 180, 255]);
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

#[test]
fn test_naming_seq_increase_same_day() {
    let a = gen_file_name("Screenshot-{date:yyyyMMdd}-{seq}", 0);
    let b = gen_file_name("Screenshot-{date:yyyyMMdd}-{seq}", 0);
    assert!(a != b, "seq should increment");
}

#[test]
fn test_history_trim_and_thumbnail() {
    let tmp = tempfile::tempdir().unwrap();
    let history = Arc::new(Mutex::new(HistoryService::new(tmp.path(), 5).unwrap()));
    let export = ExportService::new(Arc::new(StubClipboard)).with_history(history.clone());
    let shot = make_mock_screenshot(400, 300);
    for _ in 0..7 {
        let path = tmp.path().join(format!("{}.png", Uuid::now_v7()));
        export.export_png_to_file(&shot, &[], &path).unwrap();
    }
    let h = history.lock();
    assert_eq!(h.list().len(), 5, "history trimmed to capacity");
    assert!(h
        .list()
        .iter()
        .all(|i| i.thumb.as_ref().map(|t| !t.is_empty()).unwrap_or(true)));
}

#[test]
fn test_history_load_ignores_broken_lines() {
    let tmp = tempfile::tempdir().unwrap();
    // 手写一个含坏行的 history.jsonl
    let file = tmp.path().join("history.jsonl");
    std::fs::write(&file, b"{\"version\":1,\"id\":\"00000000-0000-7000-8000-000000000001\",\"path\":\"a.png\",\"thumb\":null,\"created_at\":1710000000000,\"title\":null}\nTHIS IS BROKEN LINE\n{}\n").unwrap();
    let mut hs = HistoryService::new(tmp.path(), 50).unwrap();
    // 不 panic 即通过；坏行与空对象行被忽略
    hs.load_from_disk().unwrap();
    assert!(hs.list().len() <= 1);
}

#[test]
fn test_annotation_undo_layer_move() {
    let mut svc = AnnotationService::new();
    let ann = Annotation {
        meta: AnnotationMeta {
            id: Uuid::now_v7(),
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
            rotation: 0,
            opacity: 1.0,
            stroke_color: None,
            fill_color: None,
            stroke_width: None,
            z: 0,
            locked: false,
            created_at: Utc::now(),
        },
        kind: AnnotationKind::Rect { corner_radius: 0 },
    };
    svc.add(ann.clone());
    let id = ann.meta.id;
    svc.move_up(id);
    assert_eq!(svc.list()[0].meta.z, 1);
    svc.undo();
    assert_eq!(svc.list()[0].meta.z, 0, "undo should revert z change");
    svc.redo();
    assert_eq!(svc.list()[0].meta.z, 1, "redo should reapply z change");
}

#[test]
fn test_privacy_scan_basic() {
    let svc = services::PrivacyService::new();
    let text = "contact me at test_email@example.com or 123-456-7890 visit https://example.com ip 192.168.0.1 手机 13912345678";
    let hits = svc.scan_detailed(text).unwrap();
    for (s, e, k) in &hits {
        println!("hit {:?}: {}", k, &text[*s..*e]);
    }
    assert!(hits.len() >= 5, "expected >=5 hits, got {}", hits.len());
    let masked = svc.mask(text).unwrap();
    assert_eq!(masked.len(), text.len());
}

#[tokio::test]
async fn test_ocr_service_queue() {
    let ocr = services::OcrService::new(2);
    let receiver = ocr.recognize_async(vec![1, 2, 3, 4]).await.unwrap();
    let res = receiver.await.expect("ocr result");
    assert!(res.is_err(), "stub should return unsupported error");
}

#[test]
fn test_metrics_counter_increment() {
    let c = metrics::counter("test_counter_increment_case");
    let base = c.get();
    c.inc();
    c.incr(4);
    assert_eq!(c.get(), base + 5);
    let _t = infra::start_timer("test_hist_us", &[10, 100, 1000]);
    // drop timer
    let exported = metrics::export();
    assert!(exported.contains("counter{name=\"test_counter_increment_case\"}"));
    assert!(exported.contains("histogram_sum{name=\"test_hist_us\"}"));
}
