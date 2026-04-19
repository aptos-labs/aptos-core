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

## Runtime Design

With the semantic model established, we now describe the concrete runtime representation and execution of closures in mono-move.

### Function Resolution

The function stored in a closure may or may not be fully resolved at pack time. There are two forms:

1. **Resolved (local)**: An `ExecutableArenaPtr<Function>` pointing to a fully materialized, monomorphized function in the executable arena. This is the case for intra-module closures where the specializer has already lowered the target function. No type arguments are needed — monomorphization has already substituted all type parameters, so even originally-generic local functions are concrete by the time they have an arena pointer.
2. **Unresolved (cross-module)**: A fully qualified function name (module address + module name + function name) plus type arguments. The target module may not be loaded yet, or the specific monomorphized instantiation may not exist. Resolution happens lazily at `CallClosure` time: the runtime resolves the name, triggers loading/specialization if needed, and caches the result.

At `CallClosure` time, the runtime must have a resolved `Function` pointer. If the closure carries an unresolved reference, it is resolved on first call (and the resolved pointer may be cached for subsequent calls if the closure is copied).

### Heap Representation

A closure is a heap object. Different closures with the same type signature can capture different numbers and sizes of values (determined by the mask and the underlying function), so the closure's total size is not statically known at the call site. Heap allocation gives a uniform 8-byte representation (a pointer) in the frame regardless of what the closure captures.

```
Owner region          Heap (closure)                    Heap (captured data)
┌────────┐     ┌──────────────────────────┐      ┌──────────────────────────┐
│   ●────┼────►│ desc_id(4) | size(4)     │      │ desc_id(4) | size(4)     │
│        │     ├──────────────────────────┤      ├──────────────────────────┤
└────────┘     │ func_ref                 │      │ tag = RAW                │
  8 bytes      │ mask (8)                 │      │ raw bytes ...            │
               │ captured_data ●──────────┼─────►│                          │
               └──────────────────────────┘      └──────────────────────────┘
                                                         OR
                                                 ┌──────────────────────────┐
                                                 │ desc_id(4) | size(4)     │
                                                 ├──────────────────────────┤
                                                 │ tag = MATERIALIZED       │
                                                 │ captured_0               │
                                                 │ captured_1               │
                                                 │ ...                      │
                                                 └──────────────────────────┘
```

Conceptually, a closure contains two enums: `ClosureFuncRef` and `ClosureCapturedData`.

`func_ref` is a `ClosureFuncRef`:

```rust
enum ClosureFuncRef {
    /// Local function, already monomorphized and materialized.
    Resolved(ExecutableArenaPtr<Function>),
    /// Cross-module function, resolved lazily at call time.
    /// Exact payload TBD (module id + function name + type args).
    Unresolved { .. },
}
```

The `Unresolved` variant is contingent on the general design of cross-module (indirect) calls, which is not yet implemented — its payload will mirror whatever representation cross-module call targets use. The exact representation of `ClosureFuncRef` (inline in the closure, a heap object, a pointer to some other table) is not yet decided. Neither variant contains heap pointers — `Resolved` points into the executable arena, and `Unresolved` will store or reference its data outside the GC heap.

`captured_data` is a pointer to a `ClosureCapturedData` heap object:

```rust
enum ClosureCapturedData {
    /// Raw BCS bytes from storage, not yet parsed.
    Raw(/* raw bytes */),
    /// Materialized flat values, as produced by PackClosure or
    /// by parsing a Raw closure on first call.
    Materialized {
        captured_0,
        captured_1,
        ...
    },
}
```

Unlike `ClosureFuncRef`, `ClosureCapturedData` is always a separate heap object — this is necessary for GC correctness. The two variants have different pointer layouts (Raw has no heap pointers; Materialized may have several), so they need different object descriptors. By making `captured_data` a pointer to a separate heap object with its own `desc_id`, each variant is self-describing and the closure's own descriptor stays fixed (it always marks `captured_data` as a heap pointer). The captured data object contains a tag field to distinguish Raw from Materialized. We could also consider letting the `desc_id` itself serve as the tag (since the two variants already need different descriptors), but for now we keep an explicit tag for clarity.

