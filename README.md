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
  core/ infra/ renderer/ services/
  api_cli/ api_napi/ platform_mac/ platform_win/
  ocr_adapter/ privacy/ macros/
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
Optional (future) features:
- `node` (N-API bindings)
- `simd_opt` (SIMD accelerated blend / mosaic)

## Architecture Flow
1. Frame captured or constructed
2. Annotations collected & z‑sorted
3. CPU renderer iterates & composites per kind
4. Export encoder writes PNG / JPEG bytes

## Roadmap (Excerpt)
- [x] Core / Renderer foundation
- [x] Highlight / Arrow / Mosaic / Dashed / Freehand / JPEG
- [ ] Font raster (fontdue)
- [ ] Snapshot baseline pixel tests
- [ ] DirtyRect / SIMD scaffold
- [ ] OCR + Privacy suggested regions
- [ ] GPU backend prototype

## Contributing
1. Update related `docs/todo/<crate>.md` before large changes.
2. Add or adjust tests for new rendering behavior.
3. Keep public APIs minimal & stable; document feature flags.

## License
MIT (see LICENSE)

Coder: GPT5/Claude 4
