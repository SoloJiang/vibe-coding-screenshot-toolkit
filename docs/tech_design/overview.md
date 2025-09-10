# 技术设计总览

本文件概述整体架构与当前（MVP）实现范围；详细设计请查看各模块文档。

## MVP 范围
与 `docs/prd/mvp.md` 一致：
- 捕获：全屏 + 内存裁剪（临时）；区域选择由自研框选 UI 提供（不依赖系统 `screencapture`）
- 标注：Rect / Arrow / Text + Undo/Redo + 简单图层顺序
- 导出：PNG 保存 & 剪贴板 + 命名模板
- 历史：最近 50 条（路径+缩略图）加载/裁剪
- CLI：capture / capture-region
 - 序列：命名模板 {seq} 跨进程持久化 (.history/seq.txt) 保证同日连续递增
 - CLI 辅助：--mock 选项供无权限/测试环境跳过真实屏幕捕获

注：模型中预置的 Highlight / Mosaic / Freehand / JPEG 编码器不属于当前验收范围。

## 架构层次
1. core：纯模型+算法（Annotation/Undo/命名模板/History 裁剪）
2. renderer：像素合成（CPU RGBA）
3. platform_*：平台捕获/剪贴板（macOS 基于 xcap）
4. services：编排（capture/annotate/export/history）
5. api_cli / api_napi：接口层（当前仅 CLI）
6. infra：通用设施（事件、配置、路径、LRU）
7. ui_overlay：交互框选（自研 GUI 选择器）

## MVP 期间约束
- 错误统一 core::ErrorKind 子集
- 线程模型：同步或轻量 tokio（无复杂并发）
- 持久化：History 追加写 JSON 行
 - 序列持久化：每输出目录 .history/seq.txt 记录 `YYYYMMDD last_seq`，启动时载入；跨日自动重置

---
注：OCR/Privacy/GPU/SIMD/DirtyRect/上传等扩展不在本文档范围内。
