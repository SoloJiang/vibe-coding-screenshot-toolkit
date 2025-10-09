# AI Coding Instructions for Screenshot Toolkit

## Project Overview
Cross-platform screenshot toolkit with modular Rust architecture focused on precise models, predictable CPU rendering, and efficient export. Currently MVP-focused with full macOS support and Windows roadmap.

## Architecture & Module Boundaries

### Layered Architecture (Bottom-Up)
1. **`core/`** - Pure models, annotations, undo stack, naming templates (no I/O)
2. **`renderer/`** - CPU RGBA composition, alpha blending, blend modes
3. **`platform_*/`** - Platform-specific capture/clipboard (`platform_mac` via xcap, `platform_win` stub)
4. **`services/`** - Business orchestration (capture → annotate → render → export → history)
5. **`api_*/`** - Interface layers (`api_cli` complete, `api_napi` placeholder)
6. **`infra/`** - Cross-cutting concerns (metrics, panic hooks, LRU cache, path resolution)
7. **`ui_overlay/`** - Custom region selector GUI (independent windowing)

### Key Dependencies Flow
- Services depend on `core` + `renderer` + `platform_*` + `infra`
- APIs depend on `services` + platform-specific imports
- Platform modules are isolated (use feature flags for target_os)
- Cross-platform compatibility through trait abstractions in `services/`

## Error Handling Pattern

**Use `screenshot_core::ErrorKind` enum** for all domain errors:
```rust
use screenshot_core::{Error, ErrorKind, Result as CoreResult};

// Preferred pattern - structured errors
Err(Error::new(ErrorKind::Capture, "xcap failed"))

// Avoid unwrap() - use Result propagation
let screenshot = capturer.capture_full()
    .map_err(|e| Error::new(ErrorKind::Capture, e.to_string()))?;
```

**Mixed Result types**:
- Use `CoreResult<T>` for domain logic
- Use `anyhow::Result<T>` for service orchestration
- Convert between them at boundaries

## Critical Development Workflows

### Build & Quality Checks
```bash
# Primary development workflow
cargo fmt && cargo build --workspace  # Available as VS Code task
make ci                                # Complete CI (fmt-check + clippy + test)
make fix                               # Auto-fix formatting + clippy issues

# Testing patterns
cargo test --workspace                 # All tests
cargo test -p screenshot_core          # Single crate
```

### Platform-Specific Development
```rust
// Conditional compilation pattern
#[cfg(target_os = "macos")]
use platform_mac::MacCapturer;

#[cfg(not(target_os = "macos"))]
use services::MockCapturer;  // For testing/unsupported platforms
```

## Project-Specific Conventions

### Naming & Structure
- **Module naming**: Snake_case with descriptive suffixes (`platform_mac`, `api_cli`)
- **Error handling**: Domain-specific `ErrorKind` variants, avoid generic "Unknown"
- **Services pattern**: Trait + implementation for mockability (`Capturer`, `Clipboard`)

### Key Business Logic Patterns
- **Naming templates**: `{date},{seq},{screen}` with cross-process sequence persistence via `.history/seq.txt`
- **History management**: JSONL append-only with auto-trimming to 50 items
- **Undo system**: Command pattern with `UndoStack` in `core/undo.rs`
- **CLI architecture**: Clap subcommands with `--mock` flag for testing without capture permissions

### Rendering & Export
- **CPU-only rendering**: No GPU dependencies, pure RGBA composition
- **Format support**: PNG (primary), JPEG (implemented), clipboard integration
- **Coordinate system**: Logical pixels, multi-monitor via `--all` flag

## File Organization Patterns

### Documentation Structure
- `docs/tech_design/` - Per-crate technical designs + `overview.md`
- `docs/todo/` - Per-module task tracking (update when adding features)
- `docs/prd/` - Product requirements and MVP scope

### Integration Points
- **External deps**: Query via `context7` or `deepwiki` MCP services before adding
- **Platform integration**: xcap for macOS capture, NSPasteboard for clipboard
- **Persistence**: File-based with atomic writes, no database dependencies

## Testing & Debugging

### Mock Patterns
```rust
// CLI testing without capture permissions
screenshot-cli capture --mock          # Uses mock gray screenshot
screenshot-cli capture-interactive --mock  # Bypasses real screen capture
```

### Common Debug Scenarios
- **Capture issues**: Check display permissions on macOS, use `--mock` for testing
- **History corruption**: Check `.history/` directory structure and JSONL format
- **Cross-platform**: Use conditional compilation and mock implementations

## Key Files to Reference
- `crates/core/src/error.rs` - Central error definitions
- `crates/services/src/lib.rs` - Service orchestration patterns
- `docs/tech_design/overview.md` - Architecture constraints and MVP scope
- `Makefile` - Quality check workflows
- `.github/prompts/develop.prompt.md` - Chinese development requirements (strict module adherence)

Remember: Follow modular boundaries strictly, avoid unwrap(), prefer `Result` propagation, and keep platform-specific code isolated.