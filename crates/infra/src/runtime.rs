/// 异步运行时管理模块
///
/// 提供全局 tokio runtime 管理和混合并发工具函数
use std::future::Future;

/// 在 tokio 的 blocking pool 中执行 CPU 密集型的 rayon 任务
///
/// 这个函数桥接了 tokio 的异步世界和 rayon 的并行世界:
/// - 从 tokio 的角度看，这是一个异步任务
/// - 从 rayon 的角度看，这是一个普通的同步任务
///
/// # 使用场景
/// - 在异步上下文中需要执行 CPU 密集型的并行计算
/// - 避免阻塞 tokio 的 runtime
///
/// # 示例
/// ```no_run
/// use infra::runtime::spawn_blocking_rayon;
///
/// async fn process_image(data: Vec<u8>) -> anyhow::Result<Vec<u8>> {
///     spawn_blocking_rayon(move || {
///         // 这里可以使用 rayon 的并行迭代器
///         use rayon::prelude::*;
///         data.par_iter().map(|&x| x * 2).collect()
///     }).await
/// }
/// ```
pub async fn spawn_blocking_rayon<F, T>(f: F) -> Result<T, tokio::task::JoinError>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f).await
}

/// 配置 rayon 线程池的 CPU 核心数
///
/// 如果不调用此函数，rayon 会使用默认配置（通常是 CPU 核心数）
///
/// # 参数
/// - `num_threads`: 线程池大小。如果为 None，使用默认值（CPU 核心数）
///
/// # 注意
/// 此函数必须在任何 rayon 操作之前调用，建议在程序启动时配置
pub fn configure_rayon_pool(num_threads: Option<usize>) -> Result<(), rayon::ThreadPoolBuildError> {
    let mut builder = rayon::ThreadPoolBuilder::new();

    if let Some(n) = num_threads {
        builder = builder.num_threads(n);
    }

    builder.build_global()
}

/// 获取当前 rayon 线程池的线程数
pub fn rayon_thread_count() -> usize {
    rayon::current_num_threads()
}

/// 运行一个需要 tokio runtime 的异步任务
///
/// 这个函数用于在没有 tokio runtime 的上下文中执行异步代码
/// 例如在测试或者同步函数中调用异步 API
///
/// # 注意
/// - 如果已经在 tokio runtime 中，应该直接使用 `.await`
/// - 这个函数会阻塞当前线程直到异步任务完成
pub fn block_on<F>(future: F) -> F::Output
where
    F: Future,
{
    tokio::runtime::Runtime::new()
        .expect("Failed to create tokio runtime")
        .block_on(future)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn_blocking_rayon() {
        let result = spawn_blocking_rayon(|| {
            use rayon::prelude::*;
            (0..100).into_par_iter().sum::<i32>()
        })
        .await
        .unwrap();

        assert_eq!(result, 4950);
    }

    #[test]
    fn test_rayon_thread_count() {
        let count = rayon_thread_count();
        assert!(count > 0, "Rayon should have at least 1 thread");
    }

    #[test]
    fn test_block_on() {
        let result = block_on(async {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            42
        });
        assert_eq!(result, 42);
    }
}
