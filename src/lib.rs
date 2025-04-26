#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#![deny(elided_lifetimes_in_paths)]

mod error;
mod replacer;
mod util;

use std::{error::Error, fmt, hash::Hash};

pub use {
    error::{EvictError, EvictResult},
    replacer::LruReplacer,
};

/// Frame identifier type.
///
/// Conceptually, the replacement policy implementation is assumed to be a
/// fixed-size array of frames, where each frame represents a container that
/// holds some page of data. The frame identifier is an index into this array.
pub trait FrameId: Copy + Hash + Eq + fmt::Display + fmt::Debug {}

impl<T> FrameId for T where T: Copy + Hash + Eq + fmt::Display + fmt::Debug {}

/// Page access type.
///
/// When pages are accessed, some policies might log it differently based on
/// nature of the access. For example, a page might be accessed for reading a
/// single data point in it or for scanning of the whole page -- policies might
/// want to distinguish between these access patterns.
pub trait AccessType {}

/// Page eviction policy.
///
/// Defines an interface for interacting with different page replacement
/// strategies. At its core, it provides methods for logging data access,
/// managing meta-data, and eventually locating the next frame to evict.
pub trait EvictionPolicy<F: FrameId> {
    /// Error type for the eviction policy.
    type Error: Error;

    /// Find the next frame to be evicted and evict it.
    ///
    /// Only non-pinned frames are candidates for eviction.
    /// Use [`EvictionPolicy::pin`] to pin frames.
    ///
    /// Successful eviction of a frame decreases the list size of non-pinned
    /// frames and potentially cleans the frame's access history.
    fn evict(&self) -> Option<F>;

    /// Peek into the next frame to be evicted.
    ///
    /// This function does not remove the frame from the list of non-pinned
    /// frames.
    fn peek(&self) -> Option<F>;

    /// Notifies the policy manager that a page controlled by the frame has been
    /// referenced/accessed.
    ///
    /// This normally updates the access history of a frame using the current
    /// timestamp.
    fn touch(&self, id: F) -> Result<(), Self::Error>;

    /// Notifies the policy manager that a page controlled by the frame has been
    /// referenced/accessed. In addition to mere occurrence of access, this
    /// method also logs the type of the access.
    fn touch_with<T: AccessType>(&self, id: F, access_type: T) -> Result<(), Self::Error>;

    /// Pin a frame, marking it as non-evictable.
    ///
    /// If the frame is already pinned, nothing happens.
    fn pin(&self, id: F) -> Result<(), Self::Error>;

    /// Unpin a frame, marking it as evictable.
    ///
    /// If the frame is already unpinned, nothing happens.
    fn unpin(&self, id: F) -> Result<(), Self::Error>;

    /// Removes an evictable frame.
    ///
    /// In contrast to [`evict`](crate::EvictionPolicy::evict), this function
    /// removes an arbitrary non-pinned frame, not necessarily the one with
    /// the highest priority.
    ///
    /// If the frame is pinned, then this function should return an error.
    fn remove(&self, id: F) -> Result<(), Self::Error>;

    /// Returns the maximum number of frames that can be stored.
    fn capacity(&self) -> usize;

    /// The number of elements that can be evicted.
    /// Essentially, this is the number of non-pinned frames.
    fn size(&self) -> usize;
}
