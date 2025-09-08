# platform_mac 模块 todo

## MVP
- ✅ 全屏捕获(主显示器)：xcap
- ✅ 多显示器初步支持：capture_all() 分别输出多文件 (CLI --all)
- 待做：权限检测 ensure_screen_capture（当前仅错误信息提示）
- ✅ 剪贴板写入 PNG
- ✅ 自研框选 UI 集成：一次全屏 + 交互返回 Region 后内存裁剪

## m2
- [ ] ScreenCaptureKit 检测与集成
- [ ] 窗口捕获 (基于 xcap + ScreenCaptureKit)
- [x] 自研框选 UI 集成 (替换 screencapture -R/-i) 基础：矩形选择 + Esc 取消
- [ ] 多显示器坐标统一 & scale
- [ ] 快捷键注册 (全局启动框选)
- [ ] 权限检测 ensure_screen_capture

## m3
- [ ] CVPixelBuffer 零拷贝优化
- [ ] xcap 性能优化和内存管理
- [ ] 框选 UI 增强：实时缩放镜、尺寸标尺、连按空格移动矩形

## m4
- [ ] 光标捕获 (基于 xcap)

## 持续
- [ ] 错误码单测
- [ ] 文档注释
- [ ] XCap 版本跟踪和更新
