# Screenshot Toolkit

> 专注交互式截图的跨平台工具，支持多显示器环境。

[中文文档 README.zh-CN.md](./README.zh-CN.md)

## 🎯 核心特性 (MVP v0.1)
- **✅ 交互式截图**：通过自研 GUI 区域选择器精确选择截图区域
- **✅ 多显示器支持**：完整支持多显示器环境和跨显示器区域选择
- **✅ 高质量导出**：PNG 格式输出和系统剪贴板集成
- **✅ 智能命名**：基于时间和序列的自动文件命名
- **✅ 友好错误处理**：详细的权限和错误提示
- **✅ macOS 完整支持**：基于 xcap 的高性能截图实现

## 🚀 快速开始

### 安装和构建
```bash
# 克隆项目
git clone https://github.com/SoloJiang/vibe-coding-screenshot-toolkit.git
cd screenshot

# 构建项目
cargo build --workspace

# 运行测试
cargo test --workspace
```

### 基本使用

**交互式截图**（推荐）：
```bash
# 基础交互式截图
./target/debug/api_cli capture-interactive

# 指定输出目录
./target/debug/api_cli capture-interactive -d ~/Screenshots

# 同时复制到剪贴板
./target/debug/api_cli capture-interactive --clipboard

# 自定义文件名模板
./target/debug/api_cli capture-interactive -t "MyShot-{date:yyyyMMdd-HHmmss}"
```

**完整参数说明**：
```bash
./target/debug/api_cli capture-interactive --help
```

### 多显示器环境
- **自动检测**：程序会自动检测所有连接的显示器
- **跨显示器选择**：支持选择跨越多个显示器的区域
- **虚拟桌面坐标**：统一处理多显示器的坐标映射
- **智能截图**：自动选择最优的截图策略（单显示器裁剪 vs 全局合成）

## 📁 项目架构
```
crates/
  core/           # 核心数据模型、错误类型、命名模板
  platform_mac/   # macOS 截图和剪贴板实现（完整）
  platform_win/   # Windows 截图实现（规划中）
  ui_overlay/     # 交互式区域选择器
  services/       # 业务逻辑：捕获、导出服务
  renderer/       # CPU 渲染引擎
  api_cli/        # 命令行接口
  infra/          # 基础设施：指标、缓存等
```

技术文档位于 `docs/` 目录：
- `docs/prd/mvp.md` - MVP 产品需求
- `docs/tech_design/` - 详细技术设计
- `docs/todo/` - 开发任务追踪
## 🎮 使用说明

### 交互式截图操作
1. **启动程序**：运行 `capture-interactive` 命令
2. **选择区域**：
   - 鼠标左键按住拖拽选择矩形区域
   - 支持跨多个显示器的区域选择
3. **调整区域**：选择后可拖拽边框精确调整
4. **确认截图**：
   - 按 `Enter` 或 `Space` 确认截图
   - 按 `Esc` 取消操作
5. **完成**：截图自动保存并可选复制到剪贴板

### 文件命名模板
支持以下变量：
- `{date:format}` - 日期时间格式
  - `{date:yyyyMMdd}` - 20240922
  - `{date:yyyyMMdd-HHmmss}` - 20240922-143022
- `{seq}` - 当日序列号（每日重置）

示例：`Screenshot-{date:yyyyMMdd-HHmmss}-{seq}` → `Screenshot-20240922-143022-001.png`

### 权限设置 (macOS)
首次使用需要授予屏幕录制权限：
1. 打开"系统偏好设置" → "安全性与隐私" → "隐私"
2. 选择"屏幕录制"
3. 点击锁图标解锁，勾选本应用
4. 重启应用程序生效

## 🔧 技术特点

### 多显示器架构
- **虚拟桌面坐标系**：统一的坐标映射系统
- **智能截图策略**：
  - 单显示器内选择 → 直接裁剪，性能最优
  - 跨显示器选择 → 虚拟桌面合成
- **实时预览**：选择过程中实时显示区域效果

### 高性能实现
- **CPU 渲染**：无 GPU 依赖，兼容性强
- **内存优化**：Arc 共享，避免像素数据拷贝
- **缓存机制**：显示器检测和背景缓存

## 📋 MVP 状态

### ✅ 已完成功能
- 交互式区域选择（包括跨显示器）
- PNG 文件保存和剪贴板复制
- 多显示器自动检测和坐标处理
- 智能文件命名和时间模板
- 友好的错误处理和权限提示
- 完整的 CLI 接口

### 🔄 后续版本计划
- **v0.2**: Windows 平台支持
- **v0.3**: 标注功能（箭头、文字、马赛克）
- **v1.0**: GUI 应用程序界面

## 🧪 测试

```bash
# 运行所有测试
cargo test --workspace

# 运行特定模块测试
cargo test -p api_cli
cargo test -p screenshot_core

# 构建发布版本
cargo build --release
```

## 🤝 贡献指南
1. 遵循现有的模块架构和边界
2. 所有更改需要通过测试
3. 重要功能需要更新文档
4. 遵循 [Rust 代码规范](https://doc.rust-lang.org/1.0.0/style/)

## 📄 许可证
MIT License - 详见 [LICENSE](./LICENSE) 文件

## 🔗 相关链接
- [技术设计文档](./docs/tech_design/overview.md)
- [MVP 需求文档](./docs/prd/mvp.md)
- [开发任务追踪](./docs/todo/)

---

**注意**: 本项目目前专注于 MVP 功能，标注、历史管理等高级功能将在后续版本中实现。

