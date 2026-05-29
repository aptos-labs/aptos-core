# Value Representation

Values are represented flat in memory. All values created by the VM and any modifications are allocated in the transaction's memory region. This document describes the concrete memory layouts used by the runtime.

## Primitives

Primitives (`u8`, `u16`, `u32`, `u64`, `u128`, `u256`, `bool`, `address`, `signer`) are stored as N bytes flat — no header, no indirection.

```
┌─────────────┐
│    value    │  N bytes
└─────────────┘
```

For the alignment story (per-primitive alignments, references, and how it flows through regions), see [`memory_alignment.md`](memory_alignment.md).

## Heap Object Header

All heap objects (structs, enums, vectors, and any future heap types) share a universal 8-byte header: the object's `InternedType` pointer. The header lives at a *negative* offset relative to the object pointer that callers hold — `obj_ptr` points at the start of the data region, and the header sits in the 8 bytes immediately before it:

```
                obj_ptr
                   │
                   ▼
┌────────────────┬──────────────────────────┐
│  InternedType  │  data region             │
└────────────────┴──────────────────────────┘
   header (-8..0)
```

The header is the object's interned type (a non-null arena pointer, 8 bytes). The runtime treats it as an opaque key: it never dereferences the type, only resolves it to a descriptor through `DescriptorProvider::descriptor_id_for_type`. The descriptor tells the GC how to trace internal pointers and (together with the stored vector capacity) how big the object is, so the GC can skip over objects during linear heap scanning (Cheney's algorithm). The header no longer stores `desc_id` or `size` — both are looked up from the type.

A **null** header word marks a *forwarded* object during GC (an `InternedType` is never null). The forwarding pointer is parked at offset 0 of the data region, as before.

The allocator reserves `OBJECT_HEADER_SIZE = MAX_ALIGN` bytes before each data region so that `obj_ptr` is itself `MAX_ALIGN`-aligned. When `MAX_ALIGN > 8`, the unused bytes precede the type word (which stays adjacent to the data at offset `-8`); the negative-offset constant doesn't shift.

Treating the header as allocator-only bookkeeping means per-type layouts (struct fields, enum tag/variants, vector length/data, closures, captured data) describe their data region exclusively — field 0 of a heap struct is at offset 0, the vector length is at offset 0, etc.

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

An 8-byte pointer in the enclosing memory region points to the data region of a heap object. The header sits in the bytes immediately preceding the pointer's target.

```
Owner region                       Heap
┌────────┐                  ┌──────────────────────────┐
│        │                  │ InternedType             │  header (-8..0)
│   ●────┼─────────────────►│ field_0                  │  obj_ptr
│        │                  │ field_1                  │
└────────┘                  │ ...                      │
  8 bytes                   └──────────────────────────┘
```

The header type resolves to an `ObjectDescriptor::Struct`, which tells the GC which field offsets hold heap pointers.

For field alignment within heap structs, see [`memory_alignment.md`](memory_alignment.md).

## Enums

The runtime will support both inline and heap enums.

### Inline enums

Inline enums are zero-padded so all variants occupy the same size, giving a fixed-width representation. The tag and variant fields are laid out directly in the enclosing memory region.

### Heap enums

The current runtime implements only heap enums:

```
Owner region                       Heap
┌────────┐                  ┌──────────────────────────┐
│        │                  │ InternedType             │  header (-8..0)
│   ●────┼─────────────────►│ tag                      │  obj_ptr
│        │                  ├──────────────────────────┤
└────────┘                  │ variant fields           │
  8 bytes                   └──────────────────────────┘
```

The GC traces enums via `ObjectDescriptor::Enum`, which provides per-variant pointer offset lists indexed by the tag.

The exact layout of heap enums is not fully settled. The current implementation pads all variants to the maximum variant size, but an alternative approach is to allocate a new heap object when switching from one variant to another, which would allow each variant to be sized exactly. For tag size and alignment, see [`memory_alignment.md`](memory_alignment.md).

Enums may need to stay on the heap for now because Move allows adding new variants via compatible module upgrades, which can change the layout. We are aiming to support inline enums and are considering introducing attributes like `[frozen]` that guarantee no future variants will be added, enabling inline representations.

*Note*: For simple enums with explicit representation (e.g., `#[repr(u64)]`), heap allocation should be avoided entirely — this could be enforced at the language level.

## Vectors

An 8-byte pointer in the enclosing memory region points to a heap object (or null for an empty/uninitialized vector).

