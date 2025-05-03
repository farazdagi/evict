use {
    evict::{EvictError, EvictionPolicy, LruReplacer},
    std::sync::Arc,
};

#[test]
fn basic_ops() {
    let replacer = LruReplacer::new(20);
    assert_eq!(replacer.capacity(), 20);

    // Scenario: unpin six elements, this updates eviction candidate list.
    replacer.unpin(1).unwrap();
    replacer.unpin(2).unwrap();
    replacer.unpin(3).unwrap();
    replacer.unpin(4).unwrap();
    replacer.unpin(5).unwrap();
    replacer.unpin(6).unwrap();
    replacer.unpin(1).unwrap(); // Unpin 1 again. It should have no effect.
    assert_eq!(6, replacer.size());

    // Scenario: get three victims from the lru.
    assert_eq!(replacer.evict(), Some(1));
    assert_eq!(replacer.evict(), Some(2));
    assert_eq!(replacer.evict(), Some(3));

    // Scenario: pin elements in the replacer.
    // Note that 3 has already been victimized, so pinning 3 should have no effect.
    replacer.pin(3).expect("cannot pin 3");
    replacer.pin(4).expect("cannot pin 4");
    assert_eq!(2, replacer.size());

    // Scenario: unpin 4. We expect that the reference bit of 4 will be set to 1.
    replacer.unpin(4).expect("cannot unpin 4");

    // Scenario: continue looking for victims. We expect these victims.
    assert_eq!(replacer.evict(), Some(5));
    assert_eq!(replacer.evict(), Some(6));
    assert_eq!(replacer.evict(), Some(4));
}

#[test]
fn touch() {
    {
        let replacer = LruReplacer::new(20);

        // Scenario: unpin elements, i.e. add them to the replacer.
        replacer.unpin(1).unwrap();
        replacer.unpin(2).unwrap();
        replacer.unpin(3).unwrap();
        assert_eq!(3, replacer.size());

        replacer.unpin(1).expect("cannot unpin 1"); // Unpin 1 again. It should have no effect.
        assert_eq!(3, replacer.size());
        assert_eq!(Some(1), replacer.peek());

        // However, if we access 1, it should be moved to the end of the queue.
        replacer.touch(1).unwrap();
        assert_eq!(Some(2), replacer.peek());

        assert_eq!(replacer.evict(), Some(2));
        assert_eq!(replacer.evict(), Some(3));
        assert_eq!(replacer.evict(), Some(1));
    }

    {
        // The first touch should add the frame to the list of non-pinned frames.
        let replacer = LruReplacer::new(20);

        // Scenario: unpin elements by touching.
        replacer.touch(1).unwrap();
        replacer.touch(2).unwrap();
        replacer.touch(3).unwrap();
        assert_eq!(3, replacer.size());

        replacer.unpin(1).expect("cannot unpin 1"); // Unpin 1 again. It should have no effect.
        assert_eq!(3, replacer.size());
        assert_eq!(Some(1), replacer.peek());

        // However, if we access 1, it should be moved to the end of the queue.
        replacer.touch(1).unwrap();
        assert_eq!(Some(2), replacer.peek());

        assert_eq!(replacer.evict(), Some(2));
        assert_eq!(replacer.evict(), Some(3));
        assert_eq!(replacer.evict(), Some(1));
    }
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

#[test]
fn multi_threaded() {
    use std::thread;

    let n = 100;
    let k = 20;
    let replacer = Arc::new(LruReplacer::new(n * k));
    let replacer_clone = Arc::clone(&replacer);

    // Replacer is thread-safe and can be shared between threads.
    // Spawn `n` threads, each thread concurrently unpins its own set of frames (of
    // size `k`). At the end we check that the size of the replacer is `n * k`.
    let mut handles = vec![];
    for i in 0..n {
        let replacer_clone = Arc::clone(&replacer_clone);
        handles.push(thread::spawn(move || {
            for j in 0..k {
                replacer_clone.unpin(i * k + j).unwrap();
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    assert_eq!(replacer.size(), n * k);
}
