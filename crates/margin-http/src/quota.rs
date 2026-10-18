use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A rolling window of spend: units in the last `window_ms`, not a token bucket.
#[derive(Debug)]
pub struct Quota {
    budget: u32,
    window_ms: u64,
    spent: VecDeque<(u64, u32)>,
}

impl Quota {
    pub fn new(budget: u32, window_ms: u64) -> Self {
        Quota {
            budget,
            window_ms,
            spent: VecDeque::new(),
        }
    }

    fn expire(&mut self, now_ms: u64) {
        while let Some((at, _)) = self.spent.front() {
            if now_ms.saturating_sub(*at) >= self.window_ms {
                self.spent.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn spent(&mut self, now_ms: u64) -> u32 {
        self.expire(now_ms);
        self.spent.iter().map(|(_, units)| units).sum()
    }

    /// How long to wait before `units` more would fit. Zero when they fit now. A call larger than
    /// the whole budget waits for the window to clear and then goes rather than deadlocking.
    pub fn wait_for(&mut self, now_ms: u64, units: u32) -> u64 {
        let spent = self.spent(now_ms);
        if spent + units <= self.budget {
            return 0;
        }
        let mut freed = 0u32;
        for (at, entry) in self.spent.iter() {
            freed += entry;
            if spent.saturating_sub(freed) + units <= self.budget {
                return (at + self.window_ms).saturating_sub(now_ms).max(1);
            }
        }
        self.spent
            .back()
            .map(|(at, _)| (at + self.window_ms).saturating_sub(now_ms).max(1))
            .unwrap_or(0)
    }

    pub fn charge(&mut self, now_ms: u64, units: u32) {
        self.expire(now_ms);
        self.spent.push_back((now_ms, units));
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// One `Quota` per key, typically per account.
#[derive(Debug)]
pub struct Ledger {
    budget: u32,
    window_ms: u64,
    quotas: Mutex<HashMap<String, Quota>>,
}

impl Ledger {
    pub fn new(budget: u32, window_ms: u64) -> Self {
        Ledger {
            budget,
            window_ms,
            quotas: Mutex::default(),
        }
    }

    /// Reserves `units`, sleeping until they fit. Charged before the call so concurrent callers
    /// cannot all read the same low number and then all spend.
    pub async fn spend(&self, key: &str, units: u32) {
        loop {
            let wait = {
                let mut quotas = self.quotas.lock().expect("the quota ledger's lock");
                let quota = quotas
                    .entry(key.to_string())
                    .or_insert_with(|| Quota::new(self.budget, self.window_ms));
                let now = now_ms();
                let wait = quota.wait_for(now, units);
                if wait == 0 {
                    quota.charge(now, units);
                }
                wait
            };
            if wait == 0 {
                return;
            }
            tokio::time::sleep(Duration::from_millis(wait)).await;
        }
    }

    pub fn remaining(&self, key: &str) -> u32 {
        let mut quotas = self.quotas.lock().expect("the quota ledger's lock");
        let quota = quotas
            .entry(key.to_string())
            .or_insert_with(|| Quota::new(self.budget, self.window_ms));
        self.budget.saturating_sub(quota.spent(now_ms()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUDGET: u32 = 6_000;
    const WINDOW_MS: u64 = 60_000;

    #[test]
    fn six_batches_fill_the_minute_and_the_seventh_waits_for_the_window_to_roll() {
        let batch = 1_000;
        let mut quota = Quota::new(BUDGET, WINDOW_MS);
        let start = 1_000_000u64;

        for n in 0..6 {
            let at = start + n * 2_000;
            assert_eq!(quota.wait_for(at, batch), 0, "batch {n} should have fitted");
            quota.charge(at, batch);
        }
        assert_eq!(quota.spent(start + 12_000), BUDGET);

        let wait = quota.wait_for(start + 12_000, batch);
        assert_eq!(wait, WINDOW_MS - 12_000);

        let after = start + WINDOW_MS;
        assert_eq!(quota.wait_for(after, batch), 0);
        assert_eq!(quota.spent(after), BUDGET - batch);
    }

    #[test]
    fn an_ordinary_minute_of_work_never_touches_the_ceiling() {
        let mut quota = Quota::new(BUDGET, WINDOW_MS);
        let mut at = 0u64;
        for units in [2, 2, 2, 2, 2, 5, 50, 100, 1, 1] {
            assert_eq!(quota.wait_for(at, units), 0, "{units} should fit");
            quota.charge(at, units);
            at += 1_000;
        }
        assert_eq!(quota.spent(at), 10 + 5 + 50 + 100 + 1 + 1);
        assert!(quota.spent(at) < BUDGET);
    }

    #[test]
    fn spend_leaves_the_window_on_its_own() {
        let mut quota = Quota::new(BUDGET, WINDOW_MS);
        quota.charge(0, 5_000);
        assert_eq!(quota.spent(59_999), 5_000);
        assert_eq!(quota.spent(60_000), 0);
    }
}
