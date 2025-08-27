# Screenshot Toolkit

> 多平台（macOS/Windows 规划中）截图与标注核心库。聚焦高质量截图捕获、标注形状渲染与高性能导出。

[English README](./README.md)

## 特性概览
- 标注类型：矩形 / 高亮(乘/滤色) / 箭头(含虚线) / 马赛克 / 自由手绘(Chaikin 平滑) / 文本占位
- 渲染：CPU RGBA 合成，Alpha + Multiply / Screen，虚线、描边、平滑曲线
- 导出：PNG / JPEG (质量可调)
- Undo：合并策略 + 时间序列 ID
- 规划：GPU / SIMD / 真正字体栅格 / OCR 建议 / 隐私自动马赛克

## 目录结构
```
crates/
  core/           # 核心模型、标注类型、撤销栈、命名模板
  infra/          # 基础设施：指标、panic 处理、LRU 缓存、路径解析
  renderer/       # CPU RGBA 合成、混合模式、形状渲染
  services/       # 业务逻辑编排（捕获、标注、导出、历史）
  platform_mac/   # macOS 捕获（xcap + screencapture）、剪贴板集成
  platform_win/   # Windows 捕获（占位/桩实现）
  ui_overlay/     # 自研区域选择器，基于 Iced GUI 框架
  api_cli/        # CLI 接口，含截图命令和交互式选择
  api_napi/       # Node.js 绑定（规划中）
  ocr_adapter/    # OCR 集成（规划中）
  privacy/        # 隐私扫描与遮罩（规划中）
  macros/         # 派生宏（规划中）
```
文档：`docs/prd` 产品，`docs/tech_design` 技术设计，`docs/todo` 模块任务。

## 快速开始
```sh
cargo test --workspace
cargo build -p renderer
```

### MVP 状态 (2025-08) ✅ 已完成
已完成端到端完整功能闭环：
- **截图捕获**：全屏 & 区域截图（macOS 原生 xcap + screencapture 回退，多显示器支持 `--all`）
- **交互式选择**：自研区域选择器，基于 Iced GUI 框架（替代 screencapture -i）
- **标注功能**：矩形 / 箭头 / 文本 + 撤销/重做 + 图层顺序操作（均可撤销）
- **导出功能**：PNG 文件导出 & macOS 剪贴板，内置 JPEG 支持
- **命名模板**：`{date},{seq},{screen}` 模板，支持跨进程当日序列持久化
- **历史记录**：最近 50 条记录含缩略图（JSONL 持久化，按容量自动裁剪）
- **CLI 命令**：完整命令集：`capture`、`capture-region`、`capture-interactive`、`metrics`
- **基础设施**：指标框架、panic 处理、LRU 缓存、路径解析

预埋功能（可用于未来扩展）：高亮 / 马赛克 / 自由手绘 标注。

### CLI 示例
全屏截图：
```sh
cargo run -p api_cli -- capture -d shots
cargo run -p api_cli -- capture --all -d multi_screen  # 多显示器支持
```

区域截图：
```sh
cargo run -p api_cli -- capture-region --rect 100,120,400,300 -d shots
```

交互式选择（自研 UI）：
```sh
cargo run -p api_cli -- capture-interactive -d shots --selector native  # 增强原生选择器
cargo run -p api_cli -- capture-interactive -d shots --selector gui    # 纯 GUI 选择器
```

模拟模式（无权限测试）：
```sh
cargo run -p api_cli -- capture -d shots --mock
```

查看指标：
```sh
cargo run -p api_cli -- metrics
```

检查结果：
```sh
ls shots/*.png
tail -n 3 shots/.history/history.jsonl
cat shots/.history/seq.txt  # 跨进程序列持久化
```
序列持久化：`.history/seq.txt` 记录 `YYYYMMDD 最后序号`，重启后继续递增；跨日自动重置。

## 架构速览
1. Frame 捕获或构造
2. Annotation (z 排序)
3. SimpleRenderer CPU 合成
4. 导出编码 (PNG/JPEG)

## Roadmap (当前 & 未来)
- [x] Core/Renderer 基础
- [x] 基础标注：矩形 / 箭头 / 文本，支持撤销/重做
- [x] 高级标注：高亮 / 马赛克 / 自由手绘 / 虚线描边
- [x] PNG & JPEG 导出，质量控制
- [x] 交互式区域选择，自研 GUI 界面
- [x] 多显示器截图支持
- [x] 跨进程序列持久化
- [x] 历史系统含缩略图
- [x] 基础设施：指标、panic 处理、缓存
- [ ] 字体栅格化（fontdue 集成）
- [ ] 渲染快照基线测试
- [ ] DirtyRect / SIMD 优化
- [ ] OCR + 隐私自动区域建议
- [ ] GPU 后端原型
- [ ] Windows 平台实现
- [ ] Node.js API 绑定

## 许可证
MIT

Coder: GPT5/Claude 4
