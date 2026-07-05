#![allow(dead_code)]
mod platform;
use core::ptr::null_mut;
use platform::Platform;
pub struct BlockHeader {
    pub prev: *mut BlockHeader,
    pub mmap_ptr: *mut u8,
    pub mmap_size: usize,
}
pub struct Arena {
    pub current_block: *mut BlockHeader,
    pub cursor: *mut u8,
    pub end: *mut u8,
}

impl BlockHeader {
    fn new(prev: *mut BlockHeader, mmap_ptr: *mut u8, mmap_size: usize) -> Self {
        Self {
            prev,
            mmap_ptr,
            mmap_size,
        }
    }
}

impl Arena {
    pub fn new() -> Self {
        let page_size = Platform::get_page_size();
        let mmap_ptr = Platform::mmap(page_size);

        let block_header = BlockHeader {
            prev: null_mut(),
            mmap_ptr,
            mmap_size: page_size,
        };
        let mut arena = Self {
            current_block: null_mut(),
            cursor: mmap_ptr,
            end: unsafe { mmap_ptr.add(page_size) },
        };
        arena.write_metadata(block_header);
        arena
    }

    pub fn alloc(&mut self, layout: std::alloc::Layout) -> *mut u8 {
        let (size, align) = (layout.size(), layout.align());
        if size == 0 {
            return null_mut();
        }

        let aligned_cursor = match Self::align_up(self.cursor as usize, align) {
            Some(ac) => ac,
            None => return null_mut(),
        };

        let new_block_size = match aligned_cursor.checked_add(size) {
            Some(new_block_size) => new_block_size,
            None => return null_mut(),
        };

        if new_block_size > self.end as usize {
            self.grow(size);
            return self.alloc(layout);
        }

        self.cursor = unsafe { self.cursor.add(new_block_size - self.cursor as usize) };
        unsafe {
            let current_block_ptr = (*self.current_block).mmap_ptr;
            current_block_ptr.add(aligned_cursor - current_block_ptr as usize)
        }
    }

    pub fn align_up(size: usize, align: usize) -> Option<usize> {
        let checked_cursor_alignment = size.checked_add(align - 1)?;
        Some(checked_cursor_alignment & !(align - 1))
    }
    fn grow(&mut self, requested_size: usize) {
        let prev_block_header = self.current_block;
        let prev_block_size = unsafe { (*self.current_block).mmap_size };
        let aligned_requested_size = Self::align_up(
            requested_size + size_of::<BlockHeader>(),
            Platform::get_page_size(),
        )
        .expect("size overflow");
        let new_block_size = match prev_block_size.checked_mul(2) {
            Some(d) => d.max(aligned_requested_size),
            None => aligned_requested_size,
        };

        let ptr = Platform::mmap(new_block_size);
        if ptr.is_null() {
            panic!("FAILS TO ALLOCATE MORE MEMORY");
        }
        let new_block_header = BlockHeader::new(prev_block_header, ptr, new_block_size);
        self.end = unsafe { ptr.add(new_block_size) };
        self.write_metadata(new_block_header);
    }

    fn write_metadata(&mut self, block_header: BlockHeader) {
        let header_ptr = block_header.mmap_ptr as *mut BlockHeader;
        unsafe {
            self.cursor = block_header
                .mmap_ptr
                .add((size_of::<BlockHeader>() + 8 - 1) & !(8 - 1));
            header_ptr.write(block_header);
            self.current_block = header_ptr;
        }
    }
}

impl std::default::Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_metadata_places_header_at_mmap_ptr_and_aligns_cursor() {
        let arena = Arena::new();

        // header should live at the very start of the chunk
        let mmap_ptr = unsafe { (*arena.current_block).mmap_ptr };
        assert_eq!(arena.current_block as *mut u8, mmap_ptr);

        // cursor should be header_size rounded up to 8, not raw header_size
        let expected_offset = Arena::align_up(size_of::<BlockHeader>(), 8).unwrap();
        let expected_cursor = unsafe { mmap_ptr.add(expected_offset) };
        assert_eq!(arena.cursor, expected_cursor);