The `Raw` form represents closures loaded from storage whose captured values have not yet been parsed — more on this in the Closure Storage section. The `Materialized` form has captured values laid out flat at their concrete sizes. To materialize, the runtime allocates a new Materialized heap object and updates the pointer in the closure — the old Raw object becomes garbage.

Other fields:

- `**mask**` (8 bytes): The `ClosureMask` bits (u64). Scalar, not traced by GC.

#### GC Descriptors

The closure object has a fixed descriptor: `func_ref` is not a heap pointer, `mask` is a scalar, and `captured_data` is always a heap pointer listed in `pointer_offsets`. This descriptor never changes.

The `ClosureCapturedData` heap object has its own descriptor:
- **Raw**: Trivial — no heap pointers to trace.
- **Materialized**: Struct-like — `pointer_offsets` lists the captured values that are heap pointers.

We may consider reusing `ObjectDescriptor::Struct` for the Materialized case in the future, since the GC just needs a size and a list of pointer offsets.

### Micro-Ops

Two fused super-instructions handle closures. Both contain variable-length data and are boxed to keep the base `MicroOp` enum small. An alternative is to store the variable-length data in a side table and reference it by index from the instruction — this could keep all `MicroOp` variants within a fixed size. Something to revisit if instruction cache behavior or enum size becomes a concern.

The current design specifies separate `SizedSlot` entries for each captured/provided argument, allowing them to be at arbitrary frame offsets. An alternative is to require the compiler to lay out all arguments contiguously, so the instruction only needs a start offset and total size — enabling a single `memcpy` instead of per-argument copies. This could be explored as an optimization.

#### `PackClosure`

```rust
PackClosure(Box<PackClosureOp>)

struct PackClosureOp {
    dst: FrameOffset,
    func_ref: ClosureFuncRef,
    mask: u64,
    /// Descriptor for the ClosureCapturedData (Materialized) heap object.
    captured_descriptor_id: DescriptorId,
    /// Each captured value's frame location and byte size, in mask order.
    captured: Vec<SizedSlot>,
}
```

Semantics:

1. Allocate a `ClosureCapturedData` (Materialized) heap object with `captured_descriptor_id`. **MAY TRIGGER GC.**
2. Copy each captured value from the frame into the captured data object.
3. Allocate the closure object, write `func_ref`, `mask`, and a pointer to the captured data object.
4. Write the closure heap pointer to `dst`.

