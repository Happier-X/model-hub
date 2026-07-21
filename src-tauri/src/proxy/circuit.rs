//! 默认熔断状态机（对齐 CC Switch 通用档，参数写死）。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;

/// 连续失败阈值。
pub const FAILURE_THRESHOLD: u32 = 4;
/// 恢复等待。
pub const RECOVERY_TIMEOUT: Duration = Duration::from_secs(60);
/// 半开成功关断阈值。
pub const HALF_OPEN_SUCCESS_THRESHOLD: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthLabel {
    Healthy,
    Warning,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone)]
struct Entry {
    state: CircuitState,
    consecutive_failures: u32,
    opened_at: Option<Instant>,
    half_open_successes: u32,
}

impl Default for Entry {
    fn default() -> Self {
        Self {
            state: CircuitState::Closed,
            consecutive_failures: 0,
            opened_at: None,
            half_open_successes: 0,
        }
    }
}

#[derive(Clone, Default)]
pub struct CircuitRegistry {
    inner: Arc<Mutex<HashMap<i64, Entry>>>,
}

impl CircuitRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn with_entry<T>(&self, provider_id: i64, f: impl FnOnce(&mut Entry) -> T) -> T {
        let mut guard = self.inner.lock().expect("circuit lock");
        let entry = guard.entry(provider_id).or_default();
        Self::maybe_transition(entry);
        f(entry)
    }

    fn maybe_transition(entry: &mut Entry) {
        if entry.state == CircuitState::Open {
            if let Some(opened) = entry.opened_at {
                if opened.elapsed() >= RECOVERY_TIMEOUT {
                    entry.state = CircuitState::HalfOpen;
                    entry.half_open_successes = 0;
                }
            }
        }
    }

    /// 是否允许请求该供应商。
    pub fn allow_request(&self, provider_id: i64) -> bool {
        self.with_entry(provider_id, |e| match e.state {
            CircuitState::Open => false,
            CircuitState::Closed | CircuitState::HalfOpen => true,
        })
    }

    pub fn record_success(&self, provider_id: i64) {
        self.with_entry(provider_id, |e| match e.state {
            CircuitState::HalfOpen => {
                e.half_open_successes += 1;
                if e.half_open_successes >= HALF_OPEN_SUCCESS_THRESHOLD {
                    e.state = CircuitState::Closed;
                    e.consecutive_failures = 0;
                    e.opened_at = None;
                    e.half_open_successes = 0;
                }
            }
            CircuitState::Closed => {
                e.consecutive_failures = 0;
            }
            CircuitState::Open => {}
        });
    }

    pub fn record_failure(&self, provider_id: i64) {
        self.with_entry(provider_id, |e| {
            e.consecutive_failures = e.consecutive_failures.saturating_add(1);
            match e.state {
                CircuitState::HalfOpen => {
                    e.state = CircuitState::Open;
                    e.opened_at = Some(Instant::now());
                    e.half_open_successes = 0;
                }
                CircuitState::Closed => {
                    if e.consecutive_failures >= FAILURE_THRESHOLD {
                        e.state = CircuitState::Open;
                        e.opened_at = Some(Instant::now());
                    }
                }
                CircuitState::Open => {}
            }
        });
    }

    pub fn health_label(&self, provider_id: i64) -> HealthLabel {
        self.with_entry(provider_id, |e| match e.state {
            CircuitState::Open => HealthLabel::Open,
            CircuitState::HalfOpen => HealthLabel::HalfOpen,
            CircuitState::Closed => {
                if e.consecutive_failures > 0 {
                    HealthLabel::Warning
                } else {
                    HealthLabel::Healthy
                }
            }
        })
    }

    pub fn consecutive_failures(&self, provider_id: i64) -> u32 {
        self.with_entry(provider_id, |e| e.consecutive_failures)
    }

    pub fn snapshot(&self) -> Vec<(i64, HealthLabel, u32)> {
        let mut guard = self.inner.lock().expect("circuit lock");
        let mut out = Vec::new();
        for (id, entry) in guard.iter_mut() {
            Self::maybe_transition(entry);
            let label = match entry.state {
                CircuitState::Open => HealthLabel::Open,
                CircuitState::HalfOpen => HealthLabel::HalfOpen,
                CircuitState::Closed => {
                    if entry.consecutive_failures > 0 {
                        HealthLabel::Warning
                    } else {
                        HealthLabel::Healthy
                    }
                }
            };
            out.push((*id, label, entry.consecutive_failures));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_after_threshold_and_recovers_to_half_open() {
        let reg = CircuitRegistry::new();
        for _ in 0..FAILURE_THRESHOLD {
            assert!(reg.allow_request(1));
            reg.record_failure(1);
        }
        assert!(!reg.allow_request(1));
        // force recovery
        {
            let mut g = reg.inner.lock().unwrap();
            let e = g.get_mut(&1).unwrap();
            e.opened_at = Some(Instant::now() - RECOVERY_TIMEOUT - Duration::from_secs(1));
        }
        assert!(reg.allow_request(1));
        reg.record_success(1);
        reg.record_success(1);
        assert!(reg.allow_request(1));
        assert!(matches!(reg.health_label(1), HealthLabel::Healthy));
    }

    #[test]
    fn half_open_failure_reopens() {
        let reg = CircuitRegistry::new();
        for _ in 0..FAILURE_THRESHOLD {
            reg.record_failure(7);
        }
        {
            let mut g = reg.inner.lock().unwrap();
            let e = g.get_mut(&7).unwrap();
            e.state = CircuitState::HalfOpen;
        }
        reg.record_failure(7);
        assert!(!reg.allow_request(7));
    }
}
