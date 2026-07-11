/// Error types used by the arena allocator.
///
/// The allocator exposes fallible operations through `Result` so allocation
/// failure, unsupported zero-sized allocations, and arithmetic overflow can be
/// reported without panicking in the core allocation path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]

/// Errors that can occur while allocating or resizing arena memory.
pub enum AllocatorError {
    /// The platform could not provide a new memory mapping.
    OutOfMemory,

    /// The requested layout has size zero.
    ZeroSizedType,

    /// A general allocation operation failed.
    AllocationFailed,

    /// Arithmetic overflow occurred while computing sizes, alignments, or
    /// cursor positions.
    Overflow,
}
impl AllocatorError {
    /// Converts an allocator error into a panic.
    ///
    /// This is used by convenience APIs such as `alloc`, while lower-level
    /// fallible APIs return the error directly.
    pub fn panic(&self) -> ! {
        panic!("{}", self.to_string())
    }
}

impl std::fmt::Display for AllocatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfMemory => {
                write!(f, "not enough memory available for allocation")
            }
            Self::ZeroSizedType => {
                write!(f, "cannot allocate a zero-sized type")
            }
            Self::AllocationFailed => {
                write!(f, "memory allocation failed")
            }
            Self::Overflow => {
                write!(f, "arithmetic overflow while calculating allocation size")
            }
        }
    }
}

impl std::error::Error for AllocatorError {}
