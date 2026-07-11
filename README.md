# aruna

`aruna` is an arena allocator written in Rust.

It requests memory from the operating system with `mmap`, stores block metadata in-band, and serves allocations by moving a cursor through the current block.

Run the tests:

```bash
cargo test
```

If your environment uses `sccache` and it gets in the way:

```bash
RUSTC_WRAPPER= cargo test
```

## Implementation

The allocator starts empty. The first allocation maps a block, writes a header at the beginning, and places the cursor after that header.

```text
+--------+-------------------------------+
| Header | allocation space              |
+--------+-------------------------------+
         ^
         cursor
```

Fast allocation is just cursor movement:

```text
                Cursor                End                                       Cursor  End
                   |                   |                                          |       |
                   v                   v                                          v       v
+------+-----+-----+-------------------+  Allocate C   +------+-----+-----+-------+---------+
|Header|  A  |  B  |       free        | ------------> |Header|  A  |  B  |   C   |  free   |
+------+-----+-----+-------------------+               +------+-----+-----+-------+---------+
```

When the current block cannot fit the request, the arena maps a new block and links it to the previous one.

```text
+------+-----+-----+-------+---------+
|Header|  A  |  B  |   C   |  free   |
+------+-----+-----+-------+---------+
   ^
   |
   +-------------------+
                       |
                       v
              +------+---------+-------------------+
              |Header|    D    |       free        |
              +------+---------+-------------------+
                     ^
                     cursor
```

Each block starts with:

```rust
BlockHeader {
    prev,
    mmap_ptr,
    mmap_size,
}
```

`reset` keeps the current block, frees older blocks, and rewinds the cursor.

```text
Before reset:

+--------+      +--------+      +----------------+
| 4 KiB  | ---> | 8 KiB  | ---> | current 16 KiB |
+--------+      +--------+      +----------------+

After reset:

+----------------+
| current 16 KiB |
+----------------+
         ^
         cursor
```

## Resizing

`grow` tries to resize the last allocation in place.

```text
+------+-----+-----+-------+---------+
|Header|  A  |  B  |   C   |  free   |
+------+-----+-----+-------+---------+
                         ^
                         cursor

+------+-----+-----+-----------------+
|Header|  A  |  B  |   C grown       |
+------+-----+-----+-----------------+
```

If the allocation is not last, `grow` allocates a new region and copies the old bytes.

`shrink` is simpler: if the allocation is last, the cursor moves backward. Otherwise, the arena keeps the memory as-is until reset.

## Current API

- `Arena::new`
- `alloc`
- `try_allocate`
- `alloc_val`
- `reset`
- `clear`
- `grow`
- `shrink`
