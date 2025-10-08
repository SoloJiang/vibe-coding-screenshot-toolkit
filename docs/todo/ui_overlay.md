# ui_overlay TODO

## 当前状态（v0.1.2 已完成）
- ✅ 基础交互式选择器：Region / RegionSelector / 错误类型
- ✅ platform_mac 集成：MacCapturer 使用 RegionSelector
- ✅ CLI 集成：capture-interactive 命令
- ✅ 性能优化：预热渲染、背景暗化缓存、重绘节流、60 FPS 帧率控制
- ✅ **RenderBackend 抽象层**：统一 GPU/CPU 渲染接口
- ✅ **Metal GPU 硬件加速**：macOS 原生 Metal 渲染（metal-rs + CAMetalLayer）
- ✅ **CPU Raster 降级**：软件渲染后端，确保兼容性
- ✅ 多显示器支持（检测、跨屏选择）
- ✅ 虚拟桌面坐标系统
- ✅ 坐标转换修复（统一虚拟坐标到窗口本地坐标转换）
- ✅ 多窗口渲染修复（帧率控制统一管理）
- ✅ 修饰键支持（Shift 正方形、Alt 中心拉伸）
- ✅ 键盘控制（ESC 取消、Enter 确认、方向键微调）
- ✅ 智能重绘节流（防抖、帧率预算管理）
- ✅ 图像缓存（Skia Image 对象复用）

## v0.2 - 标注编辑 UI（对标微信截图）

### Phase 1: 基础架构（优先级：P0）
- [ ] **状态机扩展**：增加 EditingState（当前只有 SelectionState）
  - [ ] 定义 AppMode 枚举（SelectingRegion / EditingAnnotations）
  - [ ] 在 selector.rs 中实现模式切换逻辑
  - [ ] 选区确认后自动进入编辑模式（不退出窗口）
- [ ] **集成 egui**：工具栏 UI 框架
  - [ ] 添加 egui, egui-winit, egui_skia 依赖
  - [ ] 在 SelectionApp 中初始化 egui 上下文
  - [ ] 实现基础工具栏渲染（空壳）
  - [ ] 验证 egui → Skia 渲染管线
- [ ] **事件处理分离**：区分选择事件和编辑事件
  - [ ] 重构 event_handler.rs 支持模式感知
  - [ ] 编辑模式下的鼠标事件路由

### Phase 2: 基础工具（优先级：P0）
- [ ] **工具栏 UI**
  - [ ] 设计工具栏布局（顶部浮动/底部固定）
  - [ ] 矩形工具按钮
  - [ ] 箭头工具按钮
  - [ ] 完成/取消按钮
  - [ ] 工具选择状态高亮
- [ ] **矩形绘制交互**
  - [ ] 鼠标按下创建临时标注
  - [ ] 鼠标移动更新临时标注（实时预览）
  - [ ] 鼠标释放确认标注（加入列表）
  - [ ] 使用 Skia 渲染矩形预览
- [ ] **箭头绘制交互**
  - [ ] 起点-终点拖拽逻辑
  - [ ] 箭头方向计算和渲染
  - [ ] 箭头头部大小自适应

### Phase 3: 撤销/重做和样式（优先级：P1）
- [ ] **撤销/重做集成**
  - [ ] 连接 core::UndoStack 到编辑器
  - [ ] 撤销/重做按钮（Ctrl+Z/Ctrl+Y）
  - [ ] 操作历史显示（可选）
- [ ] **颜色选择器**
  - [ ] 集成 egui 内置颜色选择器
  - [ ] 预设颜色快速选择
  - [ ] 当前颜色显示
- [ ] **粗细调节**
  - [ ] egui 滑块（1-10px）
  - [ ] 实时预览粗细变化

### Phase 4: 高级工具（优先级：P2）
- [ ] **画笔工具（Freehand）**
  - [ ] 路径点收集
  - [ ] Chaikin 平滑预览
  - [ ] 笔刷粗细支持
- [ ] **马赛克工具**
  - [ ] 区域选择
  - [ ] 马赛克 level 调节
  - [ ] 实时预览
