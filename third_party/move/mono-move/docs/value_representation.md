# Value Representation

Values are represented flat in memory. All values created by the VM and any modifications are allocated in the transaction's memory region. This document describes the concrete memory layouts used by the runtime.

## Primitives

Primitives (`u8`, `u16`, `u32`, `u64`, `u128`, `u256`, `bool`, `address`, `signer`) are stored as N bytes flat — no header, no indirection.

```
┌─────────────┐
│    value    │  N bytes
└─────────────┘
```

Alignment is still under consideration. There will likely be some form of alignment requirement, but the exact scheme has not been finalized.

## Heap Object Header

All heap objects (structs, enums, vectors, and any future heap types) share a universal 8-byte header:

```
┌──────────────────────────┐
│ desc_id(4) | size(4)     │  8 bytes
└──────────────────────────┘
```

- `desc_id` (`u32`): indexes into the descriptor table, telling the GC how to trace internal pointers.
- `size` (`u32`): total object size in bytes (header + data, 8-byte aligned), so the GC can skip over objects during linear heap scanning (Cheney's algorithm).

## Structs

The runtime supports both inline and heap structs.

### Inline structs

Fields are laid out directly at compile-time-determined offsets in the enclosing memory region (stack frame, or the data area of another heap object). No heap allocation, no pointer indirection — field access is a direct base + offset load.

```
┌─────────────────────────────────┐
│ field_0 │ field_1 │ field_2 ... │  N bytes total, flat
└─────────────────────────────────┘
```

Inline structs may not be explicitly reflected in runtime code, as they are already supported by the low-level micro-ops (data movement, borrows with offset arithmetic). No special runtime support is needed. Additional micro-ops may be added to make them more efficient. The compiler has significant control over their implementation.

### Heap structs

An 8-byte pointer in the enclosing memory region points to a heap object with the standard header followed by fields.

```
Owner region                       Heap
┌────────┐                  ┌──────────────────────────┐
│   ●────┼─────────────────►│ desc_id(4) | size(4)     │  header
│        │                  ├──────────────────────────┤
└────────┘                  │ field_0                  │
  8 bytes                   │ field_1                  │
                            │ ...                      │
                            └──────────────────────────┘
```

The `desc_id` indexes into the descriptor table so the GC knows which field offsets hold heap pointers (`ObjectDescriptor::Struct`).

Alignment of fields within heap structs is also under consideration.

## Enums

The runtime will support both inline and heap enums.

### Inline enums

Inline enums are zero-padded so all variants occupy the same size, giving a fixed-width representation. The tag and variant fields are laid out directly in the enclosing memory region.

### Heap enums

The current runtime implements only heap enums:

```
Owner region                       Heap
┌────────┐                  ┌──────────────────────────┐
│   ●────┼─────────────────►│ desc_id(4) | size(4)     │  header
│        │                  ├──────────────────────────┤
└────────┘                  │ tag                      │
  8 bytes                   ├──────────────────────────┤
                            │ variant fields           │
                            └──────────────────────────┘
```

The GC traces enums via `ObjectDescriptor::Enum`, which provides per-variant pointer offset lists indexed by the tag.

The exact layout of heap enums is not fully settled. The current implementation pads all variants to the maximum variant size, but an alternative approach is to allocate a new heap object when switching from one variant to another, which would allow each variant to be sized exactly. The tag size and alignment are also under consideration.

Enums may need to stay on the heap for now because Move allows adding new variants via compatible module upgrades, which can change the layout. We are aiming to support inline enums and are considering introducing attributes like `[frozen]` that guarantee no future variants will be added, enabling inline representations.

*Note*: For simple enums with explicit representation (e.g., `#[repr(u64)]`), heap allocation should be avoided entirely — this could be enforced at the language level.

## Vectors

An 8-byte pointer in the enclosing memory region points to a heap object (or null for an empty/uninitialized vector).

```
Owner region                  Heap
┌────────┐             ┌──────────────────────────┐
│   ●────┼────────────►│ desc_id(4) | size(4)     │  header
│        │             ├──────────────────────────┤
└────────┘             │ length (u64)             │
  8 bytes              ├──────────────────────────┤
  (or null             │ elem_0                   │
   if empty)           │ elem_1                   │
                       │ ...                      │
                       └──────────────────────────┘
```

The vector's metadata (length) lives on the heap alongside the element data. The GC needs the length to know how many elements to trace for inner pointers. Capacity is not stored explicitly — it is derived from the header: `cap = (size - VEC_DATA_OFFSET) / elem_size`.

A null pointer represents an empty vector with no heap allocation. `VecNew` writes null; the first `VecPushBack` allocates lazily.

The `desc_id` tells the GC how to trace element pointers via `ObjectDescriptor::Vector`, which stores the element size and the byte offsets within each element that hold heap pointers.

Alignment of elements within vectors is also under consideration.

## Composite Layouts

### Heap struct containing a vector field

```
Owner region         Heap (struct)                  Heap (vector)
┌────────┐    ┌──────────────────────┐       ┌──────────────────────┐
│   ●────┼───►│ desc_id(4) | size(4) │       │ desc_id(4) | size(4) │
└────────┘    ├──────────────────────┤       ├──────────────────────┤
              │ some_field           │       │ length (u64)         │
              │ vec_ptr  ●───────────┼──────►│ elem_0               │
              └──────────────────────┘       │ elem_1               │
                                             │ ...                  │
                                             └──────────────────────┘
```

### Heap struct containing another heap struct

```
Owner region         Heap (outer)                    Heap (inner)
┌────────┐    ┌──────────────────────┐        ┌──────────────────────┐
│   ●────┼───►│ desc_id(4) | size(4) │        │ desc_id(4) | size(4) │
└────────┘    ├──────────────────────┤        ├──────────────────────┤
              │ inner_ptr ●──────────┼───────►│ field_0              │
              │ other_field          │        │ field_1              │
              └──────────────────────┘        └──────────────────────┘
```

## References (Fat Pointers)

References are 16-byte fat pointers: `(base_ptr: *mut u8, byte_offset: u64)`. The actual target address is `base_ptr + byte_offset`.

```
The ref                          Target memory
┌────────┐                       ┌────────┐
│ base ●─┼──────────────────────►│ ...    │
│ offset │                       │ value  │  ← base + offset
└────────┘                       │ ...    │
  16 bytes                       └────────┘
```

The base pointer is what the GC tracks and updates during collection; the byte offset is a scalar that remains stable across GC moves. Only the base pointer half is listed in `pointer_offsets` for GC scanning.

### Reference to a stack local

```
  ref = (slot_ptr, 0)
```

The base pointer points directly to the local's slot. The offset is always 0. Since the pointer address belongs to the stack (not the heap), the GC will not attempt to trace or relocate it.

### Reference to a heap struct field

```
  ref = (heap_ptr, STRUCT_DATA_OFFSET + field_offset)
```

Base points to the heap object. The GC can relocate the struct and update the base pointer; the field offset stays the same.

### Reference to a vector element

```
  ref = (vec_heap_ptr, VEC_DATA_OFFSET + idx * elem_size)
```

Base points to the vector's heap object. Safe as long as no growth occurs while the borrow is live (enforced by Move's borrow checker).

