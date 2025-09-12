# Screenshot Toolkit

> 多平台（macOS，Windows 进行中）截图与标注引擎。聚焦可靠捕获、可预测渲染与高效导出。

[English README](./README.md)

## 已实现特性
- 标注：矩形 / 箭头（含虚线）/ 文本占位；预置类型（高亮/马赛克/自由手绘）已在模型层就绪
- 渲染：CPU RGBA 合成，混合模式 Multiply / Screen，虚线、描边、平滑
- 导出：PNG（内置 JPEG 编码器），macOS 剪贴板复制
- 撤销/重做：合并策略，UUID v7 序列

## 目录结构
```
crates/
  core/           # 核心模型、标注类型、撤销栈、命名模板
  infra/          # 基础设施：指标、panic 处理、LRU 缓存、路径解析
  renderer/       # CPU RGBA 合成、混合模式、形状渲染
  services/       # 业务逻辑编排（捕获、标注、导出、历史）
  platform_mac/   # macOS 捕获（基于 xcap 0.7+）、剪贴板集成
  platform_win/   # Windows 捕获（占位/桩实现）
  ui_overlay/     # 自研 GUI 区域选择器
  api_cli/        # CLI 接口，含截图命令和交互式选择
  api_napi/       # Node.js 绑定（规划中）
  ocr_adapter/    # OCR 集成（规划中）
  privacy/        # 隐私扫描与遮罩（规划中）
  macros/         # 派生宏（规划中）
```
文档：`docs/prd` MVP 范围与验收；`docs/tech_design` 技术设计；`docs/todo` 模块任务。

## 快速开始
```sh
cargo test --workspace
cargo build -p renderer
```

### 状态
已完成捕获 → 标注 → 渲染 → 导出闭环；提供自研 GUI 交互式区域选择。
- 截图：全屏与区域（macOS 基于 xcap；多显示器 `--all`）
- 交互选择：自研 GUI Overlay 选区
- 标注：矩形 / 箭头 / 文本，支持撤销/重做与图层顺序
- 导出：PNG 文件与 macOS 剪贴板（NSPasteboard）；可用 JPEG 编码器
- 命名：`{date},{seq},{screen}`，按天序列持久化
- 历史：最近 50 条（JSONL + 缩略图，自动裁剪）
- CLI：`capture`、`capture-region`、`capture-interactive`、`metrics`
- 基础：指标、panic、LRU、路径解析

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

交互式选择：
```sh
cargo run -p api_cli -- capture-interactive -d shots
# 使用 GUI 选择器选择区域并保存 PNG
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

## 路线图
- [x] Core/Renderer 基础
- [x] 基础标注：矩形 / 箭头 / 文本，支持撤销/重做
- [x] 高级标注：高亮 / 马赛克 / 自由手绘 / 虚线描边
- [x] PNG & JPEG 导出，质量控制
- [x] 多显示器截图支持
- [x] 跨进程序列持久化
- [x] 历史系统含缩略图
- [x] 基础设施：指标、panic 处理、缓存
- [ ] 交互式区域选择 GUI
- [ ] 字体栅格化（fontdue 集成）
- [ ] 渲染快照基线测试
- [ ] DirtyRect / SIMD 优化
- [ ] OCR + 隐私自动区域建议

## 贡献
1. 变更架构时同步更新 `docs/tech_design/*.md`。
2. 为渲染行为变更补充/修正测试。
3. 保持公共 API 精简稳定；标注 feature flags。

## 许可证
MIT
