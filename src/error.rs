use crate::FrameId;

/// Cache eviction policy error.
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum EvictError {
    /// Invalid frame id.
    #[error("Invalid frame id: {0}")]
    InvalidFrameId(FrameId),

    /// Trying to remove pinned frame.
    #[error("Trying to remove pinned frame: {0}")]
    PinnedFrameRemoval(FrameId),

    /// Cannot add any more pages to the frame replacer.
    #[error("Frame replacer is full")]
    FrameReplacerFull,

    /// Invalid sequence number.
    #[error("Invalid timestamp")]
    InvalidTimestamp,

    /// No free frames available.
    #[error("No free frames available (nor in free list nor in frame replacer)")]
    NoFramesAvailable,
}

/// Cache eviction policy result type.
pub type EvictResult<T> = Result<T, EvictError>;
