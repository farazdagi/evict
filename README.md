# evict

[![crates.io](https://img.shields.io/crates/d/evict.svg)](https://crates.io/crates/evict)
[![docs.rs](https://docs.rs/evict/badge.svg)](https://docs.rs/evict)

Comprehensive list of effective
[page replacement policies](https://en.wikipedia.org/wiki/Page_replacement_algorithm)
implementations.

## Features

- [x] Support for custom data structures: this crate abstracts the eviction policy, and thus it can
  be used to support any data structure used to store actual pages (caches, buffer pools etc).
  The library is designed to do one thing, but do it well.
- [x] Support for custom eviction policies: implementing custom replacement policy is as easy as to
  implement `EvictionPolicy` for your type.
- [x] Multi-threaded: no problem wrapping the eviction policy in an `Arc<_>` and sharing it across
  threads.
- [x] Both conventional and state of the art eviction policies are provided out of the box (see
  Future Work section below):
  - [x] [`LRU`](crate::LruReplacer) (Least Recently Used)
  - [x] [`LRU-K`](crate::LruKReplacer) (LRU with access frequency tracking) (see
    [paper](https://dl.acm.org/doi/10.1145/170036.170081))

## Motivation

Whenever an in-memory database or cache reaches its maximum size, it needs to evict some pages to
make room for new ones. The eviction policy determines which pages to page out (and possibly write
to disk) when a new page is requested.

The choice of eviction policy can have a significant impact on the performance of the database or
cache. Depending on the workload, some of these policies might work better than the others.

This crate provides a set of eviction policies that can be used to manage the eviction process in
caches, buffer pools etc. So, it doesn't provide any data structures to store the pages, instead it
concentrates on providing a flexible and efficient implementation of the eviction policies that you
can use when implementing such data structures.

## Usage

Everything spins around the [`EvictionPolicy`](crate::EvictionPolicy) trait. It abstracts frame
management and eviction functionality and provides a common interface for different algorithms.

### Basic usage (LRU)

``` rust
use {
    evict::{EvictionPolicy, LruReplacer},
};

// Create a new LRU policy with a maximum capacity of 20 frames.
// All policies are thread-safe and can be shared across threads.
let replacer = LruReplacer::new(20);
assert_eq!(replacer.capacity(), 20);

// By default frames are pinned and are not candidates for eviction.
assert_eq!(replacer.size(), 0);
assert_eq!(replacer.evict(), None);

// So, when creating a new page in, say, buffer pool,
// notify the replacer by unpinning frames responsible for pages.
// Once unpinned, frame is considered for eviction.
replacer.unpin(1);
replacer.touch(2); // the first touch works as unpin
replacer.unpin(3);

// When a page is accessed, touch its frame in replacer.
// In most polices it affects the eviction order.
//
// In LRU, the frame is moved to the very end of the list
// of evictable candidates.
replacer.touch(1);

// At some point you may want to decide which frame to evict.

// Frame 1 has been touched the last, so Frame 2 will be evicted.
// Eviction order: 2 -> 3 -> 1
assert_eq!(replacer.evict(), Some(2));
assert_eq!(replacer.size(), 2);

// Frame 3
assert_eq!(replacer.evict(), Some(3));
assert_eq!(replacer.size(), 1);

// Frame 1 was touched the last, so it will be evicted last.
assert_eq!(replacer.evict(), Some(1));
assert_eq!(replacer.size(), 0);

assert_eq!(replacer.evict(), None);
```

More advanced usage examples can be found in the documentation for each eviction policy.

## Future work

Expand the list of eviction policies to include more algorithms:

- [ ] `MRU` (Most Recently Used)
- [ ] `FIFO` (First In First Out)
- [ ] `Random`
- [ ] `LFU` (Least Frequently Used)
- [ ] `2Q` (Two Queue)
- [ ] `LIRS` (Low Inter-reference Recency Set)
- [ ] `Clock`
- [ ] `ARC` (Adaptive Replacement Cache)
- [ ] `CAR` (Cache with Adaptive Replacement)
- [ ] `LRFU` (Least Recently/Frequently Used)
- [ ] `SLRU` (Segmented LRU)

## License

MIT
