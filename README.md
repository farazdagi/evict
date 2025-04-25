# evict

[![crates.io](https://img.shields.io/crates/d/evict.svg)](https://crates.io/crates/evict)
[![docs.rs](https://docs.rs/evict/badge.svg)](https://docs.rs/evict)

Comprehensive list of effective
[page replacement policies](https://en.wikipedia.org/wiki/Page_replacement_algorithm)
implementations.

## Features

- [x] Multi-threaded: no problem wrapping the eviction policy in an `Arc<_>` and sharing it across
  threads.
- [x] Support for custom data structures: the eviction policies are implemented as traits operating
  on frame `IDs`, so you can implement your own data structure and use it with the policies.
- [x] Support for custom eviction policies: the eviction policies are implemented as traits, so you
  can implement your own eviction policy and use it with the library.
- [ ] Both conventional and state of the art eviction policies out of the box:
  - [x] `LRU` (Least Recently Used)
  - [ ] `MRU` (Most Recently Used)
  - [ ] `FIFO` (First In First Out)
  - [ ] `Random`
  - [ ] `LRU-K` (Least Recently Used with K)
  - [ ] `LFU` (Least Frequently Used)
  - [ ] `2Q` (Two Queue)
  - [ ] `LIRS` (Low Inter-reference Recency Set)
  - [ ] `Clock`
  - [ ] `ARC` (Adaptive Replacement Cache)
  - [ ] `CAR` (Cache with Adaptive Replacement)
  - [ ] `LRFU` (Least Recently/Frequently Used)
  - [ ] `SLRU` (Segmented LRU)

## Motivation

Whenever an in-memory database or cache reaches its maximum size, it needs to evict some pages to
make room for new ones. The eviction policy determines which pages to page out (and possibly write
to disk) when a new page is requested.

The choice of eviction policy can have a significant impact on the performance of the database or
cache. Depending on the workload, some of these policies might work better than the others.

## Usage

Everything spins around the [`EvictionPolicy`](crate::EvictionPolicy) trait. It abstracts the
eviction functionality and provides a common interface for all eviction policies.

### Basic usage (LRU)

``` rust
use {
    evict::{EvictionPolicy, LruReplacer},
    std::sync::Arc,
};

// Create a new LRU policy with a maximum size of 20 pages.
let replacer = Arc::new(LruReplacer::new(20));

// By default all pages are pinned and cannot be evicted.
assert_eq!(replacer.size(), 0);
assert_eq!(replacer.evict(), None);

// Whenever a page is created in your page buffer, you should notify the policy.
// This will mark as evictable pages 1, 2, 3.
// .. new page is created in, say, buffer pool; notify the replacer:
replacer.unpin(1);
replacer.unpin(2);
replacer.unpin(3);

// Some page has been recently used, so we should notify the policy.
replacer.touch(1);

// Now the policy can evict pages.
// Note that page 1 has been touched, so page 2 will be evicted.
assert_eq!(replacer.evict(), Some(2));
assert_eq!(replacer.size(), 2);

assert_eq!(replacer.evict(), Some(3));
assert_eq!(replacer.size(), 1);

assert_eq!(replacer.evict(), Some(1));
assert_eq!(replacer.size(), 0);

assert_eq!(replacer.evict(), None);
```

More advanced usage examples can be found in the documentation for each eviction policy.

## License

MIT
