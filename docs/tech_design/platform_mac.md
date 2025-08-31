# platform_mac 模块技术设计

## 职责
 - 区域截图（临时方案）：`screencapture -R x,y,w,h` 封装 `MacCapturer::capture_region` (计划被自研 UI 裁剪替换)
 - 交互式框选（临时方案）：`screencapture -i` 封装 `MacCapturer::capture_region_interactive`（将废弃）

## MVP 子集
- 全屏捕获 (主显示器) - 基于 xcap + screencapture 回退
- 交互式命令取消原因区分（用户取消 vs 权限 vs 失败）
- 权限检测（失败返回错误，不做 UI 引导）

## 延后
窗口/延时/连续、ScreenCaptureKit 高级路径、快捷键注册、光标图像、窗口高亮。

## 捕获策略 (基于 XCap)
当前 MVP 实现：优先使用 `xcap` (基于 CoreGraphics) 获取主显示器图像；失败（权限/内部错误）自动回退 macOS 系统命令 `screencapture -x` 生成临时 PNG 后解码为 RGBA。已提供多显示器初步支持：`capture_all()` 遍历 `Monitor::all()`，对每个显示器单独生成 `Screenshot`。

**XCap 技术优势：**
- 成熟稳定的 Rust 生态库
- 基于 CoreGraphics，性能优秀
- 良好的多显示器支持
- 社区活跃，维护良好

后续阶段：
1. 尝试引入 ScreenCaptureKit (macOS 12.3+) 以支持窗口/区域/高性能连续捕获。
2. 多显示器：当前为"分别输出"模式；后续可增加"合并拼接"模式（按照物理坐标系构建单一 FrameSet）。
3. 零拷贝：评估 CVPixelBuffer -> 自定义 Frame 引用。
4. 自研框选 UI：通过一次全屏捕获后在内存裁剪，减少多次系统调用开销，并允许实时预览与标注。

## 组件
MacScreenCapturer, DisplayInfoProvider, ClipboardMac, HotkeyRegistrarMac, PermissionsMac。

## 错误映射
权限不足 -> E_NO_PERMISSION；捕获失败 -> E_CAPTURE_FAIL。

## 风险
| 风险 | 缓解 |
|------|------|
| 权限阻塞 | 首次失败提示用户授权；授权后重试 |
| API 差异 | 运行时探测：xcap -> screencapture -> (未来) ScreenCaptureKit |
| 临时文件残留 | 使用 tempfile 自动清理 |
| 多显示器尺寸/scale | 记录 scale (当前固定 1.0) 后续补充 |