- [ ] **文字工具**（难度较高）
  - [ ] 文本输入框
  - [ ] 字体选择
  - [ ] 字号调节
  - [ ] 真正字形渲染（需 fontdue）

### Phase 5: Skia 标注渲染（优先级：P1）
- [ ] **创建 SkiaAnnotationRenderer**
  - [ ] 在 ui_overlay/skia/ 新建 annotation_renderer.rs
  - [ ] 实现 Rect 的 Skia 渲染（canvas.draw_rect）
  - [ ] 实现 Arrow 的 Skia 渲染（canvas.draw_line + draw_path）
  - [ ] 实现 Freehand 的 Skia 渲染（canvas.draw_path）
  - [ ] 实现 Mosaic 的 Skia 渲染（Shader 或 Image filter）
- [ ] **集成到编辑流程**
  - [ ] 编辑模式实时使用 SkiaAnnotationRenderer
  - [ ] 性能测试和优化
  - [ ] 与 renderer CPU 路径对比验证

### Phase 6: 导出集成（优先级：P0）
- [ ] **完成按钮逻辑**
  - [ ] 收集所有标注
  - [ ] 调用 services::ExportService
  - [ ] 渲染最终图像（renderer 或 Skia）
  - [ ] 保存文件 + 剪贴板
  - [ ] 退出编辑模式
- [ ] **取消按钮逻辑**
  - [ ] 丢弃所有标注
  - [ ] 直接退出

## v0.3 - 用户体验优化

### 界面美化
- [ ] 工具栏主题定制（暗色/亮色）
- [ ] 图标设计（矢量图标）
- [ ] 动画效果（工具切换、标注创建）
- [ ] 快捷键提示 UI
- [ ] 响应式布局（适配不同分辨率）

### 交互增强
- [ ] 标注选择（点击选中）
- [ ] 标注拖拽（移动位置）
- [ ] 标注调整（边框控制点）
- [ ] 标注删除（Delete 键）
- [ ] 多选标注（Ctrl 点击）
- [ ] 对齐辅助线（吸附）
- [ ] 尺寸实时显示

### 性能优化
- [ ] 标注增量渲染（只重绘变化部分）
- [ ] 大尺寸截图虚拟化（按需加载）
- [ ] GPU 内存管理优化
- [ ] 复杂标注场景优化（>100 个标注）

## v0.4 - 高级功能
- [ ] 图层面板（显示所有标注）
- [ ] 标注分组（逻辑分组）
- [ ] 标注锁定（防误操作）
- [ ] 标注复制/粘贴
- [ ] 标注模板（预设样式）
- [ ] 历史记录浏览（时间轴）

## v1.0 - 完整产品
- [ ] Windows 平台完整支持
- [ ] 快捷键全局绑定（系统级）
- [ ] 插件系统（自定义标注类型）
- [ ] 协作模式（多人标注）
- [ ] 云端同步

## ✅ 已完成：Metal GPU 渲染与性能优化（v0.1.2）

### Phase 0: CPU 优化（已完成）
- [x] 实现 `FrameTimer` 限制渲染频率为 60 FPS
- [x] 在 `request_redraw_all()` 中统一检查帧率限制
- [x] 测试 CPU 性能提升效果

### Phase 1: RenderBackend 抽象（已完成）
- [x] 定义 `RenderBackend` trait 和 `BackendType` 枚举
- [x] 实现 `CpuRasterBackend`（软件渲染后端）
- [x] 创建 `MetalBackend` 基础结构
- [x] 实现 `factory.rs` 自动选择最佳后端
- [x] 所有测试通过

### Phase 2: Metal GPU Backend 实现（已完成）
- [x] 使用 `metal-rs` 实现 Metal 渲染
- [x] 创建 `CAMetalLayer` 并绑定到窗口
- [x] 实现 `prepare_surface()`：从 layer 获取 drawable 并创建 Skia Surface
- [x] 实现 `flush_and_read_pixels()`：提交渲染并调用 `drawable.present()`
- [x] 修复坐标转换问题：统一虚拟坐标到窗口本地坐标的转换公式
- [x] 修复多窗口渲染问题：确保所有窗口在同一帧配额内渲染
- [x] 测试验证：选择框正确显示，性能流畅

