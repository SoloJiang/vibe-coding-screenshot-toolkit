# Installation & Setup Guide

## Prerequisites

### macOS
- Rust 1.70+ (recommended: use [rustup](https://rustup.rs/))
- Xcode Command Line Tools: `xcode-select --install`
- Screen Recording permission (System Settings → Privacy & Security → Screen Recording)

### System Requirements
- macOS 10.15+ (Catalina or later)
- 4GB RAM minimum, 8GB recommended
- 100MB disk space for build artifacts

## Installation

### 1. Clone Repository
```bash
git clone https://github.com/YourOrg/vibe-coding-screenshot-toolkit.git
cd vibe-coding-screenshot-toolkit
```

### 2. Build Project
```bash
# Build all crates
cargo build --workspace --release

# Or build specific CLI tool
cargo build -p api_cli --release
```

### 3. Run Tests (Optional)
```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p core
cargo test -p services
```

## Quick Start

### Basic Screenshot Capture
```bash
# Full screen capture
cargo run -p api_cli -- capture -d ./screenshots

# Multi-monitor capture (macOS)
cargo run -p api_cli -- capture --all -d ./screenshots

# Region capture (crop from full screen)
cargo run -p api_cli -- capture-region --rect 100,100,800,600 -d ./screenshots
```

### Interactive Selection
```bash
# Enhanced native selector (recommended)
cargo run -p api_cli -- capture-interactive -d ./screenshots --selector native

# Pure GUI selector
cargo run -p api_cli -- capture-interactive -d ./screenshots --selector gui
```

### Mock Mode (No Permissions Required)
```bash
# Use mock gray image for testing
cargo run -p api_cli -- capture -d ./screenshots --mock
cargo run -p api_cli -- capture-region --rect 50,50,400,300 -d ./screenshots --mock
```

## Permission Setup (macOS)

### Screen Recording Permission
1. Run any capture command first (it will fail with permission error)
2. Open System Settings → Privacy & Security → Screen Recording
3. Enable permission for Terminal (or your shell application)
4. Restart terminal and try again

### Troubleshooting Permissions
```bash
# Test if permissions are working
cargo run -p api_cli -- capture --mock  # Should always work
cargo run -p api_cli -- capture         # Requires screen recording permission
```

## Configuration

### Naming Templates
Customize file naming with templates:
```bash
# Default template: Screenshot-{date:yyyyMMdd-HHmmss}-{seq}
cargo run -p api_cli -- capture -t "MyShot-{date:yyyy-MM-dd}-{seq}" -d ./shots

# Available placeholders:
# {date:FORMAT}  - Date/time formatting
# {seq}          - Daily incremental sequence  
# {screen}       - Screen index (for multi-monitor)
```

### Output Directories
```bash
# Specify output directory
cargo run -p api_cli -- capture -d ~/Desktop/screenshots

# Output includes:
# ├── Screenshot-20250828-143022-1.png  # Main screenshot
# ├── Screenshot-20250828-143022-2.png  # Next screenshot
# └── .history/
#     ├── history.jsonl                 # History records with thumbnails
#     └── seq.txt                       # Sequence persistence (YYYYMMDD last_seq)
```

## Advanced Usage

### View System Metrics
```bash
cargo run -p api_cli -- metrics
```

### History Management
```bash
# View history (JSONL format)
cat ./screenshots/.history/history.jsonl | tail -5

# Check sequence state
cat ./screenshots/.history/seq.txt
```

### Development Mode
```bash
# Enable debug logging
RUST_LOG=debug cargo run -p api_cli -- capture -d ./debug_shots

# Run with verbose output
cargo run -p api_cli --verbose -- capture -d ./shots
```

## Building Distribution

### Release Build
```bash
# Optimized release build
cargo build --release --workspace

# CLI binary location
./target/release/api_cli capture --help
```

### Feature Flags
```bash
# Build without optional features
cargo build -p ui_overlay --no-default-features

# Available features:
# - default: All standard features
# - iced-ui: Iced-based GUI components (default)
```

## Troubleshooting

### Common Issues

#### "Permission denied" errors
- Ensure Screen Recording permission is granted
- Try `--mock` flag for testing without permissions

#### Build failures
```bash
# Clean and rebuild
cargo clean
cargo build --workspace

# Update dependencies
cargo update
```

#### Runtime errors
```bash
# Check system compatibility
cargo run -p api_cli -- version

# Test with mock mode first
cargo run -p api_cli -- capture --mock -d ./test
```

### Performance Issues
```bash
# Check metrics for performance data
cargo run -p api_cli -- metrics

# Use release build for better performance
cargo build --release
cargo run --release -p api_cli -- capture -d ./shots
```

## Support

### Getting Help
- Check documentation: `docs/` directory
- View technical designs: `docs/tech_design/`
- Check TODO lists: `docs/todo/`

### Reporting Issues
Include the following information:
- OS version (macOS version)
- Rust version: `rustc --version`
- Command that failed
- Full error output
- Screenshot permissions status

### Development
- See `docs/tech_design/overview.md` for architecture
- Check `CONTRIBUTING.md` for development guidelines
- Review module-specific docs in `docs/tech_design/`
