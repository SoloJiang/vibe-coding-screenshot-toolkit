use chrono::{TimeZone, Utc};
use screenshot_core::naming::{next_seq, reset_sequence_for, set_sequence_for};

#[test]
fn sequence_resets_on_new_day() {
    // 模拟 2025-08-18 日内已用 5
    set_sequence_for("20250818", 5);
    let t1 = Utc
        .with_ymd_and_hms(2025, 8, 18, 23, 59, 50)
        .single()
        .unwrap();
    let s1 = next_seq(t1);
    assert_eq!(s1, 6);

    // 跨到下一天
    let t2 = Utc.with_ymd_and_hms(2025, 8, 19, 0, 0, 1).single().unwrap();
    let s2 = next_seq(t2);
    assert_eq!(s2, 1, "sequence should reset to 1 on new day");

    // 清理重置 side effect 以免影响其它测试
    reset_sequence_for("00000000");
}