#### `CallClosure`

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
4. Interleave captured values (from the captured data object) and provided arguments (from the caller's frame) into the callee's argument slots, using the mask.
5. Perform the call (save metadata, set new fp, jump to callee).

The runtime interprets the mask at call time because the caller doesn't statically know which function the closure wraps (the closure may have been passed to the caller as an argument with type `|u64| -> u64`).

*TODO*: There may be additional runtime consistency checks needed at `CallClosure` time (e.g., verifying that the resolved function's signature is compatible with the provided arguments). The exact set of checks is to be determined.

**Reconstructing captured value layout.** The mask tells *which* parameters are captured, but not their sizes or byte offsets within the captured data object. The runtime derives this from the resolved `Function` + `mask`: it looks up parameter sizes from the `Function` struct and combines with the mask to compute both the captured value offsets and the target argument slot positions. Alternatives considered: storing a `(offset, size)` list per captured value in the closure body (self-describing but adds overhead), or storing a pointer to a compile-time-generated layout descriptor (extra pointer per closure).

#### Why `CallClosure` must be a fused instruction

A closure is opaque at the call site — the caller only knows the closure's type signature (e.g., `|u64| -> u64`), not which function it wraps. Different closures with the same type can have different underlying functions, different masks, different numbers and sizes of captured values, and different callee frame sizes. The specializer at the `CallClosure` site cannot generate a static sequence of smaller instructions because it doesn't know:

- How many bytes to read from the captured data object, or at what offsets
- Which callee argument slots the captured values map to
- The callee's total argument region size

This information is only available at runtime via `func_ref` → `Function` + `mask`. Decomposition into smaller instructions is not possible for the general case.

`PackClosure` is different: the compiler always knows the concrete function and captured values at the pack site, so decomposition into `HeapNew` + `HeapMoveTo` + `HeapMoveToImm8` is feasible. We use a fused instruction for now to reduce dispatch overhead, but may revisit this if keeping the instruction set smaller turns out to be preferrable.

### GC Considerations

1. **Safe points**: `PackClosure` allocates and is therefore a GC safe point. The specializer must ensure the safe-point layout at the `PackClosure` instruction accounts for any live heap pointers in the frame (including captured values, since GC occurs before the copy).
2. **Stack scanning during calls**: For materialized closures, `CallClosure` does not allocate — it reads from the closure, copies into the callee's frame, and performs the call. However, for raw closures, `CallClosure` must materialize the captured data first, which allocates and **may trigger GC**. The PC after `CallClosure` (the return point) is always a safe point, just like after any other call instruction — the callee may trigger GC, and upon return the caller's frame must have a valid pointer layout in the `FrameLayoutMap`.
3. **Closures capturing closures**: Works naturally for materialized closures — a captured closure is an 8-byte heap pointer in the captured values region, and its offset appears in the Materialized object's `pointer_offsets`.

### Operations Requiring Structural Access to Captured Values

Several operations need to traverse the captured values' structure at runtime, which is challenging because closures are opaque — the captured types are hidden behind the closure's type signature.

**Equality and comparison.** The old VM supports structural equality and ordering for closures: compare functions by canonical name (module_id + function_name + type_args), then compare captured values lexicographically. This works across resolution states (resolved vs. unresolved). Reimplementing this requires (a) canonical identity in `Function` (currently only has `name`), (b) mixed resolved/unresolved comparison support, and (c) recursive structural comparison of captured values driven by runtime type metadata.

**String formatting.** Debug/display representations of closures need to format captured values, which requires knowing their types.

**`bcs::to_bytes`.** BCS serialization of a closure (e.g., when a struct containing a closure is serialized) requires traversing captured values using their type layouts.

The plan is to reimplement the current semantics, but this is not a priority for the initial implementation — it will be supported incrementally. The same infrastructure (runtime type traversal of captured values) will serve all three use cases above. If the complexity proves too high, making some or all of these operations runtime errors for closures remains an option to be reevaluated in the future.

### Safety: Bytecode Verifier Guarantees

The old VM performs several runtime checks on closure operations that we rely on the bytecode verifier to enforce instead:

- **Pack visibility**: `check_pack_closure_visibility` checks that the caller can reference the target function (same-module or public). The bytecode verifier's visibility checks guarantee this statically.
- **Captured value types**: `verify_pack_closure` checks that captured values match the function's parameter types and are not references. The bytecode verifier's type checker and reference checker enforce this.
- **Closure signature match at call site**: The old VM checks that the number of provided arguments + captured arguments equals the callee's parameter count, and that return counts match. The bytecode verifier's type checker ensures this.
- **Type depth/size limits**: `check_function_type_count_and_depth` guards against excessively deep function types. The bytecode verifier enforces type complexity limits at load time.

By relying on the bytecode verifier for these properties, the mono-move runtime can skip these checks and trust that the closure's `func_ref`, `mask`, and captured values are consistent.

## Closure Storage

*Note: the closure storage design is contingent on the general design of global storage in mono-move, which is not yet finalized. Details here may change.*

### Current VM

Closures with the `store` ability can be persisted in global state. The old VM uses a self-describing serialized format:

```
[format_version, module_id, fun_id, ty_args, mask, layout_0, value_0, layout_1, value_1, ...]
```

The function is identified by canonical name (module_id + fun_id + ty_args), not by a pointer or index. Each captured value is preceded by its `MoveTypeLayout`.

**Why embed layouts?** Without them, deserializing captured values would require loading the target function's module to look up parameter types, applying the mask to select the captured ones, and deriving layouts — which may reference types from yet more modules. Embedding layouts makes the serialized closure fully self-contained: deserialization needs no module loading. The closure remains unresolved (function identified by name only) until it is actually called.

**When are layouts computed?** At `PackClosure` time, but only for storable closures (those wrapping `public` or `#[persistent]` functions). Non-storable closures skip this since they'll never be serialized. `PackClosure` already loads the target function's full definition for cross-module calls, so layout computation piggybacks on that — no additional loading.

**No loading on storage round-trips.** Consider loading a `vector<|u64| -> u64>` from storage, replacing one element, and writing it back. On read, all closures deserialize into unresolved form — no modules loaded. On write-back, the untouched closures still carry their original serialized data, which is written back verbatim. The new closure has its layouts pre-computed from pack time. No module loading occurs at any point.

### Design Goals for mono-move

The mono-move closure storage design should preserve the following properties:

1. **Delay function loading whenever possible.**
  - No module loading on deserialization.
  - No module loading on write-back of untouched closures.
  - (Stretch goal) No module loading on `PackClosure` for cross-module targets. The pack site has access to the target function's parameter signatures via the current module's function handle table, so captured value types and layouts can be determined locally. Function resolution is deferred to `CallClosure`.
2. **No stored layouts.** Captured value layouts should not need to be stored in the closure or in a side table. They are either embedded in the raw serialized bytes (round-trip case) or derivable on demand from loaded type definitions (materialized case).
3. **Storage format compatibility** with the current VM, since existing on-chain data must remain readable after migration.

### Proposed Approach for mono-move

The core idea is **lazy deserialization of captured values**, reflected by the `ClosureCapturedData` enum in the heap representation above. A closure loaded from storage is not fully parsed — its captured values stay as a raw BCS blob until the closure is actually called.

**Deserialization (read from storage):** Read the function identity (module_id + fun_id + ty_args) and mask. Store the remaining bytes as a `Raw` blob. No module loading, no value parsing.

**Materialization (on first call):** Parse the raw blob using the embedded layouts to extract individual values into flat memory. Resolve the function reference. This is the only point where module loading occurs.

**Serialization (write to storage):**

- *Raw closure (untouched):* Write back the function identity + mask + original blob verbatim. No loading, no layout computation.
- *Materialized closure (called or newly packed):* Since the captured values have been materialized, their type definitions must already be loaded. The runtime can walk these definitions to serialize each value directly — no intermediate layout objects or stored layouts needed.

**`PackClosure`:** Stores func_ref + mask + flat captured values. No layout pre-computation — layouts are derived on demand at serialization time if needed.

### Whether to Store Layouts

The current serialized format embeds a `MoveTypeLayout` before each captured value. Our lazy deserialization design does not strictly need these embedded layouts — the raw blob is stored as opaque bytes and only parsed at materialization time, when type definitions are loaded anyway.

The initial plan is to **keep writing layouts**, maintaining full backward compatibility with the V1 format. This avoids having to update peripheral systems (indexers, explorers, SDKs) that rely on the self-describing format to decode closure data. A future V2 format that omits layouts (leaner on-chain footprint, but no longer self-describing) could be introduced later with a separate migration plan.

## Interior Mutability

The design includes lazy resolution (`ClosureFuncRef`: unresolved → resolved) and lazy deserialization (`ClosureCapturedData`: raw → materialized). Closures require interior mutability to cache these transitions — without it, repeated calls on copies of the same closure (a common pattern: copy then call) would re-resolve and re-materialize each time, and operations like equality, formatting, and serialization that need materialized values would also pay this cost repeatedly.

**Where the cached state lives.** For `ClosureCapturedData`, the current design already supports in-place updates: the closure object holds a pointer to the captured data heap object, so materializing just means allocating a new Materialized object and updating the pointer — the closure's own descriptor doesn't change. For `ClosureFuncRef`, caching the resolved pointer is straightforward if it's stored inline (just overwrite it). Alternatively, resolved functions could be cached in a side table outside the closure, leaving the heap object untouched. A side table works naturally for function pointers but is harder for captured values — materialized values live on the GC heap and need to be traced, so splitting them from the closure introduces complexity in ownership and lifetime management.
