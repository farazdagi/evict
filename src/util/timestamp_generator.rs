use chrono::Utc;

/// Unique timestamp generator.
///
/// Monotonically increasing timestamp generator, with an additional guarantee
/// of uniqueness of produced values. The granularity of the timestamp is
/// nanoseconds, and if two calls to `generate` happen too quickly, so that full
/// nanosecond doesn't elapse, the second (and any consecutive) call will be
/// incremented by 1.
#[derive(Debug, Default)]
pub struct UniqueTimestampGenerator {
    last_timestamp: i64,
}

impl UniqueTimestampGenerator {
    /// Creates a new unique timestamp generator.
    #[must_use]
    pub const fn new() -> Self {
        Self { last_timestamp: 0 }
    }

    /// Returns a unique timestamp.
    pub fn generate(&mut self) -> i64 {
        let mut timestamp = Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or(self.last_timestamp);
        if timestamp <= self.last_timestamp {
            timestamp = self.last_timestamp + 1;
        }
        self.last_timestamp = timestamp;
        timestamp
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        std::{thread::sleep, time::Duration},
    };

    #[test]
    fn test_uniq_timestamp_nanos() {
        let mut seq = UniqueTimestampGenerator::new();
        let mut prev = 0i64;
        for _ in 0..10 {
            let timestamp = seq.generate();
            assert_ne!(prev, timestamp);
            prev = timestamp;
        }
    }

    #[test]
    fn ensure_rate() {
        struct TestCase {
            sleep_time: Duration,
            expected_diff: i64,
        }

        let tests = vec![
            TestCase {
                sleep_time: Duration::from_micros(1),
                expected_diff: 1_000,
            },
            TestCase {
                sleep_time: Duration::from_micros(100),
                expected_diff: 100_000,
            },
            TestCase {
                sleep_time: Duration::from_millis(1),
                expected_diff: 1_000_000,
            },
            TestCase {
                sleep_time: Duration::from_millis(200),
                expected_diff: 200_000_000,
            },
        ];
        let mut seq = UniqueTimestampGenerator::new();
        for test in tests {
            let t1 = seq.generate();
            sleep(test.sleep_time);
            let t2 = seq.generate();
            assert!(t2 - t1 >= test.expected_diff);
        }
    }
}
