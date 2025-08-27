# 截图开发 CLI

[English](./README.md) | 简体中文

用于截图工具集工作区的开发调试 CLI。提供全屏 / 区域 / 多显示器截图、文件命名模板等基础能力，方便在 MVP 阶段验证核心 crate。

> 状态：MVP / 开发者工具（暂未面向终端用户打包）

## 功能
- 全屏截图（macOS 真实捕获：`xcap` + `screencapture` 回退；其他平台使用灰色 mock）
- 多显示器截图 (`--all`)，按屏索引命名
- 区域截图（基于全屏结果裁剪）
- PNG 导出（内部已有 JPEG 支持，尚未暴露参数）
- 命名模板：`{date}`、`{seq}`、`{screen}` 占位符
- 捕获失败自动回退 / 使用 mock，保证命令可继续

## 编译与运行
在仓库根目录：
```bash
cargo build -p api_cli
```
默认打印版本：
```bash
cargo run -p api_cli --
```

## 命令列表
```
capture           全屏截图（单屏或全部显示器）
capture-region    区域截图（从全屏裁剪）
version           显示版本（缺省子命令时触发）
```

## capture
全屏 / 多屏截图。

### 参数
| 参数 | 别名 | 说明 | 默认 |
|------|------|------|------|
| `-d`, `-o`, `--out-dir`, `--out` |  | 输出目录 | `.` |
| `-t`, `--template` |  | 命名模板（不含扩展名） | `Screenshot-{date:yyyyMMdd-HHmmss}-{seq}` |
| `--all` |  | (macOS) 捕获所有显示器 | `false` |

示例：
```bash
cargo run -p api_cli -- capture -d ./shots
cargo run -p api_cli -- capture --all -d ./multi -t "Shot-{date:yyyyMMdd-HHmmss}-{seq}-{screen}"
```
非 macOS 平台会生成 800x600 灰底 mock 图片。

### 多显示器行为
- 每个成功的显示器输出一张 PNG
- 某个显示器失败仅记录 warn 并跳过
- 全部失败 -> 回退单屏；仍失败 -> 使用 mock

## capture-region
从全屏截图裁剪一个矩形区域（macOS 真实，其它平台 mock）。

### 参数
| 参数 | 说明 |
|------|------|
| `-d`, `-o`, `--out-dir`, `--out` | 输出目录 |
| `-t`, `--template` | 命名模板 |
| `--rect` | 逗号分隔 `x,y,w,h` 四个整数 |

示例：
```bash
cargo run -p api_cli -- capture-region --rect 100,120,300,200 -d ./crop
```

## 命名模板
支持占位符：
- `{date:FORMAT}` 日期时间（格式 token：`yyyy MM dd HH mm ss`）
- `{seq}` 当天自增序号（跨日自动重置）
- `{screen}` 屏幕序号（`--all` 时有意义）

默认模板：`Screenshot-{date:yyyyMMdd-HHmmss}-{seq}` 生成示例：
```
Screenshot-20250201-101530-1.png
Screenshot-20250201-101530-2.png
```

## 权限说明（macOS）
首次运行需授予“屏幕录制”权限：系统设置 -> 隐私与安全性 -> 屏幕录制。未授权时会回退并可能使用 mock。

## 退出与错误
- 目前仅在参数错误（例如 `--rect` 格式不正确）时返回非 0。
- 捕获失败会尽量回退，最终给 mock，保持开发流程不中断。

## Roadmap（CLI）
- 增加 JPEG 导出参数 (`--jpeg-quality`)
- 接入历史记录 / 缩略图（服务层已实现）
- 剪贴板复制选项 (`--copy`)
- 结构化 JSON 输出 (`--json`) 方便脚本集成

## 许可证
MIT（见仓库根目录 LICENSE）
