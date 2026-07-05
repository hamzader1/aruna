pub struct ArenaLayout {
    size: usize,
    align: usize,
}

impl ArenaLayout {
    pub fn new<T>() -> Self {
        Self {
            size: std::mem::size_of::<T>(),
            align: std::mem::align_of::<T>(),
        }
    }
}