### Alternative: raw pointer references

An alternative representation is to use a single raw pointer that points directly to the target value, without a base/offset split. This is more compact (8 bytes instead of 16) and avoids the offset addition on every access.

The trade-off is that the GC can no longer trivially identify which heap object the reference points into. Instead, the GC would need to recover the base address of the containing object — e.g., via binary search over the set of live heap objects. This adds complexity to the GC implementation (base-address recovery, correct handling of interior pointers during relocation) and makes it harder to reason about correctness. The added code complexity is probably the bigger concern, though the performance cost of base-address recovery during GC is also worth measuring.

## Vector Growth and Why Operations Go Through References

When a push exceeds capacity, the vector must be reallocated: a new, larger heap object is allocated, the data is copied over, and the old object is abandoned (reclaimed by GC). This means the heap pointer to the vector changes on growth.

The problem is that the vector pointer may live in different places — a local on the stack, a field inside a heap struct, a field inside another vector's element, etc. The code performing the push needs to write the updated pointer back to wherever the vector is owned. If vector operations took a direct pointer to the heap object, there would be no way to update the owner after reallocation.

For this reason, vector operations (`VecPushBack`, `VecPopBack`, etc.) operate through a **fat pointer reference** (`vec_ref`) that points to the slot holding the vector's heap pointer. After reallocation, the new heap pointer is written back through this reference, updating the owner in place — regardless of whether the owner is a stack local, a struct field, or anything else.

## Function Values / Closures

TBD.
