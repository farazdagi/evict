use std::sync::atomic::{AtomicI64, Ordering};

/// Thread-safe unique sequence number generator.
///
/// Whenever some replacer needs to log the time of the last access, it can use
/// sequence numbers as a timestamp.
#[derive(Debug, Default)]
pub struct UniqueSequence {
    val: AtomicI64,
}

impl UniqueSequence {
    /// Creates a new unique timestamp generator.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            val: AtomicI64::new(0),
        }
    }

    /// Returns next sequence number.
    ///
    /// Whenever maximum value is reached, the function returns `None`.
    pub fn next(&self) -> Option<i64> {
        let val = self.val.fetch_add(1, Ordering::SeqCst);
        if val == i64::MAX { None } else { Some(val) }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        std::{
            sync::Arc,
            thread::{self, sleep},
            time::Duration,
        },
    };

    #[test]
    fn basic_inc() {
        let seq = UniqueSequence::new();
        let mut prev = -1i64;
        for _ in 0..1000 {
            let timestamp = seq.next().expect("Failed to get timestamp");
            assert_ne!(prev, timestamp);
            assert!(timestamp > prev);
            assert_eq!(timestamp - prev, 1);
            prev = timestamp;
        }
    }

    #[test]
    fn multi_threaded() {
        let seq = Arc::new(UniqueSequence::new());

        let t = 10;
        let n = 100;

        let mut handles = vec![];
        for _ in 0..t {
            let seq_clone = Arc::clone(&seq);
            handles.push(thread::spawn(move || {
                for _ in 0..n {
                    let timestamp = seq_clone.next().expect("Failed to get timestamp");
                    assert!(timestamp >= 0);
                    sleep(Duration::from_millis(1));
                }
            }));
        }

        for handle in handles {
            handle.join().expect("Thread panicked");
        }
        assert_eq!(seq.next(), Some(t * n));
    }
}
