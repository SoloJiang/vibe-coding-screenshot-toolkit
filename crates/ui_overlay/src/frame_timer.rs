/// 帧率控制器
///
/// 用于限制渲染帧率到 60 FPS，避免过度绘制导致性能问题
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
    /// # 参数
    /// * `target_fps` - 目标帧率（例如 60）
    pub fn new(target_fps: u32) -> Self {
        let target_frame_time = Duration::from_micros(1_000_000 / target_fps as u64);

        Self {
            // 初始化为已经过去一帧时间，确保第一帧可以立即渲染
            last_frame: Instant::now() - target_frame_time,
            target_frame_time,
            frame_count: 0,
            total_time: Duration::ZERO,
        }
    }

    /// 检查是否应该渲染新的一帧
    ///
    /// 如果距离上一帧时间 >= 目标帧时间，返回 true 并更新计数器
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

}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_frame_timer_basic() {
        let mut timer = FrameTimer::new(60); // 60 FPS

        // 第一帧应该立即可以渲染（因为初始化时已经减去了一个帧时间）
        assert!(timer.should_render());

        // 立即检查应该不渲染（帧时间未到）
        assert!(!timer.should_render());

        // 等待足够时间后应该渲染
        sleep(Duration::from_millis(17)); // ~60 FPS
        assert!(timer.should_render());
    }

    #[test]
    fn test_frame_timer_throttling() {
        let mut timer = FrameTimer::new(60);

        let first_render = timer.should_render();
        assert!(first_render);

        // 立即再次检查，应该被节流
        let second_render = timer.should_render();
        assert!(!second_render);
    }
}
