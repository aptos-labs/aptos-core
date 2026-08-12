# Native Function Interface — Draft

> **Status:** Informal first draft. Companion: [`native_functions_existing.md`](native_functions_existing.md) — survey of the current Aptos VM native interface.
>
> **Note:** Rust signatures throughout this doc are illustrative — exact shapes may be adjusted during implementation.

## 0. Goals and Milestone

**Goals.** Native functions are first-class citizens in MonoMove — direct access to VM internals (stack and heap memory, the loader, the gas meter, etc.), behaving conceptually like VM instructions rather than opaque external calls, subject to the same safety and metering guarantees as the rest of the VM.

Immediate milestone: **prototype that can run a Decibel transaction end-to-end** (see [`native_functions_decibel_minimum.md`](native_functions_decibel_minimum.md) for the ~40-native MVP set). Long-term goals (typed argument access, full memory safety) are flagged and punted.

---

## 1. Registration & Resolution

- **Specializer is totally ignorant of native bodies.** It reads only the function handles declared in `CompiledModule`. For a `native`-marked handle it emits a generic call site — no callee monomorphization, type args passed at runtime. It never inspects the Rust impl.
- **Linking is lazy** — natives bind to call sites at first invocation, not at module load. Matches existing VM behavior; modules that declare unimplemented natives still load.
- Natives register with the runtime as `(addr, module, name) → NativeImpl`. `NativeImpl` is the Rust fn pointer; the ABI (arg/return slot layout) is derived from the Move declaration at the call site, not declared on the Rust side.
- At dispatch the `Function` struct is either a Move body or a native body. Call/return path is uniform per [`stack_and_calling_convention.md`](stack_and_calling_convention.md).
- **Selective monomorphization** (specializing a generic native per type-arg) is **out of scope for now**. It's the one place the specializer would need richer per-native knowledge.

---

## 2. Native Function Signature & Context

`NativeContext` is a **trait**, not a concrete type. A native is written generic over it:

```rust
trait NativeContext { /* methods listed below */ }

fn my_native<C: NativeContext>(ctx: &mut C) -> Result<NativeStatus, VMInternalError> { ... }
```

Each VM instance commits to one concrete `C` (`ProductionContext`, `TestContext`, etc.). The function table holds `type NativeFunction<C> = Box<dyn Fn(&mut C) -> Result<NativeStatus, VMInternalError>>` with `C` fixed, so every context call statically dispatches.

Because the native is a `Fn` closure, it can carry its own captured state — useful for small per-native config or constants. For anything that needs to persist across calls or be shared between natives, use the extension system (§6) instead.

**Why a trait, and why generic.** The context exposes sub-components that may themselves be generic — the gas meter being the canonical case. Without a generic context, we can't pass these in at their concrete type, which is a major consistency and correctness pain in the current VM (the `SafeNativeContext` workaround at `aptos-native-interface/src/context.rs`, the `&mut dyn NativeGasMeter` indirection, the parallel `legacy_gas_used` counter). A trait parameterized over `C` keeps every sub-component statically typed: `ctx.charge_gas(...)` is a static call, the SafeNative wrapper collapses into the production `C`.

Capabilities, grouped by section:

- **Arguments** (§3)
- **Returns** (§3)
- **Heap allocation** (§4)
- **Type args + type reflection** (§5)
- **Native extensions** (§6)
- **Globals** (§7)
- **Events** (§8)
- **Gas** — see [`gas_design.md`](gas_design.md)
- **Feature flags / VM config** — `ctx.feature_flags()`, `ctx.vm_config()`, etc.

---

## 3. Data Manipulation (Args & Returns)

- **Natives never see `fp` directly.** The context exposes typed accessors over the calling frame.
- **Restricted frame interaction.** Natives interact with the calling frame only through args (read) and returns (write). No locals on the VM stack — any Rust-side scratch / locals the native needs live on the Rust stack instead. To be revisited during implementation.

Context APIs (tentative):

```rust
ctx.num_args() -> usize
unsafe fn arg<T: VMValue>(&self, i: usize) -> Result<T, VMInternalError>

ctx.num_returns() -> usize
unsafe fn set_return<T: VMValue>(&mut self, i: usize, val: T) -> Result<(), VMInternalError>
```

`VMValue` is implemented for primitives (`u8`, `u64`, `bool`, `AccountAddress`, ...) and for VM type wrappers around common heap data structures (struct pointer, vector, reference). Adding support for a new ABI-passable type is one `impl VMValue for ...` away.

The `i` is a positional index, not a raw frame offset — the context resolves it against the call site's ABI on each call. Type matching between `T` and the slot's Move type is *not* checked (hence `unsafe`). 

If the lookup ever becomes a hot-path concern, an `arg_at_offset` escape hatch is possible.

