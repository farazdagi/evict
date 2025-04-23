use {
    evict::{EvictionPolicy, LruPolicy, error::EvictError},
    std::sync::Arc,
};

#[test]
fn basic_ops() {
    let lru_replacer = Arc::new(LruPolicy::new(20));

    // Scenario: unpin six elements, i.e. add them to the replacer.
    lru_replacer.unpin(1).unwrap();
    lru_replacer.unpin(2).unwrap();
    lru_replacer.unpin(3).unwrap();
    lru_replacer.unpin(4).unwrap();
    lru_replacer.unpin(5).unwrap();
    lru_replacer.unpin(6).unwrap();
    lru_replacer.unpin(1).unwrap(); // Unpin 1 again. It should have no effect.
    assert_eq!(6, lru_replacer.size());

    // Scenario: get three victims from the lru.
    let value = lru_replacer.evict().unwrap();
    assert_eq!(1, value);
    let value = lru_replacer.evict().unwrap();
    assert_eq!(2, value);
    let value = lru_replacer.evict().unwrap();
    assert_eq!(3, value);

    // Scenario: pin elements in the replacer.
    // Note that 3 has already been victimized, so pinning 3 should have no effect.
    lru_replacer.pin(3).unwrap();
    lru_replacer.pin(4).unwrap();
    assert_eq!(2, lru_replacer.size());

    // Scenario: unpin 4. We expect that the reference bit of 4 will be set to 1.
    lru_replacer.unpin(4).unwrap();

    // Scenario: continue looking for victims. We expect these victims.
    let value = lru_replacer.evict().unwrap();
    assert_eq!(5, value);
    let value = lru_replacer.evict().unwrap();
    assert_eq!(6, value);
    let value = lru_replacer.evict().unwrap();
    assert_eq!(4, value);
}

#[test]
fn touch() {
    let lru_replacer = LruPolicy::new(20);

    // Scenario: unpin elements, i.e. add them to the replacer.
    lru_replacer.unpin(1).unwrap();
    lru_replacer.unpin(2).unwrap();
    lru_replacer.unpin(3).unwrap();
    assert_eq!(3, lru_replacer.size());

    lru_replacer.unpin(1).unwrap(); // Unpin 1 again. It should have no effect.
    assert_eq!(3, lru_replacer.size());
    assert_eq!(Some(1), lru_replacer.peek());

    // However, if we access 1, it should be moved to the end of the queue.
    lru_replacer.touch(1).unwrap();
    assert_eq!(Some(2), lru_replacer.peek());

    let value = lru_replacer.evict().unwrap();
    assert_eq!(2, value);
    let value = lru_replacer.evict().unwrap();
    assert_eq!(3, value);
    let value = lru_replacer.evict().unwrap();
    assert_eq!(1, value);
}

#[test]
fn remove() {
    let lru_replacer = LruPolicy::new(20);

    // Scenario: unpin elements, i.e. add them to the replacer.
    lru_replacer.unpin(1).unwrap();
    lru_replacer.unpin(2).unwrap();
    lru_replacer.unpin(3).unwrap();
    assert_eq!(3, lru_replacer.size());

    // Scenario: remove 2 from the replacer.
    lru_replacer.remove(2).unwrap();
    assert_eq!(2, lru_replacer.size());
    assert_eq!(Some(1), lru_replacer.peek());

    // Scenario: remove 1 from the replacer.
    lru_replacer.remove(1).unwrap();
    assert_eq!(1, lru_replacer.size());
    assert_eq!(Some(3), lru_replacer.peek());

    // Scenario: pin 3 and then try removing it from the replacer.
    lru_replacer.pin(3).unwrap();
    assert_eq!(
        lru_replacer.remove(3),
        Err(EvictError::PinnedFrameRemoval(3))
    );
}
