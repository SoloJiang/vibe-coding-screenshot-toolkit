# Screenshot Toolkit

> 高性能跨平台截图工具，支持交互式区域选择和多显示器环境。

🤖 **AI 生成**：整个代码库完全由 AI 生成和驱动，展示了 AI 辅助开发工作流的强大能力。

[English README](./README.md)

[![CI](https://github.com/SoloJiang/vibe-coding-screenshot-toolkit/actions/workflows/ci.yml/badge.svg)](https://github.com/SoloJiang/vibe-coding-screenshot-toolkit/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)

## ✨ 功能特性

### 🤖 AI 驱动开发
本项目展示了 AI 辅助软件开发的能力：
- **完整代码生成**：所有 Rust 代码、架构和实现均由 AI 生成
- **AI 指导工作流**：开发过程完全由 AI 规划和执行驱动
- **智能问题解决**：通过 AI 推理解决复杂技术挑战
- **自动化文档**：技术文档、PRD 和 TODO 管理均由 AI 处理

### 🎯 核心能力 (v0.1 - 当前版本)
- **🖱️ 交互式区域选择**：自研 GUI 覆盖层，支持鼠标拖拽选择
- **🖥️ 多显示器支持**：完整的跨显示器区域选择，虚拟桌面坐标系统
- **⚡ Metal GPU 加速**：原生 macOS Metal 渲染，60 FPS 流畅体验
- **📸 智能截图策略**：单显示器优化裁剪 vs 跨显示器合成
- **💾 高质量导出**：PNG 输出和系统剪贴板集成
- **🏷️ 智能命名**：基于时间模板的自动命名和序列编号
- **🛡️ 健壮错误处理**：用户友好的权限和错误提示

### 🚀 即将推出 (v0.2)
- **✏️ 标注工具**：矩形、箭头和自由绘制工具
- **🎨 实时编辑**：带撤销/重做功能的实时预览
- **🎛️ 工具栏 UI**：直观的编辑界面
- **🎨 样式自定义**：颜色选择器和笔触宽度控制

## 🚀 快速开始

### 安装

```bash
# 克隆仓库
git clone https://github.com/SoloJiang/vibe-coding-screenshot-toolkit.git
cd screenshot

# 构建项目
cargo build --workspace

# 运行测试
cargo test --workspace
```

### 基本使用

**交互式截图**（主要功能）：
```bash
# 基础交互式截图
cargo run -p api_cli -- capture-interactive

# 指定输出目录
cargo run -p api_cli -- capture-interactive -d ~/Screenshots

# 同时复制到剪贴板
cargo run -p api_cli -- capture-interactive --clipboard

# 自定义文件名模板
cargo run -p api_cli -- capture-interactive -t "MyShot-{date:yyyyMMdd-HHmmss}"
```

**命令行选项**：
```bash
cargo run -p api_cli -- capture-interactive --help
```

### 多显示器环境

- **自动检测**：自动检测所有连接的显示器
- **跨显示器选择**：选择跨越多个显示器的区域
- **虚拟桌面坐标**：统一的坐标映射系统
- **智能渲染**：单显示器与多显示器场景的性能优化

## 🏗️ 架构设计

```
crates/
├── core/           # 核心数据模型、错误类型、命名模板
├── platform_mac/   # macOS 实现（xcap + Metal GPU）
├── platform_win/   # Windows 实现（框架就绪）
├── ui_overlay/     # 交互式区域选择器，Metal 渲染
├── services/       # 业务逻辑：捕获、导出、标注服务
├── renderer/       # CPU 渲染引擎（降级方案）
├── api_cli/        # 命令行接口
└── infra/          # 基础设施：指标、缓存、工具
```

## 🎮 使用指南

### 交互式截图工作流

1. **启动**：运行 `capture-interactive` 命令
2. **选择区域**：
   - 左键拖拽选择矩形区域
   - 支持跨显示器区域选择
   - 使用 `Shift` 固定正方形，`Alt` 中心拉伸
3. **调整**：拖拽边框精确调整选择区域
4. **确认**：按 `Enter` 或 `Space` 截图
5. **取消**：按 `Esc` 中止操作
6. **导出**：自动保存到文件并可选择复制到剪贴板

### 文件命名模板

支持的变量：
- `{date:format}` - 日期/时间格式
  - `{date:yyyyMMdd}` → 20240922
  - `{date:yyyyMMdd-HHmmss}` → 20240922-143022
- `{seq}` - 当日序列号（每日重置）

示例：`Screenshot-{date:yyyyMMdd-HHmmss}-{seq}` → `Screenshot-20240922-143022-001.png`

### macOS 权限设置

首次使用需要屏幕录制权限：
1. 打开"系统偏好设置" → "安全性与隐私" → "隐私"
2. 选择"屏幕录制"
3. 点击锁图标解锁，勾选本应用
4. 重启应用程序

## 🔧 技术特性

### 高性能渲染
- **Metal GPU 加速**：原生 macOS Metal 渲染，60 FPS 性能
- **CPU 降级**：软件渲染确保兼容性
- **智能缓存**：背景缓存和图像复用
- **内存优化**：Arc 共享避免像素数据拷贝

### 多显示器架构
- **虚拟桌面系统**：跨显示器的统一坐标映射
- **智能截图策略**：
  - 单显示器选择 → 直接裁剪（最优性能）
  - 跨显示器选择 → 虚拟桌面合成
- **实时预览**：交互过程中的实时选择反馈

### 跨平台设计
- **macOS**：Metal GPU 加速的完整实现
- **Windows**：框架就绪，实现中
- **模块化架构**：平台特定代码隔离在平台模块中

## 📋 开发路线图

### v0.1 - 交互式截图核心 ✅
- [x] 自定义 GUI 的交互式区域选择
- [x] 多显示器支持和跨显示器选择
- [x] Metal GPU 渲染，60 FPS 性能
- [x] PNG 导出和剪贴板集成
- [x] 基于时间模板的智能文件命名
- [x] 健壮的错误处理和用户指导

### v0.2 - 标注编辑系统 🚧
- [ ] 带标注工具的工具栏 UI
- [ ] 矩形和箭头绘制工具
- [ ] 自由绘制支持
- [ ] 颜色选择器和笔触自定义
- [ ] 撤销/重做功能
- [ ] 编辑过程中的实时预览

### v0.3 - 高级标注功能 📋
- [ ] 带字体选择的文字标注
- [ ] 马赛克和高亮工具
- [ ] 高级编辑功能
- [ ] 标注模板和预设

### v1.0 - 完整产品 🎯
- [ ] Windows 平台完整支持
- [ ] 全局快捷键支持
- [ ] 自定义工具插件系统
- [ ] 历史记录管理界面
- [ ] 云端同步功能

## 🧪 测试

```bash
# 运行所有测试
cargo test --workspace

# 运行特定模块测试
cargo test -p api_cli
cargo test -p platform_mac

# 构建发布版本
cargo build --release
```

## 📊 性能指标

- **启动时间**：< 500ms
- **界面响应**：< 50ms
- **截图处理**：< 1s（4K 分辨率）
- **内存使用**：< 100MB（无标注编辑时）
- **渲染性能**：Metal GPU 加速 60 FPS

## 🤖 AI 开发展示

本项目展示了 AI 驱动软件开发的潜力：

### AI 生成组件
- **完整 Rust 代码库**：所有 15+ 个 crate 和模块均由 AI 生成
- **架构设计**：模块化、跨平台架构由 AI 设计
- **性能优化**：Metal GPU 渲染和 60 FPS 优化由 AI 实现
- **错误处理**：全面的错误处理和用户指导由 AI 编写
- **文档管理**：技术文档、PRD 和开发任务由 AI 管理

### AI 开发工作流
- **规划**：AI 分析需求并创建详细的开发计划
- **实现**：AI 生成代码、测试和文档
- **问题解决**：AI 识别并解决复杂技术挑战
- **迭代**：AI 持续改进代码质量和性能
- **维护**：AI 管理持续开发和功能添加

## 🤝 贡献指南

本项目欢迎探索 AI 辅助开发的贡献：
1. 遵循现有的模块化架构和边界
2. 所有更改必须通过测试套件
3. 重要功能需要更新文档
4. 遵循 [Rust 代码规范](https://doc.rust-lang.org/1.0.0/style/)
5. 考虑如何通过 AI 增强开发过程

## 📄 许可证

MIT License - 详见 [LICENSE](./LICENSE) 文件

## 🔗 相关文档

- [技术设计](./docs/tech_design/overview.md)
- [产品需求](./docs/prd/prd.md)
- [开发任务](./docs/todo/)

---

**注意**：本项目目前专注于交互式截图核心功能。标注编辑和高级功能计划在后续版本中实现。