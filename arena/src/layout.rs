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

    pub fn from_size_align(size: usize, align: usize) -> std::result::Result<Self, ()> {
        unsafe {
            Ok(Self {
                size,
                align: std::mem::transmute(align),
            })
        }
    }
}
