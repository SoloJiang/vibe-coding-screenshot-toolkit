# 技术设计总览

本文件概述整体架构与当前实现范围；详细设计请查看各模块文档。

## 项目定位
专注于交互式框选截图工具，支持多显示器和跨显示器截图功能。

## 功能范围
- **交互式截图**：基于自研 UI overlay 的区域选择框架
- **多显示器支持**：检测并支持多个显示器环境
- **跨显示器选择**：支持选择跨越多个显示器的区域（规划中）
- **导出功能**：PNG 保存 & 剪贴板输出
- **命名模板**：支持 {date}、{seq} 等变量的文件命名

## 架构层次
1. **core**：纯模型+算法（Screenshot/Frame/命名模板）
2. **renderer**：像素合成（CPU RGBA）
3. **platform_mac**：macOS 交互式截图实现（基于 xcap + ui_overlay）
4. **services**：编排（export）
5. **api_cli**：CLI 接口（仅 capture-interactive 命令）
6. **infra**：通用设施（事件、配置、路径、LRU）
7. **ui_overlay**：交互框选 GUI（自研选择器）

## 当前约束
- 错误统一 core::ErrorKind 子集
- 线程模型：同步或轻量 tokio（无复杂并发）
- 仅支持交互式截图，移除全屏和区域截图等其他模式
- 专注 macOS 平台，Windows 平台为未来扩展预留

---
注：OCR/Privacy/GPU/SIMD/History/Upload 等扩展功能不在当前实现范围内。
