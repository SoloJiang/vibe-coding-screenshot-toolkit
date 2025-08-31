use std::panic::{self, PanicHookInfo};
use std::sync::Once;

static INIT: Once = Once::new();

/// 安装一个简单 panic hook：打印线程名、payload、回溯（debug 构建下）。
pub fn install() {
    INIT.call_once(|| {
        let default = panic::take_hook();
        panic::set_hook(Box::new(move |info: &PanicHookInfo| {
            eprintln!(
                "[panic] thread={:?} {}",
                std::thread::current().name(),
                format_panic(info)
            );
            #[cfg(debug_assertions)]
            {
                let bt = std::backtrace::Backtrace::force_capture();
                eprintln!("{bt}");
            }
            // 仍调用默认 hook 以保留原行为（例如 abort/backtrace 配置）
            default(info);
        }));
    });
}

fn format_panic(info: &PanicHookInfo) -> String {
    let loc = info.location();
    let location = loc
        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
        .unwrap_or_else(|| "<unknown>".into());
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        format!("{s} @ {location}")
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        format!("{s} @ {location}")
    } else {
        format!("(non-string panic) @ {location}")
    }
}
