# Closure Design for mono-move Runtime

## Background

Move supports closures (function values) via three bytecode instructions:
- `PackClosure(func, mask)` / `PackClosureGeneric(func, mask)` — create a closure by partially applying arguments
- `CallClosure(sig)` — invoke a closure value

A `ClosureMask` is a `u64` bitmask indicating which of the function's parameters are captured at pack time vs. provided at call time. For example, for `f(a, _, b, _)` with mask `0b0101`, positions 0 and 2 are captured. At call time, the caller provides the remaining (uncaptured) arguments, and the runtime reconstructs the full argument list by interleaving captured and provided values.

The mono-move stackless IR already translates these into slot-based form:
```
PackClosure(dst_slot, func_handle, mask, [captured_slots...])
PackClosureGeneric(dst_slot, func_inst, mask, [captured_slots...])
CallClosure([result_slots...], sig, [closure_slot, arg_slots...])
```

The missing piece is the runtime: value representation, micro-ops, GC integration, and the calling convention for closures.

## Semantic Model

These properties shape every design decision below, so we state them upfront.

### Capture Is a Move

Capturing is a **move**. In the old VM, `PackClosure` pops values off the operand stack — they are consumed. In mono-move, the captured values are moved (memcpy'd) from the source frame slots into the closure's heap body. After packing, the source slots are logically dead (the bytecode verifier / borrow checker guarantees they won't be read again). No implicit copy or clone occurs.

This is consistent with Move's ownership model: if a value has only the `copy` ability, the programmer must explicitly copy it before capturing; otherwise the original is moved into the closure.

### No Internal Mutability — Closures Are One-Shot

Calling a closure **consumes** it. In the old VM, `CallClosure` pops the closure off the operand stack, destructures it via `unpack()`, and interleaves the captured values with the provided arguments via `mask.compose(captured, args)`. The captured values become ordinary by-value parameters to the callee. The closure ceases to exist after the call.

Consequences:
- **No mutable state**: There is no mechanism to mutate captured values across calls — each call destroys the closure.
- **Multi-call requires copy**: To call a closure more than once, the caller must `copy` it first. This is only possible if the closure has the `copy` ability (requires all captured values to also have `copy`).
- **No write barriers**: The runtime never writes back into a closure's captured values.

### No Reference Capture

The compiler rejects closures that capture references (mutable or immutable). Captured values are always owned, by-value data. This eliminates aliasing concerns and simplifies GC — the closure's heap object contains only owned values, never pointers into stack frames or other borrowed state.

### Same Function, Same Code — Caller Does the Work

A function that is captured into a closure is the same function that can be called directly. There is no "closure-specific" variant — the same compiled micro-op sequence executes for both direct calls and closure calls. This means:

1. **The caller reconstructs the full argument list.** Before calling the function (whether directly or via closure), all parameters must be placed in the callee's argument slots in the standard layout. For a direct call, the caller writes all arguments. For a closure call, the caller interleaves captured values (read from the closure heap object) with provided arguments using the mask, so that the callee's frame looks identical in both cases.

2. **The callee is oblivious.** It executes the same code regardless of how it was invoked. It doesn't know whether its arguments came from a direct call or were partially supplied by a closure.

3. **No special calling convention.** This avoids duplicating function bodies or introducing closure-specific entry points.

## Heap Representation

A closure is a heap object. In the caller's frame, it occupies 8 bytes (a heap pointer).

```
Owner region                       Heap
┌────────┐                  ┌──────────────────────────┐
│   ●────┼─────────────────►│ desc_id(4) | size(4)     │  header
│        │                  ├──────────────────────────┤
└────────┘                  │ func_ptr (8)             │  pointer to Function
  8 bytes                   │ mask (8)                 │  ClosureMask bits
                            ├──────────────────────────┤
                            │ captured_0               │
                            │ captured_1               │
                            │ ...                      │
                            └──────────────────────────┘
```

Fields:
- **`func_ptr`** (8 bytes): Raw pointer to the target `Function` in the executable arena. NOT a heap pointer — the GC must not trace it.
- **`mask`** (8 bytes): The `ClosureMask` bits (u64). Scalar, not traced by GC.
- **`captured_0..N`**: The captured values, laid out contiguously at their concrete sizes. Some of these may be heap pointers (vectors, structs, enums, other closures).

### Reconstructing Captured Value Layout at Call Time

At a `CallClosure` site, the caller must read captured values out of the closure and place them into the callee's argument slots. The mask tells *which* parameters are captured, but not their sizes or byte offsets within the closure body.

**Chosen approach**: Derive layout from `func_ptr` + `mask`. The caller reads `func_ptr` from the closure, looks up parameter sizes from the `Function` struct, and combines with the mask to compute both the captured value offsets within the closure body and the target argument slot positions. No extra per-closure metadata needed.

**Alternatives considered**:
- Store a `(offset, size)` list per captured value in the closure body — self-describing but adds variable-size metadata overhead to every closure.
- Store a pointer to a compile-time-generated layout descriptor — one extra 8-byte pointer per closure, avoids the variable-length list but requires a separate descriptor table.

### GC Descriptor

A new `ObjectDescriptor::Closure` variant describes the pointer layout of the captured values region. `func_ptr` is NOT a heap pointer (it lives in the executable arena) and must NOT appear in `pointer_offsets`.

Conceptually this is identical to `ObjectDescriptor::Struct` — the GC just needs a size and a list of byte offsets that hold heap pointers. We may consider reusing `Struct` in the future to avoid the extra variant, but a dedicated `Closure` variant keeps the distinction explicit for now.

## Micro-Ops

Two fused super-instructions handle closures. Both contain variable-length data and are boxed to keep the base `MicroOp` enum small.

### `PackClosure`

```rust
PackClosure(Box<PackClosureOp>)

struct PackClosureOp {
    dst: FrameOffset,
    func_ptr: ExecutableArenaPtr<Function>,
    mask: u64,
    descriptor_id: DescriptorId,
    /// (src_offset, size) for each captured value, in mask order.
    captured: Vec<(FrameOffset, u32)>,
}
```

Semantics:
1. Allocate a heap object with the given `descriptor_id`. **MAY TRIGGER GC.**
2. Write `func_ptr` and `mask` into the closure header.
3. Copy each captured value from the frame into the closure body.
4. Write the heap pointer to `dst`.

### `CallClosure`

```rust
CallClosure(Box<CallClosureOp>)

struct CallClosureOp {
    closure_src: FrameOffset,
    /// (src_offset, size) for each provided (non-captured) argument, in call order.
    provided_args: Vec<(FrameOffset, u32)>,
}
```

Semantics:
1. Read `func_ptr` and `mask` from the closure heap object.
2. Look up parameter sizes from the `Function` struct to determine captured value layout.
3. Interleave captured values (from the closure body) and provided arguments (from the caller's frame) into the callee's argument slots, using the mask.
4. Perform the call (save metadata, set new fp, jump to callee).

The runtime interprets the mask at call time because the caller doesn't statically know which function the closure wraps (the closure may have been passed opaquely as a `|u64| -> u64`).

### Why `CallClosure` must be a fused instruction

A closure is opaque at the call site — the caller only knows the closure's type signature (e.g., `|u64| -> u64`), not which function it wraps. Different closures with the same type can have different underlying functions, different masks, different numbers and sizes of captured values, and different callee frame sizes. The compiler at the `CallClosure` site cannot generate a static sequence of smaller instructions because it doesn't know:

- How many bytes to read from the closure body, or at what offsets
- Which callee argument slots the captured values map to
- The callee's total argument region size

This information is only available at runtime via `func_ptr` → `Function` + `mask`. Decomposition into smaller instructions is not possible for the general case.

`PackClosure` is different: the compiler always knows the concrete function and captured values at the pack site, so decomposition into `HeapNew` + `HeapMoveTo` + `HeapMoveToImm8` is feasible. We use a fused instruction for now to reduce dispatch overhead, but may revisit this if keeping the instruction set smaller turns out to be preferrable.

## GC Considerations

1. **Closure heap objects** are traced via `ObjectDescriptor::Closure`. `func_ptr` is NOT a heap pointer and is not in `pointer_offsets`.

2. **Closures capturing closures**: Works naturally — a captured closure is an 8-byte heap pointer in the captured values region, and its offset appears in `pointer_offsets`.

3. **Safe points**: `PackClosure` allocates and is therefore a GC safe point. The compiler must ensure the safe-point layout at the `PackClosure` instruction accounts for any live heap pointers in the frame (including captured values that haven't been moved into the closure yet, since GC occurs before the copy).

4. **Stack scanning during calls**: `CallClosure` itself does not allocate — it reads from the closure, copies into the callee's frame, and performs the call. No GC can occur during the interleaving phase. However, the PC after `CallClosure` (the return point) is a safe point, just like after any other call instruction — the callee may trigger GC, and upon return the caller's frame must have a valid pointer layout in the `FrameLayoutMap`.
