#![allow(warnings)]
use arena::{Arena, EMPTY_BLOCK};
use std::{alloc::Layout, mem::ManuallyDrop};
mod platform;
use platform::Platform;

fn main() {
    let mut r1 = Arena::new();
    let layout = Layout::new::<i32>();
    let ptr = r1.alloc(layout);
    assert!(r1.is_last_allocation(ptr, layout.size()));

    unsafe {
        r1.grow(
            ptr,
            layout,
            Layout::from_size_align_unchecked(128, layout.align()),
        );
    }
}
