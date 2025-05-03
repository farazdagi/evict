//! LRU-K page replacement algorithm.
//!
//! The algorithm implemented here is based on the [LRU-K paper](https://dl.acm.org/doi/10.1145/170036.170081).

use {
    crate::{AccessType, EvictError, EvictResult, EvictionPolicy, FrameId},
    hlc_gen::{HlcGenerator, HlcTimestamp},
    parking_lot::RwLock,
    std::{
        collections::{HashMap, VecDeque},
        sync::Arc,
    },
};

/// The look-back window for LRU-K frame replacer.
pub const LRUK_REPLACER_K: usize = 10;

/// Correlated reference period (in milliseconds).
///
/// We will follow Gray's "5-second rule" and give 5 seconds in-between to
/// consider two references as uncorrelated.
pub const LRUK_REPLACER_REF_PERIOD: i64 = 5_000;

/// Configuration of the LRU-K replacer.
#[derive(Debug)]
pub struct LruKConfig {
    /// Maximum number of frames to keep track of.
    pub capacity: usize,

    /// Number of most recent page accesses to keep track of.
    pub k: usize,

    /// Correlated reference period (in milliseconds).
    /// Consider intra-transaction references for an update operation, where
    /// page is first referenced during database search and then when the update
    /// is committed. Such access is considered correlated and should not affect
    /// (reward or penalize) the page's backward-k distance.
    pub ref_period: i64,
}

impl Default for LruKConfig {
    fn default() -> Self {
        Self {
            capacity: 4096,
            k: 2,
            ref_period: 0,
        }
    }
}

/// Page information.
#[derive(Debug)]
struct PageInfo {
    /// Page's access history. Timestamps of up to the `k` most recent
    /// *uncorrelated* page references/accesses.
    ///
    /// The most recent reference is at the back of the list.
    refs: VecDeque<HlcTimestamp>,

    /// Timestamp of the last page reference.
    ///
    /// This value is updated on every access, i.e. even if the access is a
    /// correlated reference -- therefore `refs[refs.len()-1]` and `last_ref`
    /// are not always the same.
    last_ref: HlcTimestamp,

    /// Whether the page is pinned or should not be considered for eviction.
    evictable: bool,
}

impl PageInfo {
    fn new(k: usize) -> Self {
        Self {
            refs: VecDeque::with_capacity(k),
            last_ref: HlcTimestamp::default(),
            evictable: true,
        }
    }

    /// Updates the access history of the page using the current timestamp.
    ///
    /// The `ref_period` parameter is used to determine whether the access is
    /// correlated or not (history is updated on uncorrelated references only).
    fn touch(&mut self, timestamp: HlcTimestamp, ref_period: i64) {
        // Update history only if the access is uncorrelated.
        // If `ref_period` is 0, we consider all references as uncorrelated.
        if ref_period == 0 || timestamp - self.last_ref > ref_period {
            // Shift the references to close the previous correlated period.
            let shift = self.refs.back().map_or(0, |last_ref| timestamp - last_ref);
            if shift > 0 {
                for history_el in &mut self.refs {
                    *history_el += shift as u64;
                }
            }

            // If the history list is already at capacity i.e. holds `k` items, remove the
            // oldest reference, before pushing back a new timestamp.
            if self.refs.len() == self.refs.capacity() {
                self.refs.pop_front();
            }
            self.refs.push_back(timestamp);
        }

        self.last_ref = timestamp;
    }
}

/// Implements the LRU-K page replacement algorithm.
pub struct LruKReplacer<F: FrameId> {
    inner: Arc<RwLock<Inner<F>>>,
}

struct Inner<F: FrameId> {
    /// Configuration of the replacer.
    config: LruKConfig,

    /// Number of evictable frames in the replacer.
    size: usize,

    /// Mapping of frame IDs to contained page information.
    ///
    /// Page information includes the page's access history.
    framed_pages: HashMap<F, PageInfo>,

    /// Monotonically increasing sequence of timestamps.
    /// Used to determine the order and time of page accesses.
    seq: HlcGenerator,
}

impl<F: FrameId> Default for LruKReplacer<F> {
    fn default() -> Self {
        Self::with_config(LruKConfig::default())
    }
}

impl<F: FrameId> LruKReplacer<F> {
    /// Creates a new LRU-K replacer with the given capacity and `k` value.
    pub fn new(capacity: usize, k: usize) -> Self {
        Self::with_config(LruKConfig {
            capacity,
            k,
            ..LruKConfig::default()
        })
    }

