//! 分组 item 选择：mode=1 轮询，其它 mode 暂用首项。

use std::sync::atomic::{AtomicU64, Ordering};

/// 根据分组 mode 与 item 数量选择起始下标。
///
/// - `mode == 1`：原子计数 `% len`（每次调用递增）
/// - 其它 mode：固定返回 `0`（后续可增强）
///
/// `len == 0` 时返回 `0`（调用方应先判空）。
pub fn select_item_index(mode: i64, len: usize, counter: &AtomicU64) -> usize {
    if len == 0 {
        return 0;
    }
    if mode == 1 {
        let n = counter.fetch_add(1, Ordering::Relaxed);
        (n as usize) % len
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_len_returns_zero() {
        let c = AtomicU64::new(0);
        assert_eq!(select_item_index(1, 0, &c), 0);
    }

    #[test]
    fn mode_one_cycles() {
        let c = AtomicU64::new(0);
        let seq: Vec<_> = (0..6).map(|_| select_item_index(1, 2, &c)).collect();
        assert_eq!(seq, vec![0, 1, 0, 1, 0, 1]);
    }
}
