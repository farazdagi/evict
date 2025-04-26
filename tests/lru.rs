mod common;

use {evict::LruReplacer, std::sync::Arc};

#[test]
fn basic_ops() {
    let replacer = Arc::new(LruReplacer::new(20));
    common::basic_ops(replacer.clone());
}

#[test]
fn touch() {
    let replacer = Arc::new(LruReplacer::new(20));
    common::touch(replacer.clone());
}

#[test]
fn remove() {
    let replacer = LruReplacer::new(20);
    common::remove(Arc::new(replacer));
}