After the first successful `set_return`, further `arg` calls error out to prevent data transmutation and (eventually) GC tracing violations.

- **Module-load cross-check (TBD).** At callee-module load time, reconcile the natives registry with the module's `native fun` declarations: reject the load if a registered impl shadows a non-native definition, and install an "unresolved native" sentinel for a `native fun` with no registered impl so the error surfaces only on call (matching V1 VM).

**Open questions:**

- **Aliasing across repeated `arg` calls.** `arg::<VectorPtr>(0)` called twice returns two wrappers pointing at the same heap chunk — aliased, not copied. Resolution TBD.

---

## 4. Memory Allocation

- **Natives never manage their own (shadow) memory.** Everything goes through the heap.
- Allocations count against the per-transaction memory limit and are GC-visible.

Context APIs (tentative):

```rust
ctx.alloc_struct(descriptor_id: DescriptorId) -> StructPtr
ctx.alloc_vec(elem_descriptor_id: DescriptorId, capacity: usize) -> VectorPtr
ctx.alloc_bytes(len: usize) -> BytesPtr                            // raw byte buffer
ctx.alloc(size: usize, descriptor_id: DescriptorId) -> *mut u8     // lower-level escape hatch
```

`StructPtr` / `VectorPtr` / `BytesPtr` are lightweight newtype wrappers around `*mut u8`. They expose structured accessors (e.g. `StructPtr::write_field::<T>(idx, val)`, `VectorPtr::push::<T>(val)`) in place of raw pointer arithmetic, with bounds and shape checks where applicable. An `as_raw() -> *mut u8` escape hatch is available.

These don't make memory access fully safe — natives still work with raw memory underneath — but they replace ad-hoc pointer arithmetic with methods that know what they're accessing.

**GC safety.** Allocations during a native call can trigger GC, which relocates heap objects. Any wrapper (or raw pointer) the native is actively holding needs to be pinned through `RootPool` so it survives — and gets updated by — collection. How that threads through the wrapper types may shift the final API: a wrapper might carry a root handle whose lifetime constrains how it composes with `&mut ctx` borrows. Details TBD.

- **Descriptor registration.** Custom heap shapes introduced by natives register their `ObjectDescriptor`s with the program-wide `ObjectDescriptorTable` at VM startup; the returned `DescriptorId`s are what `ctx.alloc_*` takes. Extensions (§6) are the natural owner — they hold the lifetime and naming of the heap shapes their natives operate on — but standalone natives can register too.
- **No shadow arenas.** Today's `AlgebraContext.objs` / `NativeRistrettoPointContext.points` aren't carried forward — uncounted growth, beyond the gas budget. The crypto natives must be **rewritten**: the handle-into-arena indirection (`RistrettoPoint { handle: u64 }` + Rust-side `Vec`) becomes a real heap struct holding the bytes directly, with a proper `ObjectDescriptor`. Off the Decibel path; separate milestone.
- Transient Rust-side scratch inside a single call is fine. Rule: anything that becomes a Move value, or grows with input size, lives on the VM heap.

---

## 5. Type Reflection

`InternedType` (defined in `core/src/types.rs`) is the VM's runtime type handle. It's **directly traversable**: natives can walk a type in place — kind, fields, layout, element type, etc. — without ever materializing a separate `TypeTag` or `MoveTypeLayout`.

When a tag or layout actually IS needed (e.g. embedded in an event header, or serialized for external consumption), they can be derived on demand. The goal is to make materialization the exception, not every reflection-using native's default.

Context APIs (tentative):

```rust
ctx.num_ty_args() -> usize
ctx.ty_arg(i) -> InternedType

// Materialized escape hatches
ctx.ty_to_tag(ty) -> TypeTag
ctx.ty_to_layout(ty) -> MoveTypeLayout
```

Type-structural lookups (kind, fields, element type, size/align, etc.) ideally live as methods on `InternedType` itself (`ty.kind()`, `ty.fields()`, ...), so natives can inspect types without routing through the context. If that doesn't pan out — borrow-checker or other constraints — the fallback is `ctx.ty_kind(ty)`-style methods on the context.

**Dispatching on a known struct.** The runtime type representation carries enough information to support this — which is what `crypto_algebra::*` relies on today, in the absence of native specialization (§1).

---

## 6. Native Extensions

Some natives need state that persists across many calls within a single transaction — counters, accumulators, caches, custom data structures. Native extensions are the mechanism for this.

**Shape.** An extension is a Rust-side struct registered with the VM at startup, instantiated fresh per session. It can carry both:

- **Heap-allocated data structures** owned via root pointers — GC-traced through the standard descriptor mechanism; each root pointer is an additional GC root in the interpreter context. Allocated via APIs like `ctx.alloc_*` (§4).
- **Rust-side fields** for small mutable state (counters, flags, host resolver references).

