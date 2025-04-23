pub mod error;
mod policy;
mod util;

pub use policy::{AccessType, EvictionPolicy, FrameId, LruPolicy};
