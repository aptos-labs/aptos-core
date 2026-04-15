# Closure Design for mono-move Runtime

## Background

Move supports closures (function values) via three bytecode instructions:
- `PackClosure(func, mask)` / `PackClosureGeneric(func, mask)` — create a closure by partially applying arguments
- `CallClosure(sig)` — invoke a closure value

A `ClosureMask` is a `u64` bitmask indicating which of the function's parameters are captured at pack time vs. provided at call time. For example, for `f(a, _, b, _)` with mask `0b0101`, positions 0 and 2 are captured. At call time, the caller provides the remaining (uncaptured) arguments, and the runtime reconstructs the full argument list by interleaving captured and provided values.

The destacking pass translates these stack-based instructions into explicit named slots. The missing piece is the runtime: value representation, micro-ops, GC integration, and the calling convention for closures.

## Semantic Model

These properties shape every design decision below, so we state them upfront.

### Capture Is a Move

Capturing is a **move**. In the old VM, `PackClosure` pops values off the operand stack — they are consumed. In mono-move, the captured values are moved (memcpy'd) from the source frame slots into the closure's heap body. After packing, the source slots are logically dead (the bytecode verifier guarantees they won't be read again). No implicit copy or clone occurs.

This is consistent with Move's ownership model: if a value has only the `copy` ability, the programmer must explicitly copy it before capturing; otherwise the original is moved into the closure.

### Closures Are Consumed When Called

Calling a closure **consumes** it. In the old VM, `CallClosure` pops the closure off the operand stack, destructures it via `unpack()`, and interleaves the captured values with the provided arguments via `mask.compose(captured, args)`. The captured values become ordinary by-value parameters to the callee. The closure ceases to exist after the call.

Consequences:
- There is no mechanism to mutate captured values across calls — each call destroys the closure.
- To call a closure more than once, the caller must `copy` it first. This is only possible if the closure has the `copy` ability (requires all captured values to also have `copy`).
- The runtime never writes back into a closure's captured values.

### No Reference Capture

The bytecode verifier rejects closures that capture references (mutable or immutable). Captured values are always owned, by-value data. This eliminates aliasing concerns and simplifies GC — the closure's heap object contains only owned values, never pointers into stack frames or other borrowed state.

This restriction could be relaxed in the future. From the runtime's perspective, a captured reference would be a 16-byte fat pointer `(base_ptr, byte_offset)` stored in the closure's heap body. The GC already handles fat pointers in stack frames — it checks whether `base_ptr` falls in the heap range and only traces/relocates heap pointers, ignoring stack pointers. The same logic would apply to fat pointers inside a closure's captured values, so no new GC mechanism would be needed. The closure's `pointer_offsets` would simply include the `base_ptr` half of any captured reference, just as `FrameLayoutMap` does for references in stack frames today. The main challenge would be on the language/verifier side (ensuring borrow safety when references escape into closures), not the runtime.

### No Special Calling Convention for Supporting Closures

A function that is captured into a closure is the same function that can be called directly. There is no "closure-specific" variant — the same compiled micro-op sequence executes for both direct calls and closure calls. This means:

1. **The caller reconstructs the full argument list.** Before calling the function (whether directly or via closure), all arguments must be placed in the callee's argument slots in the standard layout. For a direct call, the caller writes all arguments. For a closure call, the caller interleaves captured values (read from the closure heap object) with provided arguments using the mask, so that the callee's frame looks identical in both cases.

2. **The callee is oblivious.** It executes the same code regardless of how it was invoked. It doesn't know whether its arguments came from a direct call or were partially supplied by a closure.

3. **No special calling convention.** This avoids duplicating function bodies or introducing closure-specific entry points.

## Function Resolution

The function stored in a closure may or may not be fully resolved at pack time. There are two forms:

1. **Resolved (local)**: An `ExecutableArenaPtr<Function>` pointing to a fully materialized, monomorphized function in the executable arena. This is the case for intra-module closures where the specializer has already lowered the target function. No type arguments are needed — monomorphization has already substituted all type parameters, so even originally-generic local functions are concrete by the time they have an arena pointer.

2. **Unresolved (cross-module)**: A fully qualified function name (module address + module name + function name) plus type arguments. The target module may not be loaded yet, or the specific monomorphized instantiation may not exist. Resolution happens lazily at `CallClosure` time: the runtime resolves the name, triggers loading/specialization if needed, and caches the result.

At `CallClosure` time, the runtime must have a resolved `Function` pointer. If the closure carries an unresolved reference, it is resolved on first call (and the resolved pointer may be cached for subsequent calls if the closure is copied).

## Heap Representation

A closure is a heap object. Different closures with the same type signature can capture different numbers and sizes of values (determined by the mask and the underlying function), so the closure's total size is not statically known at the call site. Heap allocation gives a uniform 8-byte representation (a pointer) in the frame regardless of what the closure captures.

```
Owner region                       Heap
┌────────┐                  ┌──────────────────────────┐
│   ●────┼─────────────────►│ desc_id(4) | size(4)     │  header
│        │                  ├──────────────────────────┤
└────────┘                  │ func_ref (N)             │  inline FunctionRef enum
  8 bytes                   │ mask (8)                 │  ClosureMask bits
                            ├──────────────────────────┤
                            │ captured_0               │
                            │ captured_1               │
                            │ ...                      │
                            └──────────────────────────┘
```

The `func_ref` field is a `ClosureFuncRef` enum laid out inline in the closure body — the same type used in the `PackClosureOp` micro-op:

```rust
enum ClosureFuncRef {
    /// Local function, already monomorphized and materialized.
    Resolved(ExecutableArenaPtr<Function>),
    /// Cross-module function, resolved lazily at call time.
    /// Exact payload TBD (module id + function name + type args).
    Unresolved { .. },
}
```

The concrete in-memory representation of this enum (tag size, padding, payload layout) is an implementation detail to be finalized. The `Unresolved` variant is contingent on the general design of cross-module (indirect) calls, which is not yet implemented — its payload will mirror whatever representation cross-module call targets use.

Neither variant contains heap pointers — `Resolved` points into the executable arena, and `Unresolved` will store or reference its data outside the GC heap. This means the GC does not need to trace `func_ref` and the same closure descriptor works for both variants. In the future, we could consider moving some of the function reference information into the object descriptor itself if that simplifies the layout.

Other fields:
- **`mask`** (8 bytes): The `ClosureMask` bits (u64). Scalar, not traced by GC.
- **`captured_0..N`**: The captured values, laid out contiguously at their concrete sizes. Some of these may be heap pointers (vectors, structs, enums, other closures).

### Reconstructing Captured Value Layout at Call Time

At a `CallClosure` site, the caller must read captured values out of the closure and place them into the callee's argument slots. The mask tells *which* parameters are captured, but not their sizes or byte offsets within the closure body.

**Chosen approach**: Derive layout from the resolved `Function` + `mask`. The runtime looks up parameter sizes from the `Function` struct and combines with the mask to compute both the captured value offsets within the closure body and the target argument slot positions. No extra per-closure metadata needed.

### Safety: Bytecode Verifier Guarantees

The old VM performs several runtime checks on closure operations that we rely on the bytecode verifier to enforce instead:

- **Pack visibility**: `check_pack_closure_visibility` checks that the caller can reference the target function (same-module or public). The bytecode verifier's visibility checks guarantee this statically.
- **Captured value types**: `verify_pack_closure` checks that captured values match the function's parameter types and are not references. The bytecode verifier's type checker and reference checker enforce this.
- **Closure signature match at call site**: The old VM checks that the number of provided arguments + captured arguments equals the callee's parameter count, and that return counts match. The bytecode verifier's type checker ensures this.
- **Type depth/size limits**: `check_function_type_count_and_depth` guards against excessively deep function types. The bytecode verifier enforces type complexity limits at load time.

By relying on the bytecode verifier for these properties, the mono-move runtime can skip these checks and trust that the closure's `func_ptr`, `mask`, and captured values are consistent.

**Alternatives considered**:
- Store a `(offset, size)` list per captured value in the closure body — self-describing but adds variable-size metadata overhead to every closure.
- Store a pointer to a compile-time-generated layout descriptor — one extra 8-byte pointer per closure, avoids the variable-length list but requires a separate descriptor table.

### GC Descriptor

A new `ObjectDescriptor::Closure` variant describes the pointer layout of the captured values region. `func_ptr` is NOT a heap pointer (it lives in the executable arena) and must NOT appear in `pointer_offsets`.

Conceptually this is identical to `ObjectDescriptor::Struct` — the GC just needs a size and a list of byte offsets that hold heap pointers. We may consider reusing `Struct` in the future to avoid the extra variant, but a dedicated `Closure` variant keeps the distinction explicit for now.

## Micro-Ops

Two fused super-instructions handle closures. Both contain variable-length data and are boxed to keep the base `MicroOp` enum small. An alternative is to store the variable-length data in a side table and reference it by index from the instruction — this could keep all `MicroOp` variants within a fixed size. Something to revisit if instruction cache behavior or enum size becomes a concern.

### `PackClosure`

```rust
PackClosure(Box<PackClosureOp>)

struct PackClosureOp {
    dst: FrameOffset,
    func_ref: ClosureFuncRef,
    mask: u64,
    descriptor_id: DescriptorId,
    /// Each captured value's frame location and byte size, in mask order.
    captured: Vec<SizedSlot>,
}
```

Semantics:
1. Allocate a heap object with the given `descriptor_id`. **MAY TRIGGER GC.**
2. Write `func_ref` and `mask` into the closure header.
3. Copy each captured value from the frame into the closure body.
4. Write the heap pointer to `dst`.

### `CallClosure`

```rust
CallClosure(Box<CallClosureOp>)

struct CallClosureOp {
    closure_src: FrameOffset,
    /// Each provided (non-captured) argument's frame location and byte size, in call order.
    provided_args: Vec<SizedSlot>,
}
```

Semantics:
1. Read `func_ref` and `mask` from the closure heap object.
2. If `func_ref` is unresolved, resolve it (load module, specialize, cache the result).
3. Look up parameter sizes from the resolved `Function` to determine captured value layout.
4. Interleave captured values (from the closure body) and provided arguments (from the caller's frame) into the callee's argument slots, using the mask.
5. Perform the call (save metadata, set new fp, jump to callee).

The runtime interprets the mask at call time because the caller doesn't statically know which function the closure wraps (the closure may have been passed to the caller as an argument with type `|u64| -> u64`).

### Why `CallClosure` must be a fused instruction

A closure is opaque at the call site — the caller only knows the closure's type signature (e.g., `|u64| -> u64`), not which function it wraps. Different closures with the same type can have different underlying functions, different masks, different numbers and sizes of captured values, and different callee frame sizes. The specializer at the `CallClosure` site cannot generate a static sequence of smaller instructions because it doesn't know:

- How many bytes to read from the closure body, or at what offsets
- Which callee argument slots the captured values map to
- The callee's total argument region size

This information is only available at runtime via `func_ptr` → `Function` + `mask`. Decomposition into smaller instructions is not possible for the general case.

`PackClosure` is different: the compiler always knows the concrete function and captured values at the pack site, so decomposition into `HeapNew` + `HeapMoveTo` + `HeapMoveToImm8` is feasible. We use a fused instruction for now to reduce dispatch overhead, but may revisit this if keeping the instruction set smaller turns out to be preferrable.

## GC Considerations

1. **Closure heap objects** are traced via `ObjectDescriptor::Closure`. The `func_ref` field contains no heap pointers in either variant, so the GC only needs to trace the captured values region. The same descriptor works for both resolved and unresolved closures.

2. **Closures capturing closures**: Works naturally — a captured closure is an 8-byte heap pointer in the captured values region, and its offset appears in `pointer_offsets`.

3. **Safe points**: `PackClosure` allocates and is therefore a GC safe point. The specializer must ensure the safe-point layout at the `PackClosure` instruction accounts for any live heap pointers in the frame (including captured values, since GC occurs before the copy).

4. **Stack scanning during calls**: `CallClosure` itself does not allocate — it reads from the closure, copies into the callee's frame, and performs the call. No GC can occur during the interleaving phase. However, the PC after `CallClosure` (the return point) is a safe point, just like after any other call instruction — the callee may trigger GC, and upon return the caller's frame must have a valid pointer layout in the `FrameLayoutMap`.