    /// Creates a new LRU-K replacer with the given configuration.
    pub fn with_config(config: LruKConfig) -> Self {
        let capacity = config.capacity;
        Self {
            inner: Arc::new(RwLock::new(Inner {
                config,
                size: 0,
                framed_pages: HashMap::with_capacity(capacity),
                seq: HlcGenerator::default(),
            })),
        }
    }
}

impl<F: FrameId> EvictionPolicy<F> for LruKReplacer<F> {
    type Error = EvictError<F>;

    fn evict(&self) -> Option<F> {
        self.peek().inspect(|id| {
            let mut inner = self.inner.write();
            // If victim is found, remove it from the replacer.
            inner.framed_pages.remove(id);
            inner.size -= 1;
        })
    }

    fn peek(&self) -> Option<F> {
        let inner = self.inner.read();

        let timestamp = inner.seq.next_timestamp()?;
        let mut max_k_dist = 0i64;
        let mut result = None;

        for (id, page) in &inner.framed_pages {
            // Pinned pages are not considered for eviction.
            if !page.evictable {
                continue;
            }

            // If the page was referenced just recently, to avoid early page replacement
            // problem (see paper), we skip it.
            if inner.config.ref_period > 0 && timestamp - page.last_ref <= inner.config.ref_period {
                continue;
            }

            // Find the backward-k distance of the page.
            let last_uncorrelated_ref = page.refs.back().copied().unwrap_or_default();
            let k_dist = if page.refs.len() < inner.config.k {
                i64::MAX - last_uncorrelated_ref.as_u64() as i64
            } else {
                timestamp.as_u64() as i64 - last_uncorrelated_ref.as_u64() as i64
            };

            if k_dist >= max_k_dist {
                max_k_dist = k_dist;
                result = Some(id.clone());
            }
        }

        result
    }

    fn touch(&self, id: F) -> EvictResult<(), F> {
        let mut inner = self.inner.write();

        // The replacer is full, cannot add new page.
        if inner.size >= inner.config.capacity && !inner.framed_pages.contains_key(&id) {
            return Err(EvictError::FrameReplacerFull);
        }

        // Obtain necessary values from immutable reference, since we will borrow it
        // as mutable later.
        let timestamp = inner
            .seq
            .next_timestamp()
            .ok_or(EvictError::SequenceExhausted)?;
        let ref_period = inner.config.ref_period;
        let k = inner.config.k;

        // Get page's access history or create a new one.
        if !inner.framed_pages.contains_key(&id) {
            inner.size += 1;
        }

        let page = inner
            .framed_pages
            .entry(id)
            .or_insert_with(move || PageInfo::new(k));

        // Record the current access.
        page.touch(timestamp, ref_period);

        Ok(())
    }

    fn touch_with<T: AccessType>(&self, id: F, _access_type: T) -> EvictResult<(), F> {
        // LRU-K does not use access type.
        self.touch(id)
    }

    fn pin(&self, id: F) -> EvictResult<(), F> {
        let mut inner = self.inner.write();

        let page = inner
            .framed_pages
            .get_mut(&id)
            .ok_or(EvictError::InvalidFrameId(id))?;

        // No-op if the frame is already in the desired state.
        if !page.evictable {
            return Ok(());
        }

        // Update the size of the replacer, if state change is necessary.
        page.evictable = false;
        inner.size -= 1;

        Ok(())
    }

    fn unpin(&self, id: F) -> EvictResult<(), F> {
        let mut inner = self.inner.write();

        let page = inner
            .framed_pages
            .get_mut(&id)
            .ok_or(EvictError::InvalidFrameId(id))?;

        // No-op if the frame is already in the desired state.
        if page.evictable {
            return Ok(());
        }

        // Update the size of the replacer, if state change is necessary.
        page.evictable = true;
        inner.size += 1;

        Ok(())
    }

    fn remove(&self, id: F) -> EvictResult<(), F> {
        let mut inner = self.inner.write();

        if let Some(page) = inner.framed_pages.get(&id) {
            if !page.evictable {
                return Err(EvictError::PinnedFrameRemoval(id));
            }
            inner.framed_pages.remove(&id);
            inner.size -= 1;
        }
        Ok(())
    }

    fn capacity(&self) -> usize {
        self.inner.read().config.capacity
    }

    fn size(&self) -> usize {
        self.inner.read().size
    }
}
