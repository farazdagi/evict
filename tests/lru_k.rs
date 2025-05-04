use {
    evict::{
        EvictError,
        EvictionPolicy,
        LruKConfig,
        LruKReplacer,
        replacer::LRUK_REPLACER_REF_PERIOD,
    },
    std::{thread::sleep, time::Duration},
};

#[test]
fn basic_ops() {
    let replacer = LruKReplacer::with_config(LruKConfig {
        capacity: 7,
        k: 2,
        ref_period: 0,
    });
    assert_eq!(0, replacer.size());

    // Scenario: add six elements to the replacer. We have [1,2,3,4,5].
    // Frame 6 is non-evictable (but still kept for access history).
    replacer.touch(1).unwrap();
    replacer.touch(2).unwrap();
    replacer.touch(3).unwrap();
    replacer.touch(4).unwrap();
    replacer.touch(5).unwrap();
    replacer.touch(6).unwrap();
    replacer.pin(6).unwrap();
    assert_eq!(5, replacer.size());

    // Scenario: Insert access history for frame 1. Now frame 1 has two access
    // histories. All other frames have max backward k-dist. The order of
    // eviction is [2,3,4,5,1].
    replacer.touch(1).unwrap();

    // Scenario: Evict three pages from the replacer. Elements with max k-distance
    // should be popped first based on LRU.
    assert_eq!(Some(2), replacer.evict());
    assert_eq!(Some(3), replacer.evict());
    assert_eq!(Some(4), replacer.evict());
    assert_eq!(2, replacer.size());

    // Scenario: Now replacer has frames [5,1].
    // Insert new frames 3, 4, and update access history for 5. We should end with
    // [3,1,5,4]
    replacer.touch(3).unwrap();
    replacer.touch(4).unwrap();
    replacer.touch(5).unwrap();
    replacer.touch(4).unwrap();
    replacer.unpin(3).unwrap();
    replacer.unpin(4).unwrap();
    assert_eq!(4, replacer.size());

    // Scenario: continue looking for victims. We expect 3 to be evicted next.
    assert_eq!(Some(3), replacer.evict());
    assert_eq!(3, replacer.size());

    // Set 6 to be evictable. 6 Should be evicted next since it has max backward
    // k-dist.
    replacer.unpin(6).unwrap();
    assert_eq!(4, replacer.size());
    assert_eq!(Some(6), replacer.evict());
    assert_eq!(3, replacer.size());

    // Now we have [1,5,4]. Continue looking for victims.
    replacer.pin(1).unwrap();
    assert_eq!(2, replacer.size());
    assert_eq!(Some(5), replacer.evict());
    assert_eq!(1, replacer.size());

    // Update access history for 1. Now we have [4,1]. Next victim is 4.
    replacer.touch(1).unwrap();
    replacer.touch(1).unwrap();
    replacer.unpin(1).unwrap();
    assert_eq!(2, replacer.size());
    assert_eq!(Some(4), replacer.evict());

    assert_eq!(1, replacer.size());
    assert_eq!(Some(1), replacer.evict());
    assert_eq!(0, replacer.size());

    // This operation should not modify size
    assert_eq!(None, replacer.evict());
    assert_eq!(0, replacer.size());
}

#[test]
fn over_capacity() {
    let replacer = LruKReplacer::with_config(LruKConfig {
        capacity: 3,
        k: 2,
        ref_period: 0,
    });
    assert_eq!(0, replacer.size());

    replacer.touch(1).unwrap();
    replacer.touch(2).unwrap();
    replacer.touch(3).unwrap();
    // Next touch should fail since the replacer is full.
    assert_eq!(replacer.touch(4), Err(EvictError::FrameReplacerFull));
}

#[test]
fn pin_frame() {
    let replacer = LruKReplacer::with_config(LruKConfig {
        capacity: 7,
        k: 2,
        ref_period: 0,
    });
    assert_eq!(0, replacer.size());

    replacer.touch(1).unwrap();
    assert_eq!(1, replacer.size());

    replacer.pin(1).unwrap();
    assert_eq!(0, replacer.size());

    // Pinning again has no effect.
    replacer.pin(1).unwrap();
    assert_eq!(0, replacer.size());
}

#[test]
fn ref_period_early_eviction() {
    let replacer = LruKReplacer::with_config(LruKConfig {
        capacity: 7,
        k: 2,
        ref_period: 100, // 100ms
    });

    // Access 1 -- it shouldn't be evicted up until `ref_period` elapses -- to avoid
    // early page replacement problem.
    replacer.touch(1).unwrap();
    assert_eq!(1, replacer.size());
    assert_eq!(None, replacer.evict());
    assert_eq!(1, replacer.size());

    // Make sure that 1 is evicted after `ref_period` elapses.
    sleep(Duration::from_millis(101));
    assert_eq!(Some(1), replacer.evict());
    assert_eq!(0, replacer.size());
}

#[test]
fn correlated_period() {
    let replacer = LruKReplacer::with_config(LruKConfig {
        capacity: 7,
        k: 2,
        ref_period: 100_000_000, // 100ms
    });

    // Access 1 multiple times -- all accesses are correlated.
    replacer.touch(1).unwrap();
    replacer.touch(1).unwrap();
    replacer.touch(1).unwrap();
    replacer.touch(1).unwrap();
    replacer.touch(1).unwrap();
    assert_eq!(1, replacer.size());

    // Access 2 multiple times but with a delay -- all accesses are uncorrelated.
    replacer.touch(2).unwrap();
    sleep(Duration::from_millis(100));
    replacer.touch(2).unwrap();
    sleep(Duration::from_millis(100));
    replacer.touch(2).unwrap();
    assert_eq!(2, replacer.size());
}

#[test]
fn remove_arbitrary_frame() {
    let replacer = LruKReplacer::with_config(LruKConfig {
        capacity: 7,
        k: 2,
        ref_period: LRUK_REPLACER_REF_PERIOD,
    });

    // Add frames 1 and 2 to the replacer.
    replacer.touch(1).unwrap();
    replacer.touch(2).unwrap();
    replacer.touch(1).unwrap();
    replacer.touch(1).unwrap();
    replacer.touch(2).unwrap();
    replacer.touch(1).unwrap();
    assert_eq!(2, replacer.size());

    // Cannot evict since reference period hasn't elapsed.
    assert_eq!(None, replacer.peek());

    // Remove frame 1 directly.
    replacer.remove(1).unwrap();
    assert_eq!(1, replacer.size());

    // Pin frame 2 and ensure it is not evictable.
    replacer.pin(2).unwrap();
    assert_eq!(0, replacer.size());
    assert_eq!(replacer.remove(2), Err(EvictError::PinnedFrameRemoval(2)));
    assert_eq!(0, replacer.size());

    // Unpin frame 2 and evict.
    replacer.unpin(2).unwrap();
    assert_eq!(1, replacer.size());
    assert_eq!(None, replacer.peek());
    replacer.remove(2).unwrap();
    assert_eq!(0, replacer.size());
}