```
Owner region                  Heap
┌────────┐             ┌──────────────────────────┐
│        │             │ InternedType             │  header (-8..0)
│   ●────┼────────────►│ length (u64)             │  obj_ptr (offset 0)
│        │             │ capacity (u64)           │  VEC_CAPACITY_OFFSET (8)
│        │             ├──────────────────────────┤
└────────┘             │ elem_0                   │  VEC_DATA_OFFSET (16)
  8 bytes              │ elem_1                   │
  (or null             │ ...                      │
   if empty)           └──────────────────────────┘
```

The vector's metadata (length and capacity) lives on the heap alongside the element data. The GC needs the length to know how many elements to trace for inner pointers, and the capacity to compute the object's total size (`OBJECT_HEADER_SIZE + VEC_DATA_OFFSET + capacity * elem_size`) — since the header no longer stores the size, the capacity is stored explicitly in the body at `VEC_CAPACITY_OFFSET = 8`, and element data begins at `VEC_DATA_OFFSET = 16`.

A null pointer represents an empty vector with no heap allocation. `VecNew` writes null; the first `VecPushBack` allocates lazily.

The header type resolves to an `ObjectDescriptor::Vector`, which stores the element size and the byte offsets within each element that hold heap pointers, so the GC can trace element pointers.

For element alignment within vectors, see [`memory_alignment.md`](memory_alignment.md).

## Composite Layouts

### Heap struct containing a vector field

```
Owner region         Heap (struct)                Heap (vector)
┌────────┐    ┌──────────────────────┐     ┌──────────────────────┐
│        │    │ InternedType         │     │ InternedType         │
│   ●────┼───►│ some_field           │     │ length (u64)         │
└────────┘    │ vec_ptr  ●───────────┼────►│ elem_0               │
              └──────────────────────┘     │ elem_1               │
                                           │ ...                  │
                                           └──────────────────────┘
```

### Heap struct containing another heap struct

```
Owner region         Heap (outer)               Heap (inner)
┌────────┐    ┌──────────────────────┐    ┌──────────────────────┐
│        │    │ InternedType         │    │ InternedType         │
│   ●────┼───►│ inner_ptr ●──────────┼───►│ field_0              │
└────────┘    │ other_field          │    │ field_1              │
              └──────────────────────┘    └──────────────────────┘
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
  ref = (heap_ptr, field_offset)
```

Base points to the heap object's data region. The field offset is the byte offset of the field within the data region (the header is at a negative offset and is invisible to references). The GC can relocate the struct and update the base pointer; the field offset stays the same.

### Reference to a vector element

```
  ref = (vec_heap_ptr, VEC_DATA_OFFSET + idx * elem_size)
```

Base points to the vector's heap object. `VEC_DATA_OFFSET` (= 16) skips the length and capacity fields at the start of the data region. Safe as long as no growth occurs while the borrow is live (enforced by Move's borrow checker).

### Alternative: raw pointer references

An alternative representation is to use a single raw pointer that points directly to the target value, without a base/offset split. This is more compact (8 bytes instead of 16) and avoids the offset addition on every access.

The trade-off is that the GC can no longer trivially identify which heap object the reference points into. Instead, the GC would need to recover the base address of the containing object — e.g., via binary search over the set of live heap objects. This adds complexity to the GC implementation (base-address recovery, correct handling of interior pointers during relocation) and makes it harder to reason about correctness. The added code complexity is probably the bigger concern, though the performance cost of base-address recovery during GC is also worth measuring.

## Vector Growth and Why Operations Go Through References

When a push exceeds capacity, the vector must be reallocated: a new, larger heap object is allocated, the data is copied over, and the old object is abandoned (reclaimed by GC). This means the heap pointer to the vector changes on growth.

The problem is that the vector pointer may live in different places — a local on the stack, a field inside a heap struct, a field inside another vector's element, etc. The code performing the push needs to write the updated pointer back to wherever the vector is owned. If vector operations took a direct pointer to the heap object, there would be no way to update the owner after reallocation.

For this reason, vector operations (`VecPushBack`, `VecPopBack`, etc.) operate through a **fat pointer reference** (`vec_ref`) that points to the slot holding the vector's heap pointer. After reallocation, the new heap pointer is written back through this reference, updating the owner in place — regardless of whether the owner is a stack local, a struct field, or anything else.

## Function Values / Closures

TBD.
