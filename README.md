# Screenshot Toolkit

> Cross‑platform (macOS / Windows roadmap) screenshot capture & annotation engine. Focused on precise models, predictable rendering, and efficient export.

[中文文档 README.zh-CN.md](./README.zh-CN.md)

## Highlights
- Shapes: Rect, Highlight (Multiply/Screen), Arrow (solid/dashed), Mosaic, Freehand (Chaikin smoothing, solid), Text placeholder blocks
- Rendering: CPU RGBA composition, alpha blending, blend modes (Multiply / Screen), dashed strokes, stroke + fill, smoothing passes
- Export: PNG + JPEG (quality parameter)
- Undo: merge policies, time‑ordered UUID v7 IDs
- Planned: GPU / SIMD, real font rasterization, OCR assisted areas, privacy auto mosaic

## Repository Layout
```
crates/
  core/           # Core models, annotation types, undo stack, naming templates
  infra/          # Infrastructure: metrics, panic hook, LRU cache, path resolution
  renderer/       # CPU RGBA composition, blend modes, shape rendering
  services/       # Business logic orchestration (capture, annotate, export, history)
  platform_mac/   # macOS capture via xcap + screencapture, clipboard integration  
  platform_win/   # Windows capture (placeholder/stub for now)
  ui_overlay/     # Self-developed region selector with Iced GUI framework
  api_cli/        # CLI interface with capture commands and interactive selection
  api_napi/       # Node.js bindings (placeholder for future)
  ocr_adapter/    # OCR integration (planned)
  privacy/        # Privacy scanning & masking (planned)
  macros/         # Derive macros (planned)
```
Docs live under `docs/`:
- `docs/prd` product requirements
- `docs/tech_design` technical designs per crate + overview
- `docs/todo` per‑module task lists

## Quick Start
Run all tests:
```sh
cargo test --workspace
```
Build only renderer crate:
```sh
cargo build -p renderer
```
### MVP Status (2025-08) ✅ COMPLETED
Implemented end-to-end loop with full feature set:
- **Capture**: Full screen & region capture (macOS native via xcap + screencapture fallback, multi-monitor support with `--all`)
- **Interactive Selection**: Self-developed region selector with Iced-based GUI (replaces screencapture -i)
- **Annotations**: Rect / Arrow / Text + Undo/Redo + z-order manipulation (all undoable)
- **Export**: PNG to file & macOS clipboard (NSPasteboard), JPEG support built-in
- **Naming**: Template `{date},{seq},{screen}` with cross-process persistent daily sequence
- **History**: Recent 50 items with thumbnails (JSONL persistence, auto-trim by capacity)
- **CLI**: Complete commands: `capture`, `capture-region`, `capture-interactive`, `metrics`
- **Infrastructure**: Metrics framework, panic hook, LRU cache, path resolution

Pre-implemented features (ready for future use): Highlight / Mosaic / Freehand annotations.

### CLI Usage
Full screen capture:
```sh
cargo run -p api_cli -- capture -d shots
cargo run -p api_cli -- capture --all -d multi_screen  # Multi-monitor support
```

Region capture:
```sh
cargo run -p api_cli -- capture-region --rect 100,120,400,300 -d shots
```

Interactive selection (Self-developed UI):
```sh
cargo run -p api_cli -- capture-interactive -d shots --selector native  # Enhanced native selector
cargo run -p api_cli -- capture-interactive -d shots --selector gui    # Pure GUI selector
```

Mock mode (for testing without screen permissions):
```sh
cargo run -p api_cli -- capture -d shots --mock
```

View metrics:
```sh
cargo run -p api_cli -- metrics
```

Check results:
```sh
ls shots/*.png
cat shots/.history/history.jsonl | tail -n 3
cat shots/.history/seq.txt  # Cross-process sequence persistence
```
Thumbnails are embedded (base64-safe PNG bytes length) inside each JSONL line `thumb` field.
Sequence persistence: per-output directory a `.history/seq.txt` stores `YYYYMMDD <last_seq>` so restart keeps increasing within the same day.
Optional (future) features:
- `node` (N-API bindings)
- `simd_opt` (SIMD accelerated blend / mosaic)

## Architecture Flow
1. Frame captured or constructed
2. Annotations collected & z‑sorted
3. CPU renderer iterates & composites per kind
4. Export encoder writes PNG / JPEG bytes

## Roadmap (Current & Future)
- [x] Core / Renderer foundation
- [x] Basic annotations: Rect / Arrow / Text with Undo/Redo
- [x] Advanced annotations: Highlight / Mosaic / Freehand / Dashed strokes
- [x] PNG & JPEG export with quality control  
- [x] Interactive region selection with self-developed GUI
- [x] Multi-monitor capture support
- [x] Cross-process sequence persistence
- [x] History system with thumbnails
- [x] Infrastructure: metrics, panic handling, caching
- [ ] Font rasterization (fontdue integration)
- [ ] Snapshot baseline pixel tests
- [ ] DirtyRect / SIMD optimization
- [ ] OCR + Privacy suggested regions
- [ ] GPU backend prototype
- [ ] Windows platform implementation
- [ ] Node.js API bindings

## Contributing
1. Update related `docs/todo/<crate>.md` before large changes.
2. Add or adjust tests for new rendering behavior.
3. Keep public APIs minimal & stable; document feature flags.

## License
MIT (see LICENSE)

Coder: GPT5/Claude 4