        // end should be exactly one page past mmap_ptr
        let page_size = Platform::get_page_size();
        assert_eq!(arena.end, unsafe { mmap_ptr.add(page_size) });
    }

    #[test]
    fn write_metadata_links_prev_correctly_on_grow() {
        let mut arena = Arena::new();
        let first_block = arena.current_block;

        // force a grow with a request bigger than default chunk
        let huge = std::alloc::Layout::from_size_align(Platform::get_page_size() * 4, 8).unwrap();
        arena.alloc(huge);

        let second_block = arena.current_block;
        assert_ne!(first_block, second_block);
        assert_eq!(unsafe { (*second_block).prev }, first_block);
        assert_eq!(unsafe { (*first_block).prev }, null_mut());
    }

    #[test]
    fn alloc_returns_non_null_and_moves_cursor() {
        let mut arena = Arena::new();
        let layout = std::alloc::Layout::from_size_align(16, 8).unwrap();

        let cursor_before = arena.cursor;
        let ptr = arena.alloc(layout);

        assert!(!ptr.is_null());
        assert!(ptr as usize >= cursor_before as usize); // ptr can only move forward (for alignment padding)
        assert_eq!(ptr as usize % 8, 0); // actually aligned as requested
        assert_eq!(arena.cursor, unsafe { ptr.add(16) }); // cursor now sits right after this allocation
    }

    #[test]
    fn alloc_zero_size_returns_null() {
        let mut arena = Arena::new();
        let layout = std::alloc::Layout::from_size_align(0, 8).unwrap();
        assert!(arena.alloc(layout).is_null());
    }

    #[test]
    fn consecutive_allocs_do_not_overlap() {
        let mut arena = Arena::new();
        let layout = std::alloc::Layout::from_size_align(24, 8).unwrap();

        let a = arena.alloc(layout);
        let b = arena.alloc(layout);

        assert!(!a.is_null() && !b.is_null());
        assert_ne!(a, b);
        // b must start at or after a + size
        assert!(b as usize >= a as usize + 24);
    }

    #[test]
    fn alloc_respects_alignment() {
        let mut arena = Arena::new();
        // deliberately unbalance cursor first
        arena.alloc(std::alloc::Layout::from_size_align(3, 1).unwrap());

        let layout = std::alloc::Layout::from_size_align(32, 32).unwrap();
        let ptr = arena.alloc(layout);

        assert_eq!(ptr as usize % 32, 0);
    }

    #[test]
    fn alloc_never_writes_past_end() {
        let mut arena = Arena::new();
        let layout = std::alloc::Layout::from_size_align(64, 8).unwrap();

        for _ in 0..1000 {
            let ptr = arena.alloc(layout);
            assert!(!ptr.is_null());
            assert!(unsafe { ptr.add(64) } as usize <= arena.end as usize);
        }
    }

    #[test]
    fn alloc_triggers_grow_when_chunk_is_full() {
        let mut arena = Arena::new();
        let page_size = Platform::get_page_size();
        let first_block = arena.current_block;

        // fill up the first chunk entirely
        let filler = std::alloc::Layout::from_size_align(page_size, 8).unwrap();
        let _ = arena.alloc(filler); // likely triggers grow since header eats some space

        assert_ne!(
            arena.current_block, first_block,
            "expected grow to have run"
        );
    }

    #[test]
    fn grow_chunk_size_at_least_fits_request() {
        let mut arena = Arena::new();
        let page_size = Platform::get_page_size();
        let requested = page_size * 10;

        let layout = std::alloc::Layout::from_size_align(requested, 8).unwrap();
        let ptr = arena.alloc(layout);

        assert!(!ptr.is_null());
        let new_block_size = unsafe { (*arena.current_block).mmap_size };
        assert!(new_block_size >= requested + size_of::<BlockHeader>());
    }

    #[test]
    fn grow_doubles_when_request_is_small() {
        let mut arena = Arena::new();
        let first_size = unsafe { (*arena.current_block).mmap_size };

        // force exactly one grow with a small request
        let filler = std::alloc::Layout::from_size_align(first_size, 8).unwrap();
        let _ = arena.alloc(filler);

        let new_size = unsafe { (*arena.current_block).mmap_size };
        assert_eq!(new_size, first_size * 2);
    }
}
