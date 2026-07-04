#![allow(dead_code)]
#[derive(Debug)]
pub struct BumpAllocator {
    start: *mut u8,
    end: *mut u8,
    cursor: *mut u8,
}

impl BumpAllocator {
    pub fn new(buffer: &mut [u8]) -> Self {
        unsafe {
            let start = buffer.as_mut_ptr();
            let cursor = buffer.as_mut_ptr();
            let end = start.add(buffer.len());
            Self { start, end, cursor }
        }
    }

    pub fn alloc(&mut self, size: usize, align: usize) -> *mut u8 {
        if size == 0 {
            return std::ptr::null_mut();
        }
        let aligned_cursor = match self.align_up(align) {
            Some(ac) => ac,
            None => return std::ptr::null_mut(),
        };
        println!(
            "ALIGNED_CURSOR: {} FROM ORIGINAL CURSOR: {}",
            aligned_cursor, self.cursor as usize
        );
        let new_size = match aligned_cursor.checked_add(size) {
            Some(new_size) => new_size,
            None => return std::ptr::null_mut(),
        };
        if new_size > self.end as usize {
            return std::ptr::null_mut();
        }
        self.cursor = new_size as *mut u8;
        unsafe { self.start.add(aligned_cursor - self.start as usize) }
        // aligned_cursor as *mut u8
    }
    fn align_up(&self, align: usize) -> Option<usize> {
        let check_curr = (self.cursor as usize).checked_add(align - 1)?;
        Some(check_curr & !(align - 1))
    }
    fn reset(&mut self) {
        self.cursor = self.start
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn make_allocator(buffer: &mut [u8]) -> BumpAllocator {
        BumpAllocator::new(buffer)
    }

    // 1. basic alloc returns non-null
    #[test]
    fn test_alloc_returns_nonnull() {
        let mut buffer = [0u8; 1024];
        let mut alloc = make_allocator(&mut buffer);
        let p = alloc.alloc(8, 8);
        assert!(!p.is_null());
    }

    // 2. returned pointer is correctly aligned
    #[test]
    fn test_alloc_alignment() {
        let mut buffer = [0u8; 1024];
        let mut alloc = make_allocator(&mut buffer);
        for align in [1, 2, 4, 8, 16, 32, 64] {
            let mut buffer = [0u8; 1024];
            let mut alloc = make_allocator(&mut buffer);
            let p = alloc.alloc(1, align);
            assert!(!p.is_null());
            assert_eq!(p as usize % align, 0, "misaligned for align={}", align);
        }
    }

    // 3. two allocations do not overlap
    #[test]
    fn test_no_overlap() {
        let mut buffer = [0u8; 1024];
        let mut alloc = make_allocator(&mut buffer);
        let p1 = alloc.alloc(64, 8);
        let p2 = alloc.alloc(64, 8);
        assert!(!p1.is_null());
        assert!(!p2.is_null());
        assert!(p2 as usize >= p1 as usize + 64);
    }

    // 4. allocations are within buffer bounds
    #[test]
    fn test_within_bounds() {
        let mut buffer = [0u8; 1024];
        let mut alloc = make_allocator(&mut buffer);
        let p = alloc.alloc(100, 8);
        assert!(!p.is_null());
        assert!(p as usize >= buffer.as_ptr() as usize);
        assert!(p as usize + 100 <= buffer.as_ptr() as usize + 1024);
    }

    // 5. alloc past end returns null
    #[test]
    fn test_alloc_exhaustion() {
        let mut buffer = [0u8; 64];
        let mut alloc = make_allocator(&mut buffer);
        let p1 = alloc.alloc(64, 1);
        assert!(!p1.is_null());
        let p2 = alloc.alloc(1, 1);
        assert!(p2.is_null());
    }

    // 6. reset allows reuse from start
    #[test]
    fn test_reset() {
        let mut buffer = [0u8; 1024];
        let mut alloc = make_allocator(&mut buffer);
        let p1 = alloc.alloc(512, 8);
        assert!(!p1.is_null());
        alloc.reset();
        let p2 = alloc.alloc(512, 8);
        assert!(!p2.is_null());
        assert_eq!(p1, p2);
    }

    // 7. multiple allocations fill buffer correctly
    #[test]
    fn test_fills_buffer() {
        let mut buffer = [0u8; 1024];
        let mut alloc = make_allocator(&mut buffer);
        let mut count = 0;
        loop {
            let p = alloc.alloc(64, 1);
            if p.is_null() {
                break;
            }
            count += 1;
        }
        assert_eq!(count, 16);
    }

    // 8. write and read back through pointer
    #[test]
    fn test_write_read() {
        let mut buffer = [0u8; 1024];
        let mut alloc = make_allocator(&mut buffer);
        let p = alloc.alloc(8, 8) as *mut u64;
        assert!(!p.is_null());
        unsafe {
            core::ptr::write(p, 0xDEADBEEF);
            assert_eq!(core::ptr::read(p), 0xDEADBEEF);
        }
    }

    // 9. zero size alloc returns null
    #[test]
    fn test_zero_size() {
        let mut buffer = [0u8; 1024];
        let mut alloc = make_allocator(&mut buffer);
        let p = alloc.alloc(0, 1);
        assert!(p.is_null());
    }

    // 10. large alignment handled correctly
    #[test]
    fn test_large_alignment() {
        let mut buffer = [0u8; 4096];
        let mut alloc = make_allocator(&mut buffer);
        let p = alloc.alloc(8, 256);
        assert!(!p.is_null());
        assert_eq!(p as usize % 256, 0);
    }

    // 11. alignment padding does not leak into next allocation
    #[test]
    fn test_alignment_padding() {
        let mut buffer = [0u8; 1024];
        let mut alloc = make_allocator(&mut buffer);
        let p1 = alloc.alloc(1, 1);
        let p2 = alloc.alloc(8, 8);
        assert!(!p1.is_null());
        assert!(!p2.is_null());
        assert_eq!(p2 as usize % 8, 0);
        assert!(p2 as usize > p1 as usize);
    }

    // 12. reset then fill again works correctly
    #[test]
    fn test_reset_and_refill() {
        let mut buffer = [0u8; 1024];
        let mut alloc = make_allocator(&mut buffer);
        for _ in 0..16 {
            let p = alloc.alloc(64, 1);
            assert!(!p.is_null());
        }
        assert!(alloc.alloc(1, 1).is_null());
        alloc.reset();
        for _ in 0..16 {
            let p = alloc.alloc(64, 1);
            assert!(!p.is_null());
        }
    }
}