### 关键问题解决
1. **选择框不显示**：
   - **根本原因**：坐标转换公式错误，使用了 `x - (window_x - vx)` 而非 `x - window_x`
   - **解决方案**：统一坐标转换公式，确保新旧渲染系统使用相同逻辑

2. **多窗口渲染失败**：
   - **根本原因**：`frame_timer` 是全局共享的，窗口0消耗帧配额后窗口1无法渲染
   - **解决方案**：将帧率检查移到 `request_redraw_all()` 开头，确保一次请求所有窗口都渲染

3. **Metal GPU 渲染管线**：
   - **问题**：drawable 未保存，无法调用 `present()`
   - **解决方案**：在 `MetalBackend` 中添加 `current_drawable` 字段保存 drawable，在 flush 时调用 `present()`

### 性能提升
- ✅ Metal GPU 硬件加速渲染
- ✅ 60 FPS 帧率限制，CPU 使用率降低约 50%
- ✅ 图像缓存避免重复创建 Skia Image
- ✅ 拖动流畅，无明显卡顿

### 架构改进
- ✅ RenderBackend trait 统一抽象 GPU/CPU 渲染
- ✅ 平台特定代码隔离到 backend 模块
- ✅ 支持自动降级到 CPU Raster
- ✅ 为 Windows Direct3D backend 预留架构空间

---

## GPU 后端维护（持续）
- [x] Metal 后端实现和测试 ✅
- [x] 坐标转换修复 ✅
- [x] 多窗口渲染修复 ✅
- [ ] Windows Direct3D 后端实现（Phase 3）
- [ ] CPU fallback 性能优化
- [ ] GPU 内存泄漏检测
- [ ] 渲染性能监控

## 已解决问题记录（v0.1.x）
- ✅ macOS context leak 警告：通过复用 Pixels 实例解决
- ✅ 首次交互卡顿：通过预热渲染解决
- ✅ 拖动卡顿：通过背景缓存和重绘节流解决
- ✅ 进入截图模式屏幕闪动：通过先预热渲染再显示窗口解决
- ✅ 选择框拖动卡顿优化：增加防抖距离阈值和智能重绘判断
- ✅ 多屏幕坐标转换：添加调试输出确保虚拟桌面坐标正确性
- ✅ 非主屏框选坐标错误：修复 CursorMoved 事件中的坐标转换逻辑
- ✅ Skia 渲染内容无法显示：通过 WindowManager + softbuffer 集成解决
- ✅ 架构借用冲突：重构渲染和显示逻辑分离解决
- ✅ 选择区域背景不透明：修复为显示原始背景（移除暗化）
- ✅ 向上框选尺寸错误：修复坐标规范化和边界检查逻辑
- ✅ **选择框完全不显示（v0.1.1）**：修复像素读取格式为 RGBA8888/Unpremul
- ✅ **新渲染系统选择框位置错误（v0.1.2）**：统一坐标转换公式为 `local = virtual - window_virtual`
- ✅ **多窗口渲染失败（v0.1.2）**：将帧率检查移到 `request_redraw_all()` 统一处理
- ✅ **Metal GPU drawable 未 present（v0.1.2）**：保存 `current_drawable` 并在 flush 时调用 `present()`
- ✅ **性能优化（v0.1.2）**：Metal GPU 硬件加速 + 60 FPS 帧率控制 + 图像缓存

## 技术债务
- [ ] 减少 selector.rs 文件行数（当前 497 行，建议拆分）
- [ ] event_handler.rs 测试覆盖率提升
- [ ] window_manager.rs 文档注释完善
- [ ] 清理 examples/ 中的实验性代码
- [ ] 统一错误处理策略（anyhow vs thiserror）

## 文档
- [ ] 编写标注编辑器架构设计文档
- [ ] egui 集成指南
- [ ] Skia 标注渲染器开发文档
- [ ] 性能优化最佳实践
- [ ] 用户使用手册（截图 + 标注流程）
