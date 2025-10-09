/// 帧率控制器
///
/// 用于限制渲染帧率，避免过度绘制
use std::time::{Duration, Instant};

pub struct FrameTimer {
    last_frame: Instant,
    target_frame_time: Duration,
    frame_count: u64,
    total_time: Duration,
}

impl FrameTimer {
    /// 创建新的帧率控制器
    ///
    /// target_fps: 目标帧率 (例如 60)
    pub fn new(target_fps: u32) -> Self {
        let target_frame_time = Duration::from_micros(1_000_000 / target_fps as u64);

        Self {
            last_frame: Instant::now(),
            target_frame_time,
            frame_count: 0,
            total_time: Duration::ZERO,
        }
    }

    /// 检查是否应该渲染新的一帧
    ///
    /// 如果距离上一帧时间 >= 目标帧时间，返回 true
    pub fn should_render(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame);

        if elapsed >= self.target_frame_time {
            self.last_frame = now;
            self.frame_count += 1;
            self.total_time += elapsed;
            true
        } else {
            false
        }
    }

    /// 强制渲染（跳过帧率限制）
    #[cfg(test)]
    pub fn force_render(&mut self) {
        self.last_frame = Instant::now();
        self.frame_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_frame_timer_basic() {
        let mut timer = FrameTimer::new(60); // 60 FPS

        // 第一帧应该立即渲染
        assert!(timer.should_render());

        // 立即检查应该不渲染（帧时间未到）
        assert!(!timer.should_render());

        // 等待足够时间后应该渲染
        sleep(Duration::from_millis(17)); // ~60 FPS
        assert!(timer.should_render());
    }

    #[test]
    fn test_force_render() {
        let mut timer = FrameTimer::new(60);

        timer.should_render(); // 第一帧

        // 立即强制渲染
        timer.force_render();

        // 帧计数应该增加
        assert_eq!(timer.frame_count, 2);
    }
}
