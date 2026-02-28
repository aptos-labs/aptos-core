# Design Notes

## Stack Map Liveness and the "Maybe Alive" Problem

### The Problem

At control flow merge points, a frame slot may hold a heap pointer on one
incoming path and a scalar on another. Standard liveness analysis computes the
union ("may be live"), but this creates a correctness issue for GC:

- **Including a dead/scalar slot** in the stack map: GC interprets an arbitrary
  u64 as a heap address, reads/writes through it — memory corruption or crash.
- **Omitting a live pointer slot**: GC doesn't update it after copying — the
  program follows a dangling pointer.

### Example

```
PC 0: ptr = VecNew(...)           // ptr holds a heap pointer
PC 1: if condition: goto PC 4
PC 2: ptr = StoreU64(42)          // ptr overwritten with a scalar
PC 3: goto PC 5
PC 4: nop
PC 5: VecNew(...)                 // GC safe point — is ptr a live pointer?
```

At PC 5, `ptr` is a pointer on one path and `42` on the other. No correct
stack map exists for both paths simultaneously.

### Architecture Context

The execution pipeline is:

1. **Move bytecode verifier** (existing, untrusted input) — verifies type
   safety, borrow rules, etc. on the original Move bytecode.
2. **Runtime re-compiler** (trusted code) — lowers verified Move bytecode into
   the register-based runtime instructions, generating frame layouts, stack
   maps, and object descriptors.
3. **Interpreter** (trusted code) — executes the runtime instructions.

Because the re-compiler is trusted code operating on verified input, the
runtime instructions and their stack maps are produced by the trusted computing
base — not by an external attacker. This means:

- **No second verifier is needed** for the runtime instructions. The original
  Move bytecode verifier guarantees type safety of the input, and the
  re-compiler is responsible for preserving that guarantee in its output.
- The "malicious stack map" attack vector does not apply — users never author
  runtime instructions directly.
- **The re-compiler itself becomes security-critical.** A bug in the
  re-compiler (e.g., wrong stack map, incorrect frame layout) could produce
  the same memory corruption described above. Thorough testing of the
  re-compiler is essential.

### Mitigations

Since the re-compiler has full type information from the verified Move
bytecode, it can solve the "maybe alive" problem by construction:

1. **Separate pointer and scalar regions in the frame**: The re-compiler
   partitions the frame layout so that pointer-typed slots and scalar-typed
   slots occupy disjoint regions. A pointer slot is *always* a pointer
   (initialized to null, set to null when dead). The stack map is simply "scan
   all pointer slots." This eliminates the ambiguity entirely.

2. **Null-on-death**: The re-compiler emits explicit null-writes to pointer
   slots when they go dead. Combined with (1), this ensures GC never
   encounters a stale non-null value in a pointer slot.

3. **Consistent types at merge points**: The re-compiler can guarantee that
   at every control flow merge point, each slot has a consistent type across
   all incoming paths — because the Move bytecode verifier already enforces
   this at the source level, and the re-compiler preserves the invariant
   during lowering.


## Supporting Move References

### The Problem

Move has interior references (`&v[i]`, `&s.field`) that point into the middle
of heap objects. A copying GC relocates objects, invalidating interior
pointers. The GC only knows how to update root pointers (pointers to the start
of an object).

### Fat Pointer Representation

All references use a uniform 16-byte representation: `(base, offset)`.

- `base`: pointer to the root of the containing object (or to a stack local).
- `offset`: byte offset from `base` to the referenced value.
- Dereference: `read(base + offset)`.

This works for all reference targets:

| Target | base | offset |
|---|---|---|
| Stack local at `fp + 24` | `fp + 24` (stack addr) | 0 |
| Vector element `v[i]` | vector root pointer | `VEC_DATA_OFFSET + i * elem_size` |
| Struct field `s.field` | struct root pointer | field byte offset |
| Global resource | depends on materialization | field byte offset |

### GC Interaction

The fat pointer's `base` field is registered in the stack map like any other
pointer slot. During GC:

- If `base` is a heap pointer (`is_heap_ptr` returns true): GC updates `base`
  to the new object location. The `offset` is unchanged because the object's
  internal layout is preserved during copying.
- If `base` is a stack pointer (`is_heap_ptr` returns false): GC skips it.
  Stack memory doesn't move.

This works because the stack and heap are separate `MemoryRegion` allocations
with non-overlapping address ranges.

### Stack Map Impact

No changes to the stack map format. The stack map is a flat list of byte
offsets where pointer-sized values live. For a fat reference at frame offset X:

- `X + 0` (the `base` field) goes in the stack map.
- `X + 8` (the `offset` field) does NOT — it's a plain integer.

The re-compiler's responsibility is to lay out fat references with `base`
first and register only the `base` offset in the stack map. Since the
re-compiler has type information from the verified Move bytecode, it knows
exactly which locals are references and can generate the correct layout.

### Cost

- Space: 16 bytes per reference on the stack (vs 8 for a raw pointer). References
  are short-lived locals, so this doesn't affect heap size.
