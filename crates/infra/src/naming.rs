use chrono::{DateTime, Local, Utc};
use once_cell::sync::OnceCell;
use regex::Regex;
use std::sync::atomic::{AtomicU32, Ordering};

static SEQ: AtomicU32 = AtomicU32::new(0);
static DAY_KEY: OnceCell<String> = OnceCell::new();

pub fn reset_day(day: &str) {
    DAY_KEY.set(day.to_string()).ok();
    SEQ.store(0, Ordering::Relaxed);
}

pub fn next_seq(now: DateTime<Utc>) -> u32 {
    let key = now.format("%Y%m%d").to_string();
    match DAY_KEY.get() {
        Some(k) if k == &key => {}
        _ => {
            let _ = DAY_KEY.set(key);
            SEQ.store(0, Ordering::Relaxed);
        }
    }
    SEQ.fetch_add(1, Ordering::Relaxed) + 1
}

pub fn parse_template(tpl: &str, screen_index: usize, now: DateTime<Utc>) -> String {
    let re = Regex::new(r"\{([^{}:]+)(?::([^{}]+))?\}").unwrap();
    let mut out = String::new();
    let mut last = 0;
    for cap in re.captures_iter(tpl) {
        let m = cap.get(0).unwrap();
        out.push_str(&tpl[last..m.start()]);
        match &cap[1] {
            "date" => {
                let fmt = cap.get(2).map(|v| v.as_str()).unwrap_or("yyyyMMdd-HHmmss");
                let chrono_fmt = fmt
                    .replace("yyyy", "%Y")
                    .replace("MM", "%m")
                    .replace("dd", "%d")
                    .replace("HH", "%H")
                    .replace("mm", "%M")
                    .replace("ss", "%S");
                let local: DateTime<Local> = DateTime::from(now);
                out.push_str(&local.format(&chrono_fmt).to_string());
            }
            "seq" => {
                out.push_str(&next_seq(now).to_string());
            }
            "screen" => {
                out.push_str(&screen_index.to_string());
            }
            _ => {}
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
    fn test_tpl() {
        let now = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        reset_day("20250101");
        let a = parse_template("Screenshot-{date:yyyyMMdd}-{seq}-{screen}", 0, now);
        let b = parse_template("Screenshot-{date:yyyyMMdd}-{seq}-{screen}", 0, now);
        assert!(a.contains("20250101"));
        assert!(a.ends_with("-1-0"));
        assert!(b.ends_with("-2-0"));
    }
}
