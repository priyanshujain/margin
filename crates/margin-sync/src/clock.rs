use std::time::{SystemTime, UNIX_EPOCH};

/// A hybrid logical clock. `ms` follows the wall clock but never runs backwards and never falls
/// behind a stamp already seen from another device; `counter` orders events inside one millisecond.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hlc {
    pub ms: i64,
    pub counter: u32,
}

impl Hlc {
    pub fn new(ms: i64, counter: u32) -> Hlc {
        Hlc { ms, counter }
    }

    /// The stamp for a local event at wall time `now_ms`.
    pub fn tick(&mut self, now_ms: i64) -> Hlc {
        if now_ms > self.ms {
            self.ms = now_ms;
            self.counter = 0;
        } else {
            self.counter += 1;
        }
        *self
    }

    /// Moves the clock past a stamp received from another device.
    pub fn observe(&mut self, seen: Hlc) {
        if seen > *self {
            *self = seen;
        }
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_are_strictly_increasing_even_when_the_wall_clock_stalls_or_goes_back() {
        let mut clock = Hlc::default();
        let a = clock.tick(100);
        let b = clock.tick(100);
        let c = clock.tick(90);
        let d = clock.tick(101);
        assert!(a < b && b < c && c < d);
        assert_eq!(d, Hlc::new(101, 0));
    }

    #[test]
    fn observing_a_later_remote_stamp_puts_local_events_after_it() {
        let mut clock = Hlc::default();
        clock.tick(100);
        clock.observe(Hlc::new(5_000, 3));
        let next = clock.tick(120);
        assert!(next > Hlc::new(5_000, 3));
        clock.observe(Hlc::new(10, 0));
        assert!(clock.tick(130) > next);
    }
}
