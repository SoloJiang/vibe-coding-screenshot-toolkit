use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use regex::Regex;
use std::sync::atomic::{AtomicU32, Ordering};

static SEQ_DAY: RwLock<Option<String>> = RwLock::new(None);
static SEQ_COUNTER: AtomicU32 = AtomicU32::new(0);

/// 重置当天序列（测试或跨日检测时调用）
pub fn reset_sequence_for(date: &str) {
    {
        let mut guard = SEQ_DAY.write();
        *guard = Some(date.to_string());
    }
    SEQ_COUNTER.store(0, Ordering::Relaxed);
}

pub fn next_seq(now: DateTime<Utc>) -> u32 {
    let day_key = now.format("%Y%m%d").to_string();
    let mut need_reset = false;
    {
        let guard = SEQ_DAY.read();
        match guard.as_ref() {
            Some(d) if d == &day_key => {}
            _ => need_reset = true,
        }
    }
    if need_reset {
        let mut guard = SEQ_DAY.write();
        *guard = Some(day_key);
        SEQ_COUNTER.store(0, Ordering::Relaxed);
    }
    SEQ_COUNTER.fetch_add(1, Ordering::Relaxed) + 1
}

/// 设置指定日期下当前序列值（用于跨进程持久化恢复）。
/// 传入的 value 表示"已使用的最后一个值"，下一次 next_seq 会在其基础上 +1。
pub fn set_sequence_for(date: &str, value: u32) {
    {
        let mut guard = SEQ_DAY.write();
        *guard = Some(date.to_string());
    }
    SEQ_COUNTER.store(value, Ordering::Relaxed);
}

/// 当前（已分配的）序列值（未调用 next_seq 返回 0）。
pub fn current_sequence() -> u32 {
    SEQ_COUNTER.load(Ordering::Relaxed)
}

/// 解析命名模板，占位符：{date:FORMAT} {seq} {screen}
pub fn parse_naming_template(tpl: &str, screen_index: usize, now: DateTime<Utc>) -> String {
    let mut out = String::with_capacity(tpl.len() + 16);
    let re = Regex::new(r"\{([^{}:]+)(?::([^{}]+))?\}").expect("regex");
    let mut last = 0;
    for cap in re.captures_iter(tpl) {
        // 安全：regex 捕获的第 0 组（完整匹配）总是存在
        let m = match cap.get(0) {
            Some(m) => m,
            None => continue,
        };
        out.push_str(&tpl[last..m.start()]);
        let key = &cap[1];
        match key {
            "date" => {
                let fmt = cap.get(2).map(|v| v.as_str()).unwrap_or("yyyyMMdd-HHmmss");
                let chrono_fmt = fmt
                    .replace("yyyy", "%Y")
                    .replace("MM", "%m")
                    .replace("dd", "%d")
                    .replace("HH", "%H")
                    .replace("mm", "%M")
                    .replace("ss", "%S");
                // 使用传入的时间 (UTC) 直接格式化，避免受本地时区影响导致测试不稳定
                out.push_str(&now.format(&chrono_fmt).to_string());
            }
            "seq" => {
                out.push_str(&next_seq(now).to_string());
            }
            "screen" => {
                out.push_str(&screen_index.to_string());
            }
            _ => { /* 未知忽略 */ }
        }
        last = m.end();
    }
    out.push_str(&tpl[last..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_date_seq_screen() {
        let now = Utc.with_ymd_and_hms(2025, 1, 2, 10, 15, 30).unwrap();
        reset_sequence_for("20250102");
        let s1 = parse_naming_template("Screenshot-{date:yyyyMMdd-HHmmss}-{seq}-{screen}", 1, now);
        let s2 = parse_naming_template("Screenshot-{date:yyyyMMdd-HHmmss}-{seq}-{screen}", 1, now);
        assert!(s1.starts_with("Screenshot-20250102-101530-1-1"));
        assert!(s2.ends_with("-2-1"));
    }

    #[test]
    fn test_unknown_ignored() {
        let now = Utc::now();
        let s = parse_naming_template("A{unknown}B", 0, now);
        assert_eq!(s, "AB");
    }
}