Rule of thumb: anything that grows with input size or contains Move values lives on the heap; the rest stays Rust-side.

> **MVP note.** Any extension's heap-allocated data structures can be implemented Rust-side first and migrated to the heap later. Same accounting caveat as §4 (Rust-side state is outside the per-transaction memory bound) applies in the interim — acceptable temporarily, long-term direction is heap-managed.

**Trait (tentative):**

```rust
trait NativeExtension: Any {
    fn finalize(&mut self, ctx: &mut NativeContext);
    fn gc_roots(&mut self) -> Box<dyn Iterator<Item = &mut *mut u8> + '_>;
}

// Constructor lives outside the trait — `Self`-returning methods aren't
// dyn-safe, so each extension type provides its own free constructor.
impl MyExtension {
    fn init(heap: &mut Heap) -> Self { ... }
}
```

- `init` (per extension, not on the trait) **constructs** a fresh extension at session start. Only the heap is available at this point — there's no calling frame, so the full native context doesn't exist yet. The extension allocates its heap data structures via `heap.alloc_*` and stores the resulting pointers in fields of `Self`. Returning `Self` (rather than mutating `&mut self`) avoids a half-initialized "before init" state with placeholder pointer fields.
- `finalize` runs at session end — wrap up (BCS-serialize accumulated state, extract a change set for the host, etc.).
- `gc_roots` yields a mutable reference to each heap root pointer the extension owns. The GC iterates during collection and writes the (possibly relocated) pointer back through each reference — alongside the call-stack walk and `RootPool`.

**Access from natives:**

```rust
ctx.get_extension::<MyExtension>() -> &mut MyExtension
```

Returns `&mut` directly — no `RefCell` workaround needed since per-transaction execution is single-threaded.

**Registration.** Extensions are registered with the VM at startup as `(TypeId, factory)` pairs, where the factory wraps the extension's `init` and boxes the result:

```rust
type ExtensionFactory = fn(&mut Heap) -> Box<dyn NativeExtension>;
```

At session start the VM invokes each factory and stows the resulting `Box<dyn NativeExtension>` in a `HashMap<TypeId, Box<dyn NativeExtension>>`. `get_extension::<T>` does the `TypeId` lookup + downcast.

> **Alternative (Option C, not chosen): typed registry.** Skip the trait-object map entirely and have the VM hold a struct with one named field per extension type (`Extensions { event_store: EventStore, transaction_context: TransactionContext, … }`). `get_extension::<T>()` becomes a type-dispatched field accessor. Lets the trait keep `init -> Self` and `impl Iterator` (RPITIT), and removes dyn dispatch. Trade-off: third parties can't add extensions without modifying the registry struct. Worth revisiting if MonoMove never grows out-of-tree extensions.

**Full list of extensions in MonoMove** (after the rework — current VM has 10, listed in `aptos-vm/src/move_vm_ext/session/mod.rs::make_aptos_extensions`):

- **EventStore** ([`event_store.md`](event_store.md)) — heap-allocated container with checkpoint/rollback.
- **TransactionContext** — Rust-side only (AUID counter, session counter, optional `UserTransactionContext`).
- **RandomnessContext** — Rust-side only (8-byte counter + flag).
- **CodeContext** — small Rust-side wrapper holding an optional pointer to a heap-allocated publish request (which can be large — module bundles + metadata).
- **StateStorageContext** — Rust-side only (borrowed host resolver).
- **ObjectContext** — memo cache for `create_user_derived_object_address_impl`. Pure compute optimization; could be dropped, or if kept should live on the heap (same accounting argument as the crypto arenas, §4).
- **AlgebraContext / RistrettoContext** — these exist today to host shadow-memory arenas for crypto objects. After the crypto-native rewrite (§4) moves those onto the regular heap, the arenas are no longer needed and the extensions can go away.
- **TableContext, AggregatorContext** — core functionality (table storage, aggregator / delayed-field bookkeeping) moves into the runtime's global value system (§7). What remains, if anything, is a thin shim translating native calls into runtime storage operations; the extension may go away entirely.

