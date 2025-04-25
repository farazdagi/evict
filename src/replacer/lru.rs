use {
    crate::{
        AccessType,
        EvictError,
        EvictResult,
        EvictionPolicy,
        FrameId,
        util::UniqueTimestampGenerator,
    },
    parking_lot::{RwLock, RwLockWriteGuard},
    priority_queue::PriorityQueue,
    std::{cmp::Reverse, sync::Arc},
};

type Priority = Reverse<i64>;
type Frames = PriorityQueue<FrameId, Priority>;

/// Least Recently Used (LRU) frame replacer.
pub struct LruReplacer {
    inner: Arc<RwLock<Inner>>,
}

struct Inner {
    /// Maximum number of frames that can be stored in the replacer.
    capacity: usize,

    /// Evictable frames in the replacer.
    frames: Frames,

    /// Monotonically increasing sequence of timestamps.
    /// Used to determine the order and time of page accesses.
    seq: UniqueTimestampGenerator,
}

impl LruReplacer {
    /// Creates a new LRU replacer.
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner {
                capacity,
                frames: PriorityQueue::with_capacity(capacity),
                seq: UniqueTimestampGenerator::new(),
            })),
        }
    }

    fn push(mut inner: RwLockWriteGuard<'_, Inner>, id: FrameId) -> EvictResult<()> {
        // Ensure that we are not beyond the capacity.
        if inner.frames.len() >= inner.capacity {
            return Err(EvictError::FrameReplacerFull);
        }

        // If the accessed frame is already within the queue, update its priority.
        // Otherwise, insert it. Both cases are handled by the `push` method.
        let priority = inner.seq.generate();
        inner.frames.push(id, Reverse(priority));

        Ok(())
    }
}

impl EvictionPolicy for LruReplacer {
    type Error = EvictError;

    fn evict(&self) -> Option<FrameId> {
        let mut inner = self.inner.write();
        inner.frames.pop().map(|(frame_id, _)| frame_id)
    }

    fn peek(&self) -> Option<FrameId> {
        let inner = self.inner.read();
        inner.frames.peek().map(|(frame_id, _)| *frame_id)
    }

    fn touch(&self, id: FrameId) -> EvictResult<()> {
        Self::push(self.inner.write(), id)
    }

    fn touch_with<T: AccessType>(&self, id: FrameId, _access_type: T) -> EvictResult<()> {
        // No special handling for access type in LRU.
        self.touch(id)
    }

    fn pin(&self, id: FrameId) -> EvictResult<()> {
        // If the frame is non-evictable, remove it from the queue.
        let mut inner = self.inner.write();
        inner.frames.remove(&id);

        Ok(())
    }

    fn unpin(&self, id: FrameId) -> EvictResult<()> {
        let inner = self.inner.write();

        // Only insert if the frame is not already in the queue.
        if inner.frames.get(&id).is_none() {
            Self::push(inner, id)?;
        }
        Ok(())
    }

    fn remove(&self, id: FrameId) -> EvictResult<()> {
        let res = self.inner.write().frames.remove(&id);
        if res.is_none() {
            return Err(EvictError::PinnedFrameRemoval(id));
        }
        Ok(())
    }

    fn capacity(&self) -> usize {
        usize::MAX
    }

    fn size(&self) -> usize {
        self.inner.read().frames.len()
    }
}
