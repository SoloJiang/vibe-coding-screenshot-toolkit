# 技术设计总览 (更新版)

本文件概述整体架构，并标记当前阶段 (MVP) 的聚焦范围与后续扩展位置。详细设计请查看各模块文档。

## MVP 对齐
对应 `docs/prd/mvp.md`，MVP 只实现：
- 捕获：全屏 + 内存裁剪（临时）；区域选择由自研框选 UI 提供（不依赖系统 `screencapture`）
- 标注：Rect / Arrow / Text + Undo/Redo + 简单图层顺序
- 导出：PNG 保存 & 剪贴板 + 命名模板
- 历史：最近 50 条（路径+缩略图）加载/裁剪
- CLI：capture / capture-region
 - 序列：命名模板 {seq} 跨进程持久化 (.history/seq.txt) 保证同日连续递增
 - CLI 辅助：--mock 选项供无权限/测试环境跳过真实屏幕捕获

> 代码中预埋但非 MVP 验收范围：Highlight / Mosaic / Freehand / JPEG 导出；视为后续扩展，不影响当前验收。

非目标（延后）：OCR、隐私、上传、Hook、延时/窗口/连续、Mosaic/Highlight/Freehand、JPEG 优化、GPU/SIMD、DirtyRect、设置/快捷键。

## 架构层次
1. core：纯模型+算法（Annotation/Undo/命名模板/History 裁剪）
2. renderer：像素合成（CPU RGBA）
3. platform_*：平台捕获/剪贴板（基于 xcap，不再回退 screencapture）
4. services：编排（capture/annotate/export/history）
5. api_cli / api_napi：接口层（MVP 仅 CLI）
6. infra：通用设施（事件、配置、路径、LRU）
7. ui_overlay：交互框选/实时标注入口（接口已预置，GUI 实现作为后续里程碑）

## MVP 期间约束
- 错误统一 core::ErrorKind 子集
- 线程模型：同步或轻量 tokio（无复杂并发）
- 持久化：History 追加写 JSON 行
 - 序列持久化：每输出目录 .history/seq.txt 记录 `YYYYMMDD last_seq`，启动时载入；跨日自动重置

## 里程碑衔接
MVP 完成后，依次引入：跨平台框选 UI / 窗口/延时/连续 -> OCR/Privacy -> Upload/Markdown/Hook -> 性能优化 (DirtyRect/SIMD/GPU) -> 扩展生态 (宏, 动态 AnnotationKind)。

---
> 后续：完整版技术点请参考各模块 `tech_design/*.md` 与 TODO 文件。
