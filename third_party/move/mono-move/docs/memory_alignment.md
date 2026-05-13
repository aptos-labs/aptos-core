# Memory Alignment

This document specifies how MonoMove aligns values in memory: where each primitive sits, how composite layouts are derived, and how the alignment requirement flows through the regions the VM owns (heap, stack, hardcoded headers). It also covers how alignment is enforced, optimization opportunities, and the configurability we want to keep open for the future.

For the layouts themselves (without the alignment lens), see [`value_representation.md`](value_representation.md). For the regions, see [`heap_and_gc.md`](heap_and_gc.md) and [`stack_and_calling_convention.md`](stack_and_calling_convention.md).

## 1. Principles

1. **Natural alignment for small types.** A value of size ≤ 8 bytes is placed at an address that is a multiple of its size (`bool` counts as 1 byte). This is what Rust and C++ do for primitives and is the alignment most hardware loads expect.

2. **Cap alignment at 8 bytes.** All Move primitives 8 bytes or larger (`u64`, `u128`, `u256`, `address`, `signer`) get 8-byte alignment, even when their size exceeds 8. This avoids the cascading padding that stronger alignment would impose on every container — the trade-off is discussed in [§8.2](#82-stronger-alignment-for-u128--u256--address--signer).

3. **Layout is a VM-internal decision.** BCS — the canonical on-disk encoding — is alignment-free, so the in-memory layout can be reshuffled on each round-trip through storage with no compatibility cost.

4. **Alignment is uniform within a build.** Two MonoMove instances of the same build produce identical layouts. We keep one global `MAX_ALIGN` (see [§2](#2-the-max_align-constant)) so frames and heap regions share a single alignment guarantee.

5. **Stay general.** 8 bytes is today's de facto bound, but hardcoding `8` everywhere would make any future bump expensive. We thread `MAX_ALIGN` and per-type alignment through the layout pipeline so raising the cap later is a constant change rather than a structural rewrite.

## 2. The `MAX_ALIGN` Constant

`MAX_ALIGN` is a single `usize` constant in `mono-move-core`, defaulting to **8**. It is referenced throughout this document as the bound on every alignment guarantee the VM provides.

It governs the alignment of the stack and heap regions, of `fp` and the bump pointer, and the padding rounded into per-object `size` fields and into frame segments. [§3](#3-vm-managed-memory-regions) describes how each region maintains this guarantee in practice.

**Constraints on `MAX_ALIGN`:**

- It must be a power of two.
- It must be at least **8**, because the heap object header (8 bytes) and the `params + locals + metadata` region (which contains the 24-byte metadata block) both need 8-byte granularity at minimum.
- It must be a multiple of every alignment used by any value or VM-internal layout. With current alignments (1, 2, 4, 8) this is satisfied by any multiple of 8.

**Raising `MAX_ALIGN` resizes the heap header reservation automatically.** Per [§5.1](#51-heap-object-header), `OBJECT_HEADER_SIZE = MAX_ALIGN` — the allocator reserves at least `MAX_ALIGN` bytes before each data region so that the object pointer (and therefore field 0) lands on a `MAX_ALIGN` boundary. The descriptor/size pair always lives in the *last* 8 bytes of that reservation (at `obj_ptr - 8` / `obj_ptr - 4`), independent of `MAX_ALIGN`. Per-type layouts (struct field offsets, enum data offset, vector data offset, closure fields, captured-data values) are all expressed relative to the data start, so none of them shift when `MAX_ALIGN` grows. Costs to keep in mind:

- **Per-object padding when `MAX_ALIGN > 8`.** A 16-byte `MAX_ALIGN` adds 8 bytes of header padding (header reservation goes from 8 → 16 bytes); a 32-byte `MAX_ALIGN` adds 24. The padding pays for "data start is always `MAX_ALIGN`-aligned" without needing per-container layout changes.
- **Frame metadata.** The 24-byte metadata block is not `MAX_ALIGN`-aligned in size when `MAX_ALIGN > 8`. The frame layout pass must pad the metadata up to `MAX_ALIGN` so the next callee's `fp` lands on an aligned offset.
- **Vector data offset.** Element 0 starts at `VEC_DATA_OFFSET = 8` within the data region (after the length field). For elements requiring more than 8-byte alignment, the data offset would need to grow per vector — but the current vector micro-ops have no mechanism to express this. See [§8.3](#83-vector-data-offset-under-stronger-element-alignment).

**Natives inherit `MAX_ALIGN`.** Native functions follow the same calling convention and read parameters from the frame at the same offsets. Any pointer they receive into the stack or heap inherits the `MAX_ALIGN` (and below) alignment guarantees. If a native ever needs stronger alignment than `MAX_ALIGN` for a Rust-side type, it must allocate its own scratch space — the VM frame won't satisfy it.

## 3. VM-Managed Memory Regions

The heap and the stack each live in a `MemoryRegion` whose base address is allocated `MAX_ALIGN`-aligned by construction. The sections below describe the per-region mechanics that maintain alignment from the base outward.

### 3.1 The Heap

The bump allocator advances by aligned steps. For each `heap_alloc` call, it rounds the bump pointer up to the object's alignment (≤ `MAX_ALIGN`), reserves the requested size, and then rounds the bump pointer up to `MAX_ALIGN` so the next allocation starts on a `MAX_ALIGN`-aligned address. The size recorded in the object header is the post-padding size, so Cheney's linear heap scan steps from one header to the next without alignment correction.

The current implementation rounds every allocation up to `MAX_ALIGN` (via `align_max`). A future change will plumb per-allocation alignment through `heap_alloc` so that allocations smaller than `MAX_ALIGN` don't pay full padding when `MAX_ALIGN` is raised.

### 3.2 The Stack

**Frame pointer (`fp`) alignment.** Every `fp` value is `MAX_ALIGN`-aligned. This is the load-bearing invariant: any local at offset `o` from `fp` is naturally aligned as long as `o` itself is a multiple of the local's alignment, and the same goes for parameters and return values.

**Frame segments and their alignment.**

```
   fp
   │
   ▼
   ├── params ──┼── locals ──┼── meta(24) ──┼── callee args/returns ──┤
   0          p_sum        pl_sum        pl_sum + 24              extended
                                            ▲
                                            │
                                    callee's fp lands here
```

- **Parameters** (`[0, p_sum)`) start at offset 0. Each parameter sits at its natural alignment relative to `fp` (which is `MAX_ALIGN`-aligned).
- **Locals** (`[p_sum, pl_sum)`) continue immediately after parameters with the same natural-alignment rule. The layout pass rounds the locals' end up to 8 so the metadata that follows is 8-byte aligned.
- **Frame metadata** (`[pl_sum, pl_sum + FRAME_METADATA_SIZE)`) is a `FRAME_METADATA_SIZE`-byte block holding `(saved_pc: u64, saved_fp: ptr, saved_func_ptr: ptr)` and requires 8-byte alignment. Today `FRAME_METADATA_SIZE = 24`.
- **Callee arg/return region** (`[pl_sum + FRAME_METADATA_SIZE, extended)`) doubles as the start of the callee's frame, so the callee's `fp = caller_fp + pl_sum + FRAME_METADATA_SIZE`.

For the callee's `fp` to be `MAX_ALIGN`-aligned given the caller's is, `pl_sum + FRAME_METADATA_SIZE` must be a multiple of `MAX_ALIGN`. With `MAX_ALIGN = 8` this is automatic (`FRAME_METADATA_SIZE = 24` is divisible by 8 and `pl_sum` is already 8-aligned by the locals layout). With `MAX_ALIGN > 8`, the metadata block needs to be padded up to `MAX_ALIGN` bytes — see [§2](#2-the-max_align-constant).

Note that `extended_frame_size` itself does *not* need to be `MAX_ALIGN`-aligned; only the `pl_sum + FRAME_METADATA_SIZE` boundary that the callee's `fp` lands on does. Multiple calls from the same caller all reuse the same callee region, and the caller's frame doesn't propagate any further alignment constraint after the metadata.

## 4. Primitive Types

Proposed (size, alignment) for each primitive, in bytes:

| Type | Size | Alignment |
|---|---|---|
| `bool`, `u8`, `i8` | 1 | 1 |
| `u16`, `i16` | 2 | 2 |
| `u32`, `i32` | 4 | 4 |
| `u64`, `i64` | 8 | 8 |
| `u128`, `i128` | 16 | 8 |
| `u256`, `i256` | 32 | 8 |
| `address` | 32 | 8 |
| `signer` | 32 | 8 |

Up through `u64` we follow Rust / C++'s natural alignment. From `u128` onwards we cap at 8: stronger alignment would force padding into every container holding such a field, which is a cost we don't want to pay by default (see [§8.2](#82-stronger-alignment-for-u128--u256--address--signer) for when it might be worth revisiting).

### References

References are 16-byte fat pointers `(base_ptr: 8B, byte_offset: 8B)`. Both halves are 8-byte aligned, so the reference itself is 8-byte aligned.

| Type | Size | Alignment |
|---|---|---|
| `&T`, `&mut T` | 16 | 8 |

### Function values / closures

Stored as a heap pointer in the owner region. 8 bytes, 8-byte aligned.

## 5. Composite Layouts

### 5.1 Heap Object Header

Every heap object has an 8-byte header `[desc_id: u32 | size: u32]` that sits at a *negative* offset relative to the object pointer that callers hold (`desc_id` at `obj_ptr - 8`, `size` at `obj_ptr - 4`). The allocator reserves `OBJECT_HEADER_SIZE = MAX_ALIGN` bytes before each data region, so the header is `MAX_ALIGN`-aligned for free (the bump pointer is `MAX_ALIGN`-aligned per [§3.1](#31-the-heap)) and the descriptor+size pair stays adjacent to the data — good for cache locality. When `MAX_ALIGN > 8`, the lower bytes of the reservation are unused padding; the negative offsets `-8` / `-4` don't shift.

### 5.2 Heap Object Body

The body starts at the object pointer (offset 0 of the data region). Body fields are placed at offsets that satisfy each field's natural alignment relative to `obj_ptr`. Because the allocator guarantees `obj_ptr` is `MAX_ALIGN`-aligned, the first field can sit at offset 0 regardless of its alignment (which is ≤ `MAX_ALIGN` by construction); raising `MAX_ALIGN` does not require any per-type layout changes.

### 5.3 Inline Structs

Inline structs have no header. They are laid out flat in their owner region (a stack frame slot, or another heap object's body). Field offsets are computed relative to the start of the struct, with each field at its natural alignment.

The struct's alignment is the max of its fields' alignments. The owner is responsible for placing the struct at an offset satisfying that alignment. The struct's size is rounded up to its alignment so that a sequence of structs (e.g., elements of a vector) packs cleanly.

### 5.4 Heap Structs

Heap structs follow [§5.1](#51-heap-object-header)–[§5.2](#52-heap-object-body). Field offsets *within the body* are computed relative to `obj_ptr` (offset 0), using the same natural-alignment rule as inline structs. The compiler-provided `pointer_offsets` for the GC are body-relative offsets.

### 5.5 Inline Enums

(Not yet implemented; design preview.)

An inline enum has the layout `[tag | pad | variant_region]`, where:

- The **tag** is conceptually 1 byte (a single discriminant). In current implementations of heap enums it is stored as `u64`, but that's the result of manual alignment rather than a tag-size requirement.
- The **variant region** is sized to the largest variant and aligned to the largest alignment among any variant's fields. Smaller variants are zero-padded to the variant-region size.
- Padding between the tag and the variant region brings the variant region to its alignment boundary.

The whole enum's alignment is `max(tag_align, variant_region_align)` — i.e., the variant-region alignment unless the variant region is empty. The total size is `align_up(tag_size, variant_region_align) + variant_region_size`, rounded up to the enum's overall alignment so a sequence of enum values packs cleanly.

### 5.6 Heap Enums

Same as [§5.4](#54-heap-structs), with the body laid out as `[tag | pad | variant_fields…]`. Today the runtime stores the tag as a `u64` for convenience, which is itself the result of manual alignment up to 8. Conceptually the tag is 1 byte; the extra 7 bytes are padding before the variant region (whose max field alignment is ≤ `MAX_ALIGN`). Treating the tag as 1-byte conceptually keeps the door open to shrinking the storage if a future micro-optimization wants to reclaim those bytes for a small variant field.

### 5.7 Vectors

```
          obj_ptr    obj_ptr + VEC_DATA_OFFSET
          │          │
          ▼          ▼
┌────┬────┬──────────┬──────────────────────┐
│desc│size│ length   │  elem_0  elem_1 ...  │
└────┴────┴──────────┴──────────────────────┘
 ▲    ▲    ╰──────── data region ──────────╯
 │    └ size, at obj_ptr - 4
 └ desc, at obj_ptr - 8
```

The header is at `obj_ptr - 8` / `obj_ptr - 4` ([§5.1](#51-heap-object-header)). `obj_ptr` itself points at the start of the data region, where the vector stores a `length` field (`u64`, at `VEC_LENGTH_OFFSET = 0`). Element 0 starts at `VEC_DATA_OFFSET = 8` from `obj_ptr`, which is 8-byte aligned — sufficient for any primitive under [§4](#4-primitive-types)'s 8-byte cap. Element `i` is at offset `VEC_DATA_OFFSET + i * elem_size`, where `elem_size` is rounded up to the element's alignment (so successive elements remain aligned). Within an element, fields are at their natural alignment.

This layout assumes element alignment is ≤ 8, which holds today. Stronger element alignment would require a per-vector data offset that the current micro-ops cannot express; see [§8.3](#83-vector-data-offset-under-stronger-element-alignment).

### 5.8 Closures

A closure is a heap object (see `closure_design.md` for full layout). The closure object itself has all fields at 8-byte alignment, so the object's overall alignment is 8.

A closure's captured values live in a *separate* heap object pointed to by `captured_data_ptr`. That object is also a heap object — same header at offset 0, body at offset 8. Within the body, the captured values are laid out like an inline struct (per [§5.3](#53-inline-structs)): each captured value at its natural alignment, packed with intervening padding as needed. The captured-data tag (Raw vs. Materialized) is conceptually 1 byte at the start of the body, currently stored with 7 bytes of padding before the values.

### 5.9 Worked Examples

These examples assume `MAX_ALIGN = 8` and the alignments from [§4](#4-primitive-types).

**Example 1 — simple struct, inline.**

```move
struct Foo { a: u8, b: u32, c: u64 }
```

| Field | Offset | Size | Notes |
|---|---|---|---|
| `a` | 0 | 1 | u8 |
| pad | 1–3 | 3 | align up to 4 for `b` |
| `b` | 4 | 4 | u32 |
| `c` | 8 | 8 | u64 |

Struct size = 16, alignment = 8.

**Example 2 — same struct, heap.**

Offsets below are relative to `obj_ptr` (the data start). The 8-byte header `[desc_id | size]` lives at `obj_ptr - 8` and adds 8 bytes of allocator-owned reservation per object.

| Offset | Content |
|---|---|
| -8 .. 0 | header (allocator-owned) |
| 0 | `a` (1 byte) |
| 1–3 | pad |
| 4 | `b` (4 bytes) |
| 8 | `c` (8 bytes) |
| 16 | end |

Data-region size = 16. Total object size (header + data) = 24 — a multiple of `MAX_ALIGN = 8`.

**Example 3 — nested struct, heap.**

```move
struct Inner { x: u32, y: u8 }      // align 4, size 8
struct Outer { a: u8, b: Inner }
```

Inner inline:

| Field | Offset | Size |
|---|---|---|
| `x` | 0 | 4 |
| `y` | 4 | 1 |

Inner has alignment 4 and size 8: `x` occupies offsets 0–3 and `y` offset 4, with 3 bytes of trailing padding so that successive Inners (e.g., in a vector) keep `x` 4-aligned.

Outer on the heap (offsets relative to `obj_ptr`; the header lives at `obj_ptr - 8`):

| Offset | Content |
|---|---|
| -8 .. 0 | header (allocator-owned) |
| 0 | Outer.`a` (1B) |
| 1–3 | pad (align Inner to 4) |
| 4 | Inner.`x` (4B) |
| 8 | Inner.`y` (1B) |
| 9–11 | Inner trailing pad |
| 12–15 | Outer trailing pad to `MAX_ALIGN` |

Data-region size = 16. Total object size = 24. Outer's body is 12 bytes (`a` + pad + Inner = 1 + 3 + 8); body alignment = 4.

**Example 4 — enum, heap.**

```move
enum Result {
    Ok(u64),
    Err(u32),
}
```

Variant region: max size = 8 (Ok's u64), max alignment = 8.

Offsets relative to `obj_ptr`; header at `obj_ptr - 8`.

| Offset | Content |
|---|---|
| -8 .. 0 | header (allocator-owned) |
| 0 | tag (1 byte conceptually; 8 bytes as currently stored) |
| 1–7 | pad |
| 8 | variant payload (8 bytes; `u64` for Ok, `u32` + 4B padding for Err) |
| 16 | end |

Total object size = 24, data-region size = 16.

**Example 5 — vector of struct.**

```move
struct Point { x: u32, y: u32 }   // size 8, align 4
let v: vector<Point>;             // 3 elements
```

Offsets relative to `obj_ptr`; header at `obj_ptr - 8`.

| Offset | Content |
|---|---|
| -8 .. 0 | header (allocator-owned) |
| 0–7 | length (u64) |
| 8 | element 0: `x`@8, `y`@12 |
| 16 | element 1: `x`@16, `y`@20 |
| 24 | element 2: `x`@24, `y`@28 |
| 32 | end |

Data-region size = 32. Total object size = 40 — a multiple of `MAX_ALIGN`.

## 6. Enforcement

Alignment correctness depends on three layers cooperating; if any of them is wrong the runtime can perform a misaligned access and produce undefined behavior.

1. **Specializer.** The compile-time layout pass (`LoweringContext::layout_slots` today, plus equivalent passes for struct bodies, enum variants, and captured-data values) is responsible for emitting offsets that respect each field's alignment. This is where most alignment rules are baked in: every micro-op that names a slot or field offset relies on this layout being correct. Bugs here would directly produce mis-aligned reads.

2. **Static verifier.** The runtime's verifier (`runtime/src/verifier.rs`) checks what it can statically before execution: frame-access bounds, jump targets, descriptor validity, and pointer-offset alignment within descriptors (today it requires 8-byte alignment). Extending the verifier to check that every slot-access micro-op uses an offset compatible with the slot's declared alignment is a natural defense-in-depth — it does not add runtime cost (verification is one-time per function) and would catch specializer bugs.

3. **Runtime invariants.** Three structural invariants do the rest:
   - `MemoryRegion::new` allocates with `MAX_ALIGN`, so the heap and stack base addresses are always sufficiently aligned.
   - The bump allocator ([§3.1](#31-the-heap)) advances by `MAX_ALIGN`-aligned steps, so every object's address satisfies `MAX_ALIGN`.
   - Every `fp` is `MAX_ALIGN`-aligned ([§3.2](#32-the-stack)), so frame-relative offsets composed with `fp` produce aligned addresses.

The combination of (1) computing aligned offsets, (2) verifying them statically, and (3) maintaining aligned base pointers gives end-to-end alignment safety without runtime alignment checks on the hot path.

> TODO: the specializer rounds `pl_sum` up to `MAX_ALIGN` so the callee's `fp` lands on a `MAX_ALIGN`-aligned offset ([§3.2](#32-the-stack)). This is currently a specializer-side convention with no static verifier check — a bug there silently misaligns every callee frame. Either extend the verifier to enforce `(pl_sum + FRAME_METADATA_SIZE) % MAX_ALIGN == 0`, or add a `debug_assert!` on the call path.

## 7. Optimizations

### 7.1 Field Reordering

For an arbitrary mix of field sizes, declaration-order layout leaves padding holes between a small field and a larger-aligned one. Permuting the field order can collapse those holes — the canonical strategy is to place fields in descending alignment order, which guarantees that every field sits at its natural offset with no internal padding (only tail padding to round the struct up to its overall alignment).

This is what Rust's default `repr(Rust)` does for struct layout. Move's source-level field declaration order is preserved by BCS but not visible to the runtime — the in-memory layout is free to differ — so the specializer can compute its own layout per type.

What this enables:

- **Smaller objects.** Tighter packing reduces both heap usage and frame size, lowering GC pressure and improving cache behavior.
- **No on-disk impact.** BCS continues to (de)serialize fields in declaration order via the inverse permutation. The optimization is purely in-memory.
- **Stable across builds via a deterministic tie-breaker.** Sorting on `(align desc, declaration order)` makes the permutation reproducible, which matters if it is ever exposed to consensus-visible computations (e.g., GC root layouts).

The same idea applies to inline structs, heap struct bodies, enum variant fields, and captured-data values. (Not vector elements — they're homogeneous.)

### 7.2 Local Reordering

The same argument applies to a function's locals (and home slots more generally). The specializer can permute local indexes to minimize frame size; with everything ≤ `MAX_ALIGN`-aligned the savings per frame are small (a few bytes), but they compound across deep call stacks.

Parameters are *not* reordered (they're a public ABI constrained by the calling convention), and neither are slots tied to the callee arg/return region.

## 8. Open Questions / Trade-offs

### 8.1 Evolving alignment as the VM matures

The alignment choices in this document are calibrated for what MonoMove does today. As the VM evolves — new primitives, new optimizations, SIMD-based natives, broader hardware targets — we may need to adjust these decisions. Each adjustment is a breaking change for any persisted in-memory state (the storage cache at the block level, or any in-memory representation that crosses a build boundary), although BCS round-trips remain unaffected because BCS is alignment-free.

In practice this means alignment changes need to be **feature-gated** the same way other layout-affecting upgrades are, with a clean cut-over point that invalidates and rebuilds any in-memory caches. This is tractable today (the runtime is at PoC stage, no committed in-memory cache yet), but the cost grows once consumers pin specific layouts. Designing alignment as a `MAX_ALIGN` constant plus per-type alignment metadata ([§2](#2-the-max_align-constant)) is exactly so that "alignment evolution" is a localized change rather than a structural rewrite.

### 8.2 Stronger alignment for `u128` / `u256` / `address` / `signer`

We capped alignment at 8 for these types in [§4](#4-primitive-types). The arguments for raising it:

- **SIMD-friendly loads.** `u128` at 16-byte alignment enables a single 128-bit load (SSE2). `u256` / `address` at 32-byte alignment enables a single AVX2 load.
- **Cache-line alignment.** 32-byte alignment puts each `address` on half a cache line, which can matter for hot resources accessed by many transactions.
- **Toolchain ergonomics.** Rust's native `u128` is 16-byte aligned. If a future runtime path uses typed `u128` reads, we'd need to match.

The arguments against (and why we picked 8):

- **Padding cost.** Raising any of these to 16 or 32 forces padding before the field in any container that doesn't start on a sufficiently aligned boundary — inline structs at odd offsets, etc. That overhead applies to every object containing such a field, not just SIMD-hot ones.
- **No SIMD use case today.** The hot path doesn't have wide-vector loads; the cost of higher alignment would be paid up front for unrealized future benefit.
- **Per-object header padding.** With `OBJECT_HEADER_SIZE = MAX_ALIGN`, raising `MAX_ALIGN` from 8 to 16 adds 8 bytes of unused header padding per heap object; 32 adds 24 bytes. Heap object bodies still start at `obj_ptr`, which the allocator guarantees is `MAX_ALIGN`-aligned — so no per-container layout change is needed when `MAX_ALIGN` grows. The cost is the padding, not the layout churn.

The recommendation is to revisit if profiling shows arithmetic on `u128` / `u256` is hot, or if SIMD-based crypto natives become a measurable win. The constants are centralized, so the change is mostly mechanical (with the caveats from [§2](#2-the-max_align-constant)).

> TODO: file an issue (and link it here) to benchmark the SIMD / cache-line-alignment win against the padding cost on representative workloads before deciding. Also worth considering: parallel moves and arg reshuffling may benefit from SIMD scatter/gather even without wide arithmetic.

### 8.3 Vector data offset under stronger element alignment

Vector micro-ops compute the address of element `i` as `vec_ptr + VEC_DATA_OFFSET + i * elem_size`, where `VEC_DATA_OFFSET = 8` is a global constant (the offset past the length field within the data region). This works as long as every element alignment is ≤ 8, which is satisfied today (max is 8 per [§4](#4-primitive-types)).

If a future change introduces a vector element with alignment > 8 — most plausibly via [§8.2](#82-stronger-alignment-for-u128--u256--address--signer)'s stronger-alignment proposal for `address` / `u256` — the correct data offset becomes `align_up(8, elem_align)`, which varies per vector. Unlike struct or enum field offsets — which are baked into each access op at specialization time — vector access uses an indexed pattern (`base + data_offset + i * elem_size`), so the data offset must be expressible as a single value the runtime can use across every elem load, store, push, pop, and growth on a given vector.

The current micro-op design has no channel for this: `VecLoadElem`, `VecStoreElem`, `VecPushBack`, `VecPopBack`, `AllocVec`, and `GrowVec` all rely on a hardcoded `VEC_DATA_OFFSET`. Bridging the gap would require adding per-vector or per-instruction data-offset metadata, or globally raising `VEC_DATA_OFFSET`, each with its own cost profile.

This is an open design question. It does not block MonoMove at `MAX_ALIGN = 8`, but it does need to be answered before any change that admits vector elements with alignment > 16.
