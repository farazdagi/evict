mod lru;
mod lru_k;

pub use {
    lru::LruReplacer,
    lru_k::{LRUK_REPLACER_K, LRUK_REPLACER_REF_PERIOD, LruKConfig, LruKReplacer},
};
