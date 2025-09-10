# Screenshot Toolkit

> Cross‑platform (macOS / Windows roadmap) screenshot capture & annotation engine. Focused on precise models, predictable rendering, and efficient export.

[中文文档 README.zh-CN.md](./README.zh-CN.md)

## Highlights
- Shapes: Rect, Highlight (Multiply/Screen), Arrow (solid/dashed), Mosaic, Freehand (Chaikin smoothing, solid), Text placeholder blocks
- Rendering: CPU RGBA composition, alpha blending, blend modes (Multiply / Screen), dashed strokes, stroke + fill, smoothing passes

# Screenshot Toolkit
```
crates/
  core/           # Core models, annotation types, undo stack, naming templates
  infra/          # Infrastructure: metrics, panic hook, LRU cache, path resolution
  renderer/       # CPU RGBA composition, blend modes, shape rendering
  services/       # Business logic orchestration (capture, annotate, export, history)
  platform_mac/   # macOS capture via xcap (>=0.7), clipboard integration
  platform_win/   # Windows capture (placeholder/stub for now)
  ui_overlay/     # Self-developed region selector
  api_cli/        # CLI interface with capture commands and interactive selection
  api_napi/       # Node.js bindings (placeholder for future)
Docs live under `docs/`:
- `docs/prd` MVP scope and acceptance
- `docs/tech_design` technical designs per crate + overview
- `docs/todo` per‑module task lists
Docs live under `docs/`:
- `docs/prd` product requirements
- `docs/tech_design` technical designs per crate + overview
- `docs/todo` per‑module task lists

## Quick Start
Run all tests:
```sh
cargo test --workspace
```

### Status
End-to-end loop for capture → annotate → render → export is implemented. Interactive region selection is available via a custom GUI overlay.
- Capture: Full screen and region (xcap on macOS; multi–monitor via `--all`)
- Interactive selection: Region selection via custom GUI overlay
- Annotations: Rect / Arrow / Text with Undo/Redo and z‑order controls
- Export: PNG to file and macOS clipboard (NSPasteboard); JPEG encoder present
- Naming: Template `{date},{seq},{screen}` with per‑day sequence persistence
- History: Recent 50 items with thumbnails (JSONL persistence, auto‑trim)
- CLI: `capture`, `capture-region`, `capture-interactive`, `metrics`
- Infra: Metrics, panic hook, LRU cache, path resolution
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
# Select region via the custom GUI overlay and save PNG
```

Interactive selection:
```sh
cargo run -p api_cli -- capture-interactive -d shots
# Use the GUI selector to choose region and save PNG
```

Mock mode (for testing without screen permissions):
```sh
cargo run -p api_cli -- capture -d shots --mock
```

View metrics:
```sh
tail -n 3 shots/.history/history.jsonl
```

Thumbnails are embedded (PNG bytes length) inside each JSONL line `thumb` field.
Sequence persistence: per-output directory a `.history/seq.txt` stores `YYYYMMDD <last_seq>` so restarts keep increasing within the same day.
```
Thumbnails are embedded (base64-safe PNG bytes length) inside each JSONL line `thumb` field.
Sequence persistence: per-output directory a `.history/seq.txt` stores `YYYYMMDD <last_seq>` so restart keeps increasing within the same day.
Optional (future) features:
- `node` (N-API bindings)
- `simd_opt` (SIMD accelerated blend / mosaic)

## Roadmap
- [x] Core / Renderer foundation
- [x] Basic annotations: Rect / Arrow / Text with Undo/Redo
- [x] Advanced annotations: Highlight / Mosaic / Freehand / Dashed strokes
- [x] PNG & JPEG export with quality control
- [x] Multi-monitor capture support
- [x] Cross-process sequence persistence
- [x] History system with thumbnails
- [x] Infrastructure: metrics, panic handling, caching
- [ ] Interactive region selection GUI
- [ ] Font rasterization (fontdue integration)
- [ ] Snapshot baseline pixel tests
- [ ] DirtyRect / SIMD optimization
- [ ] OCR + Privacy suggested regions

## Contributing
1. Update related `docs/tech_design/*.md` when changing architecture.
2. Add or adjust tests for new rendering behavior.
3. Keep public APIs minimal & stable; document feature flags.

## License
MIT (see LICENSE)
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
