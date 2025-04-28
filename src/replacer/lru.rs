use {
    crate::{AccessType, EvictError, EvictResult, EvictionPolicy, FrameId, util::UniqueSequence},
    parking_lot::{RwLock, RwLockWriteGuard},
    priority_queue::PriorityQueue,
    std::{cmp::Reverse, sync::Arc},
};

/// Least Recently Used (LRU) frame replacer.
///
/// This implementation uses a priority queue to manage the frames.
/// The priority queue is ordered by the last access time of the frames. The
/// most recently accessed frame is pushed to the back of the queue, while the
/// least recently accessed item is the first to be evicted.
pub struct LruReplacer<F: FrameId> {
    inner: Arc<RwLock<Inner<F>>>,
}

struct Inner<F: FrameId> {
    /// Maximum number of frames that can be stored in the replacer.
    capacity: usize,

    /// Evictable frames in the replacer.
    frames: PriorityQueue<F, Reverse<i64>>,

    /// Monotonically increasing sequence of timestamps.
    /// Used to determine the order and time of page accesses.
    seq: UniqueSequence,
}

impl<F: FrameId> LruReplacer<F> {
    /// Creates a new LRU replacer.
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner {
                capacity,
                frames: PriorityQueue::with_capacity(capacity),
                seq: UniqueSequence::new(),
            })),
        }
    }

    fn push(mut inner: RwLockWriteGuard<'_, Inner<F>>, id: F) -> EvictResult<(), F> {
        // Ensure that we are not beyond the capacity.
        if inner.frames.len() >= inner.capacity {
            return Err(EvictError::FrameReplacerFull);
        }

        // If the accessed frame is already within the queue, update its priority.
        // Otherwise, insert it. Both cases are handled by the `push` method.
        let priority = inner.seq.next().ok_or(EvictError::SequenceExhausted)?;
        inner.frames.push(id, Reverse(priority));

        Ok(())
    }
}

impl<F: FrameId> EvictionPolicy<F> for LruReplacer<F> {
    type Error = EvictError<F>;

    fn evict(&self) -> Option<F> {
        let mut inner = self.inner.write();
        inner.frames.pop().map(|(frame_id, _)| frame_id)
    }

    fn peek(&self) -> Option<F> {
        let inner = self.inner.read();
        inner.frames.peek().map(|(frame_id, _)| frame_id.clone())
    }

    fn touch(&self, id: F) -> EvictResult<(), F> {
        Self::push(self.inner.write(), id)
    }

    fn touch_with<T: AccessType>(&self, id: F, _access_type: T) -> EvictResult<(), F> {
        // No special handling for access type in LRU.
        self.touch(id)
    }

    fn pin(&self, id: F) -> EvictResult<(), F> {
        // If the frame is non-evictable, remove it from the queue.
        let mut inner = self.inner.write();
        inner.frames.remove(&id);

        Ok(())
    }

    fn unpin(&self, id: F) -> EvictResult<(), F> {
        let inner = self.inner.write();

        // Only insert if the frame is not already in the queue.
        if inner.frames.get(&id).is_none() {
            Self::push(inner, id)?;
        }
        Ok(())
    }

    fn remove(&self, id: F) -> EvictResult<(), F> {
        let res = self.inner.write().frames.remove(&id);
        if res.is_none() {
            return Err(EvictError::PinnedFrameRemoval(id));
        }
        Ok(())
    }

    fn capacity(&self) -> usize {
        self.inner.read().capacity
    }

    fn size(&self) -> usize {
        self.inner.read().frames.len()
    }
}