**Rollback.** MonoMove is migrating away from the current VM's multi-session model (separate prologue / user / epilogue sessions stitched via `RespawnedSession`) toward a **single session** spanning the whole transaction, with rollback for partial failures. We go with the **single-set + per-extension rollback** model: extensions are created once per transaction, live across all phases, and each implements checkpoint/rollback only if it has state worth rolling back. EventStore already does; counters / memo caches / monotonically-increasing things don't. (The alternative — spawning a fresh extension set per sub-phase, like today's Aptos VM — cuts against the single-session direction and is rejected.)

---

## 7. Global Value System

- **Delegate entirely.** The native interface exposes thin operations that pass through to the global value system (direction at #19536; not finalized).
- Checkpoint / rollback is the global value system's concern, not the native interface's.

In practice today, only existence checks are invoked via a native — `borrow_global`, `move_to`, `move_from` etc. are bytecode ops, not native calls. The native-facing context API is correspondingly small:

```rust
ctx.exists_at(addr: AccountAddress, ty: InternedType) -> bool
```

Table-specific operations (keyed by a `StorageKey`-style scheme rather than `address + type`) will be added separately once the global value system's table story firms up.

---

## 8. Events

Delegated to the event store ([`event_store.md`](event_store.md)). Natives only see emit; checkpoint and rollback are driven by the execution engine.

Context API:

```rust
ctx.emit_event(ty: InternedType, value: ValuePtr) -> Result<(), _>
```

`ty` and `value` come from the calling Move frame via §3's accessors; the context plumbs them through to the event store.

---

## 9. Results / Errors

All errors are transaction-aborting — execution state is discarded on failure, so natives don't need to clean up the frame or restore state. On success, the native must have written the correct number of return values at the expected offsets; the interpreter proceeds unconditionally.

Errors split into two channels:

- **VM-internal** (`VMInternalError`): not meant to be inspected by the native — bubbles back to the VM via `?`. It is the VM's type-erased boundary error, so any component error erases into it.
- **Native-visible**: the native handles them explicitly via its own error type.

More practically, the native context methods should follow these patterns:
- A method that produces only the former returns `VMResult<NativeStatus>` (i.e. `Result<NativeStatus, VMInternalError>`)
- A method that produces both returns `VMResult<Result<NativeStatus, CustomError>>`
  - One with only native-visible errors can return simplified form `Result<NativeStatus, CustomError>`.

See [`error_design.md`](error_design.md) for the broader VM error story.

---

## 10. Code & Function

**TODO.** All deferred:

- Function resolution & loading
- Function values / closures
- Dynamic dispatch (e.g. dispatchable fungible assets, account abstraction)
- Caller-frame introspection (e.g. `event::write_module_event_to_store` reading the caller's module ID for the cross-module-emit check)

---

## 11. Gas Metering

**TODO.** Detailed design lives in [`gas_design.md`](gas_design.md). High-level direction for natives:

- **Constant-cost natives** (e.g. `signer::borrow_address`) — fixed cost, charge upfront. Could potentially batch with the surrounding basic block's gas charge, though savings may not justify the complexity.
- **Data-dependent cost** (vector operations, serialization, etc.) — inspect input size, charge upfront before the work.
- **Iterative or unpredictable cost** — charge incrementally (e.g. per loop iteration). Per-iteration charging still satisfies charge-before-work. Post-hoc charging is acceptable when cost is most naturally computed after the work, but only for transient bounded-constant violations that are carefully reviewed.

Contract: **charge before work**. See [`vm_security_and_correctness.md`](vm_security_and_correctness.md) for broader gas-metering invariants.

---

## 12. Miscellaneous Open Items

- **Runtime reference checks.** The current VM has an opt-in runtime reference checker for borrow-rule safety. Whether MonoMove keeps an equivalent is open.

---

## 13. Security Considerations

Native functions are trusted Rust code that bypasses the bytecode verifier — every native extends the VM's TCB. A bug in a native can violate any VM invariant (memory safety, type safety, gas metering, determinism) without any static check catching it. Treat them as a disproportionate risk relative to code volume.

Native function safety is **best-effort by design.** The interface does what's cheaply done (typed accessors, descriptor-driven GC tracing, audit-driven invariants), but natives ultimately have direct access to VM internals when they need it and are responsible for their own correctness.

Concerns specific to natives:

- **Memory and type safety.** Direct stack/heap access; an incorrect read or write silently corrupts execution state.
- **Reference safety.** Borrow semantics are modeled but not runtime-verified; minimize ref-returning natives.
- **Gas metering.** Natives charge their own gas; undercharging is a DoS vector. The asymptotic-safety invariant must hold for every native.
- **Determinism.** No non-determinism — especially relevant when calling external crypto libraries.
- **Boundedness.** Loops, recursion, and allocation must be bounded. Self-managed shadow memory bypasses these bounds entirely and is therefore disallowed (§4).
- **Panic safety.** A panic crashes the node — including panics from dependencies (`unwrap`, OOB indexing, third-party crate assertions).
- **Calling convention violations.** Wrong return offsets, wrong arity, frame inconsistency — the interpreter trusts the native unconditionally on success.

**Distributed ownership.** Natives are owned by different teams (crypto, framework, etc.). Domain teams may not know VM invariants; the VM team may not know each domain. Bridging this gap is something we'll need to look into further.

**Mitigation.** Constrained-interface helpers (§3 typed accessors, §4 wrappers) limit accidents but don't make natives safe — they're powerful by design. Audit remains essential.
