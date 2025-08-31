# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-08-28

### Added

#### Core Features
- **Interactive Screenshot Selection**: Self-developed region selector using Iced GUI framework
  - Enhanced native selector with macOS `screencapture` integration
  - Pure GUI selector with interactive coordinate input
  - Support for Esc/Enter keyboard shortcuts for cancel/confirm
- **Multi-Monitor Support**: Capture all displays with `--all` flag
- **Cross-Process Sequence Persistence**: Maintain daily incremental sequence across CLI invocations
- **Metrics Framework**: Built-in performance monitoring and statistics export
- **Panic Hook System**: Enhanced error reporting and debugging capabilities

#### CLI Commands
- `capture`: Full screen capture with multi-monitor support
- `capture-region`: Region capture with coordinate specification
- `capture-interactive`: Interactive region selection with custom UI
- `metrics`: Export system performance metrics
- `version`: Display version information

#### Infrastructure
- **LRU Cache**: Generic caching system for performance optimization
- **Path Resolution**: Robust directory and file path management
- **History System**: Persistent storage of screenshot history with thumbnails
- **Naming Templates**: Advanced template system with `{date}`, `{seq}`, `{screen}` placeholders

#### Testing & Quality
- Cross-platform integration tests for CLI commands
- Sequence persistence testing across process boundaries
- Mock mode for testing without screen recording permissions
- Comprehensive error handling and validation

### Implemented

#### Core Module (`screenshot_core`)
- Screenshot, Frame, and FrameSet data structures
- Annotation types: Rect, Arrow, Text (with Highlight, Mosaic, Freehand pre-implemented)
- Undo/Redo system with merge strategies for continuous operations
- Naming template parser with date formatting and sequence management
- History management with capacity-based trimming
- Serialization support for annotations and history items

#### Infrastructure Module (`infra`)
- Metrics registry with counters and histograms
- Event bus for inter-component communication
- Configuration management with atomic file operations
- UUID v7 ID generation for time-ordered identifiers
- LRU cache implementation with O(1) operations
- Panic hook installation for enhanced debugging

#### Platform Integration (`platform_mac`)
- XCap-based screen capture with fallback to `screencapture`
- Multi-monitor detection and capture
- Region capture using native macOS tools
- Clipboard integration for PNG export
- Permission detection and error reporting

#### Services Layer (`services`)
- Capture orchestration with platform abstraction
- Annotation service with CRUD operations and undo support
- Export service with PNG/JPEG encoding and clipboard support
- History service with JSONL persistence and thumbnail generation
- Privacy scanning (basic regex-based detection)
- OCR service placeholder with thread pool architecture

#### UI Overlay (`ui_overlay`)
- Iced-based GUI framework integration
- Region selector trait with multiple implementations
- Enhanced native selector combining system tools with UI improvements
- Interactive coordinate input with presets and validation
- Background image handling for visual context

#### Renderer Module (`renderer`)
- CPU-based RGBA composition engine
- Support for multiple annotation types with z-ordering
- PNG and JPEG encoding with quality control
- Blend modes: Normal, Multiply, Screen
- Anti-aliasing and smoothing for vector graphics

### Technical Improvements
- **Type Safety**: Comprehensive use of Rust's type system for correctness
- **Error Handling**: Unified error types across all modules
- **Memory Management**: Arc-based sharing to minimize data copying
- **Concurrency**: Thread-safe implementations where required
- **Performance**: Optimized data structures and algorithms

### Documentation
- Comprehensive technical design documents for all modules
- Updated MVP requirements and acceptance criteria
- API documentation with usage examples
- Installation and setup guides
- Troubleshooting and FAQ sections

### Developer Experience
- Workspace-based multi-crate project structure
- Unified dependency management with workspace inheritance
- Feature flags for optional components
- Development prompts and guidelines
- Comprehensive test coverage

## [Unreleased] - Future Features

### Planned
- Font rasterization with fontdue integration
- DirtyRect optimization for performance
- SIMD acceleration for rendering operations
- OCR integration with tesseract
- Privacy auto-masking with AI detection
- GPU rendering backend
- Windows platform implementation
- Node.js API bindings (NAPI)
- Real-time collaborative annotations
- Cloud storage integration
- Advanced selection shapes (circles, polygons)

### Under Consideration
- Web assembly compilation
- Mobile platform support
- Plugin system for custom annotations
- Scriptable automation interface
- Integration with popular design tools

---

## Development Status

### MVP Completion ✅
All MVP requirements have been successfully implemented and tested:
- End-to-end screenshot capture and annotation workflow
- Cross-platform architecture (macOS complete, Windows stub)
- Extensible plugin system foundation
- Production-ready CLI interface
- Comprehensive test coverage

### Next Milestones
1. **Enhanced UI** - Advanced interactive features and visual improvements
2. **Windows Support** - Complete Windows platform implementation  
3. **Performance** - SIMD and GPU acceleration
4. **Integrations** - OCR, cloud storage, and external tool APIs
5. **Ecosystem** - Plugin system and community contributions
