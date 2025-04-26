
use {
    evict::{EvictError, EvictionPolicy, LruReplacer},
    std::sync::Arc,
};

#[test]
fn basic_ops() {
    let replacer = Arc::new(LruReplacer::new(20));
    assert_eq!(replacer.capacity(), 20);

    // Scenario: unpin six elements, i.e. add them to the replacer.
    replacer.unpin(1).unwrap();
    replacer.unpin(2).unwrap();
    replacer.unpin(3).unwrap();
    replacer.unpin(4).unwrap();
    replacer.unpin(5).unwrap();
    replacer.unpin(6).unwrap();
    replacer.unpin(1).unwrap(); // Unpin 1 again. It should have no effect.
    assert_eq!(6, replacer.size());

    // Scenario: get three victims from the lru.
    let value = replacer.evict().unwrap();
    assert_eq!(1, value);
    let value = replacer.evict().unwrap();
    assert_eq!(2, value);
    let value = replacer.evict().unwrap();
    assert_eq!(3, value);

    // Scenario: pin elements in the replacer.
    // Note that 3 has already been victimized, so pinning 3 should have no effect.
    replacer.pin(3).unwrap();
    replacer.pin(4).unwrap();
    assert_eq!(2, replacer.size());

    // Scenario: unpin 4. We expect that the reference bit of 4 will be set to 1.
    replacer.unpin(4).unwrap();

    // Scenario: continue looking for victims. We expect these victims.
    let value = replacer.evict().unwrap();
    assert_eq!(5, value);
    let value = replacer.evict().unwrap();
    assert_eq!(6, value);
    let value = replacer.evict().unwrap();
    assert_eq!(4, value);
}

#[test]
fn touch() {
    let replacer = Arc::new(LruReplacer::new(20));

    // Scenario: unpin elements, i.e. add them to the replacer.
    replacer.unpin(1).unwrap();
    replacer.unpin(2).unwrap();
    replacer.unpin(3).unwrap();
    assert_eq!(3, replacer.size());

    replacer.unpin(1).unwrap(); // Unpin 1 again. It should have no effect.
    assert_eq!(3, replacer.size());
    assert_eq!(Some(1), replacer.peek());

    // However, if we access 1, it should be moved to the end of the queue.
    replacer.touch(1).unwrap();
    assert_eq!(Some(2), replacer.peek());

    let value = replacer.evict().unwrap();
    assert_eq!(2, value);
    let value = replacer.evict().unwrap();
    assert_eq!(3, value);
    let value = replacer.evict().unwrap();
    assert_eq!(1, value);
}

#[test]
fn remove() {
    let replacer = LruReplacer::new(20);

    // Scenario: unpin elements, i.e. add them to the replacer.
    replacer.unpin(1).unwrap();
    replacer.unpin(2).unwrap();
    replacer.unpin(3).unwrap();
    assert_eq!(3, replacer.size());

    // Scenario: remove 2 from the replacer.
    replacer.remove(2).unwrap();
    assert_eq!(2, replacer.size());
    assert_eq!(Some(1), replacer.peek());

    // Scenario: remove 1 from the replacer.
    replacer.remove(1).unwrap();
    assert_eq!(1, replacer.size());
    assert_eq!(Some(3), replacer.peek());

    // Scenario: pin 3 and then try removing it from the replacer.
    replacer.pin(3).unwrap();
    assert_eq!(replacer.remove(3), Err(EvictError::PinnedFrameRemoval(3)));
}
