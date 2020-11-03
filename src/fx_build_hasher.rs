use rustc_hash::FxHasher;
use std::hash::BuildHasher;

#[derive(Clone)]
pub struct FxBuildHasher;

impl FxBuildHasher {
    pub fn new() -> Self {
        Self {}
    }
}

impl BuildHasher for FxBuildHasher {
    type Hasher = FxHasher;

    fn build_hasher(&self) -> Self::Hasher {
        FxHasher::default()
    }
}