- Runtime: one `add` per dereference to compute `base + offset`.
- GC complexity: zero — `base` is just another pointer in the stack map.

### Alternatives Considered

| Approach | Tradeoff |
|---|---|
| No interior references (Java, OCaml) | Too restrictive for Move |
| Non-moving GC (Go) | Fragmentation, no compaction |
| GC tracks interior pointers (C# CLR) | Complex GC, reverse object lookups |
| Disallow GC while references are live | Impossible — `vector::empty()` allocates while `&mut v[i]` is live |
| **Fat pointers** | 16 bytes/ref, one add/deref — chosen approach |


## Stack Map Representations

The current prototype uses `HashMap<usize, Vec<u32>>` — a map from PC to a
list of pointer slot offsets. This is simple but has real costs: heap
allocation per entry, hash computation on every GC, and poor cache locality.

### Constraint: Calling Convention and Frame Layout

A fully partitioned frame (all pointers first, then scalars) would let the
stack map collapse to a single `pointer_region_size` per function. However,
the standard calling convention complicates this: the caller writes arguments
at fixed offsets in the callee's frame, and if the function signature mixes
pointer and scalar parameters, the callee's frame begins with a mixed region
that can't be rearranged unilaterally.

Two approaches address this:

### Approach A: Per-Function Pointer Slot Array + Per-PC Bitmap

Each function declares which of its frame slots are pointer-typed, regardless
of argument order:

```
Function {
    code: Vec<Instruction>,
    frame_size: u32,
    pointer_slots: Vec<u32>,         // byte offsets of all pointer-typed slots
    stack_maps: HashMap<usize, u64>, // PC -> bitmap over pointer_slots (optional)
}
```

GC root scanning iterates `pointer_slots`, checking the bitmap (if present)
or scanning all slots unconditionally:

```rust
for (i, &offset) in func.pointer_slots.iter().enumerate() {
    if bitmap & (1 << i) != 0 {
        let ptr = read_ptr(fp, offset as usize);
        // trace and update if heap pointer ...
    }
}
```

This gives a spectrum of precision vs. simplicity:

- **No per-PC data** (simplest): Just `pointer_slots` per function.
  Null-initialize all pointer slots on entry. GC always scans all of them,
  skipping nulls via `is_heap_ptr`. Zero lookup cost.
- **Per-PC bitmap**: A `u64` per GC safe point (covers up to 64 pointer
  slots per function). Only live pointer slots are scanned. Requires a
  per-PC lookup (sorted array + binary search, or direct indexing by PC).

Either way, `pointer_slots` is shared across all PCs in the function —
only the small bitmap varies per safe point. For a function with 5 pointer
slots out of 12 total, the bitmap is 5 bits wide. Much more compact than
a `Vec<u32>` per PC.

### Approach B: Re-compiler Customizes Calling Convention Per Function

Since the re-compiler controls both the caller and callee sides, it can
define a **per-function calling convention** that reorders arguments to
achieve true pointer/scalar separation in the frame:

```
fn foo(a: u64, b: vector<u64>, c: u64)

  Source parameter order:  a (scalar), b (pointer), c (scalar)
  Frame layout:            b (pointer) | a (scalar), c (scalar)
                           ^             ^
                           pointer rgn   scalar rgn
```

The caller places arguments in the callee's frame according to the reordered
layout, not the source-level parameter order. Since both sides are produced
by the same re-compiler, they always agree on the layout.

This achieves a fully partitioned frame. The stack map reduces to a single
number per function: `pointer_region_size: u32`. GC scans
`[fp, fp + pointer_region_size)` and skips everything after. No bitmaps,
no per-PC data, no lookup tables.

**Tradeoffs:**

- Each function effectively has its own ABI. The re-compiler must track the
  mapping between source parameter index and frame offset for each callee.
- Debugging and stack inspection becomes less intuitive since frame layout
  doesn't match source-level parameter order.
- However, this is purely internal — never exposed to users or external
  tools. The complexity is fully contained within the re-compiler.

### Comparison

| | Approach A (pointer array) | Approach B (custom ABI) |
|---|---|---|
| Stack map per function | `Vec<u32>` of pointer offsets | single `u32` (region size) |
| Per-PC data | optional bitmap (`u64`) | none |
| GC scan | iterate `pointer_slots` | linear scan `[fp, fp+size)` |
| Calling convention | standard (source order) | per-function (reordered) |
| Re-compiler complexity | low | moderate (must track reordering) |
| Null-init needed | yes (pointer slots) | yes (pointer region) |

### Recommendation

Approach A is simpler and sufficient as a starting point. The per-function
`pointer_slots` array with null-initialization gives correct GC root scanning
with no per-PC lookup, accommodates any argument layout, and doesn't
constrain the calling convention. Per-PC bitmaps can be layered on later if
scanning dead null slots becomes a measurable cost.

Approach B is a worthwhile optimization to consider if GC root scanning shows
up as a bottleneck. It eliminates all per-PC metadata and reduces scanning to
a tight linear sweep over a contiguous pointer region. The added re-compiler
complexity is modest given it already manages frame layout, and the
non-standard ABI is invisible outside the runtime.
