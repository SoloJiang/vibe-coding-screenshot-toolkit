# ui_overlay TODO

## MVP
- [x] 定义基础 API：Region / RegionSelector / 错误类型
- [x] 集成 platform_mac：`MacCapturer::capture_region_interactive_custom` 使用 `RegionSelector`
- [x] CLI 接线：`capture-interactive` 调用 `create_gui_region_selector`
- [ ] 集成 services：RegionSelectService 使用 ui_overlay（后续 PR）
- [ ] 为 renderer/export 提供区域像素裁剪适配（由 services 完成）

## 细化与增强
- [x] 半透明蒙层 + 描边绘制（pixels）
- [x] 移除 mac_selector，统一 `selector`（winit + pixels）实现
- [x] 兼容 winit 0.30：完成迁移到 ApplicationHandler/run_app，消除旧 API 警告；消除了 Pixels 生命周期/自引用问题
- [x] 键盘快捷：Shift 固定比例、Alt 从中心、方向键 n 像素移动、ESC 取消
- [x] 平台胶水隔离：新增 `platform` 模块封装 macOS 呈现设置（隐藏菜单栏/Dock），核心选择器减少条件编译
- [ ] 多显示器支持：选择跨屏坐标与 scale 处理
- [ ] 吸附：对齐屏幕边/网格/窗口（可选）
- [ ] 可配置主题/颜色/线宽
- [ ] 点击穿透/置顶策略（Win 专项）

## 文档/测试
- [x] 技术设计文档 `docs/tech_design/ui_overlay.md`
- [x] 更新文档以反映 `platform` 模块抽象
- [x] README 更新（根与 crate）
- [x] 示例删除与 Changelog 记录（内部记录）

## 已解决问题记录
- [x] macOS “Context leak detected, msgtracer returned -1” 日志反复出现：
	- 现象：运行交互截图时系统日志持续输出 context leak 警告，且最终交互返回取消。
	- 根因：每帧创建 Pixels/SurfaceTexture 导致 CA/Metal 上下文频繁构造，AppKit 检测到资源未及时释放。
	- 修复：窗口创建后构建并复用单个 Pixels；窗口尺寸变化时 resize_surface；字段顺序保证 drop 次序先 Pixels 再 Window。使用 Box<Window> 与受控 'static 引用，确保生命周期安全。

- [x] 首次交互卡顿明显：
	- 现象：第一次触发框选时卡顿，选择框出现后流畅。
	- 根因：首次交互触发时才初始化 pixels/wgpu 与首次渲染。
 	- 修复：窗口初始不可见；在 resumed 中创建窗口后立即 ensure_pixels 并 render_once 预热，再显示窗口；移除空闲时的持续 request_redraw，改为按需重绘。
