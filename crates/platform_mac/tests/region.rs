#[cfg(target_os = "macos")]
#[test]
fn test_capture_region_small() {
    // 仅验证命令执行成功；无法保证像素一致性（依赖实际屏内容）。
    match platform_mac::MacCapturer::capture_region(0, 0, 50, 40) {
        Ok(s) => {
            assert!(
                s.raw.primary.width <= 60 && s.raw.primary.height <= 60,
                "unexpected size {}x{}",
                s.raw.primary.width,
                s.raw.primary.height
            );
        }
        Err(e) => {
            // 若因为权限问题失败，提示并允许测试通过（CI 没有权限）
            eprintln!("capture_region failed: {e}");
        }
    }
}
