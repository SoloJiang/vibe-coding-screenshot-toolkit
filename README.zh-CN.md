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
  core/ infra/ renderer/ services/
  api_cli/ api_napi/ platform_mac/ platform_win/
  ocr_adapter/ privacy/ macros/
```
文档：`docs/prd` 产品，`docs/tech_design` 技术设计，`docs/todo` 模块任务。

## 快速开始
```sh
cargo test --workspace
cargo build -p renderer
```

## 架构速览
1. Frame 捕获或构造
2. Annotation (z 排序)
3. SimpleRenderer CPU 合成
4. 导出编码 (PNG/JPEG)

## Roadmap (节选)
- [x] Core/Renderer 基础
- [x] 高亮 / 箭头 / 马赛克 / 虚线 / 自由手绘 / JPEG
- [ ] 字体栅格 (fontdue)
- [ ] 渲染快照基线测试
- [ ] DirtyRect / SIMD
- [ ] OCR + 隐私自动区域
- [ ] GPU 后端

## 许可证
MIT

Coder: GPT5/Claude 4
