# platform_mac 模块技术设计

## 职责
 - 区域截图：通过一次全屏捕获后在内存裁剪（配合自研 UI 提供的矩形），不依赖系统命令

## 能力
- 全屏捕获（主显示器）基于 xcap（不回退 screencapture），当前依赖 xcap 0.7
- 多显示器：遍历 `Monitor::all()`，逐一产出 `Screenshot`
- 剪贴板：写入 PNG（NSPasteboard）

注：窗口/延时/连续捕获、ScreenCaptureKit 路径、全局快捷键、窗口高亮等不在当前范围。

## 捕获策略 (基于 XCap)
当前实现：使用 `xcap` (基于 CoreGraphics) 获取主显示器图像；若失败则返回错误（不再回退到 `screencapture`）。已提供多显示器支持：`capture_all()` 遍历 `Monitor::all()`，对每个显示器单独生成 `Screenshot`。在 0.7 版本中，`Monitor::is_primary()` 返回 `Result<bool, _>`，代码中采用 `unwrap_or(false)` 进行安全默认处理。

**xcap 路径说明：**
- 成熟稳定的 Rust 生态库
- 基于 CoreGraphics，性能优秀
- 良好的多显示器支持
- 社区活跃，维护良好

本实现以 xcap（CoreGraphics）为主路径；失败时返回错误，不回退到 `screencapture`。

## 组件
MacCapturer, MacClipboard。

## 错误映射
权限不足 -> E_NO_PERMISSION；捕获失败 -> E_CAPTURE_FAIL。

## 风险
| 风险 | 缓解 |
|------|------|
| 权限阻塞 | 首次失败提示用户授权；授权后重试 |
| API 差异 | 运行时探测：xcap -> (未来) ScreenCaptureKit |
| 临时文件残留 | 使用 tempfile 自动清理 |
| 多显示器尺寸/scale | 记录 scale (当前固定 1.0) 后续补充 |
