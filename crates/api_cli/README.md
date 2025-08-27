# Screenshot Dev CLI

English | [简体中文](./README.zh-CN.md)

Development CLI for the screenshot toolkit workspace. Provides basic full screen / region capture, multi‑monitor export (macOS), and file naming templating for validating the core crates during MVP phase.

> Status: MVP / developer tool (not packaged for end users yet)

## Features
- Full screen capture (real on macOS via `xcap` + `screencapture` fallback, mock elsewhere)
- Multi‑monitor capture (`--all`) with per‑screen indexed file names
- Region capture (crop from full screenshot)
- PNG export (JPEG internal support present, not yet wired as flag)
- Naming template with placeholders `{date}`, `{seq}`, `{screen}`
- Graceful fallback to mock image when platform capture fails

## Install / Run
Workspace build (from repo root):
```bash
cargo build -p api_cli
```
Run (show version by default):
```bash
cargo run -p api_cli --
```

## Commands
```
capture           Full screen capture (one or all displays)
capture-region    Crop a region from a full screen capture
version           Print version (default when no subcommand is supplied)
```

## capture
Full or multi‑monitor capture.

### Options
| Flag | Alias | Description | Default |
|------|-------|-------------|---------|
| `-d`, `-o`, `--out-dir`, `--out` |  | Output directory | `.` |
| `-t`, `--template` | | Naming template (without extension) | `Screenshot-{date:yyyyMMdd-HHmmss}-{seq}` |
| `--all` | | Capture all displays (macOS only) | `false` |

Example:
```bash
cargo run -p api_cli -- capture -d ./shots
cargo run -p api_cli -- capture --all -d ./multi -t "Shot-{date:yyyyMMdd-HHmmss}-{seq}-{screen}"
```

On non‑macOS platforms a gray mock image (800x600) is generated.

### Multi‑monitor behavior
- Each successful display produces one PNG.
- Failures on an individual display are logged (warning) and skipped.
- If all displays fail, tool falls back to single capture; if that fails, falls back to mock.

## capture-region
Crop a rectangle from a full screenshot (macOS real capture or mock elsewhere).

### Options
| Flag | Description |
|------|-------------|
| `-d`, `-o`, `--out-dir`, `--out` | Output directory |
| `-t`, `--template` | Naming template |
| `--rect` | Comma separated `x,y,w,h` (integers) |

Example:
```bash
cargo run -p api_cli -- capture-region --rect 100,120,300,200 -d ./crop
```

## Naming Template
Supported placeholders inside `{}`:
- `{date:FORMAT}` Date/time with formatting tokens: `yyyy MM dd HH mm ss`
- `{seq}` Per‑day incremental sequence (resets when day changes)
- `{screen}` Zero‑based screen index (only meaningful with `--all`)

Default template: `Screenshot-{date:yyyyMMdd-HHmmss}-{seq}` producing e.g.
```
Screenshot-20250201-101530-1.png
Screenshot-20250201-101530-2.png
```
(Second line produced by a second run in the same second; sequence increments.)

## Permissions (macOS)
First run requires granting Terminal / your shell process “Screen Recording” permission:
System Settings -> Privacy & Security -> Screen Recording.
If permission is missing you'll see a fallback warning and potentially a mock image.

## Exit / Errors
- Non‑zero exit only on argument/rect validation failure currently.
- Capture failures still produce a mock image (zero exit) to keep developer iteration fast.

## Roadmap (CLI)
- Add JPEG export flag (`--jpeg-quality`)
- Wire history / thumbnail recording (already implemented in services)
- Add clipboard copy flag (`--copy`)
- Optional JSON output (`--json`) for automation

## License
MIT (see repository root LICENSE)
