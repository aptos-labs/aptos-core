# mono-move-runtime

Executes **micro-ops**: the flat, monomorphic instruction set the specializer
produces from Move bytecode. Register-based interpreter over a unified linear
stack, with a bump-allocated heap and Cheney's copying GC. Input is assumed
already verified and lowered by the MonoMove pipeline.

Design rationale lives in `../docs/`. Read the relevant one before changing a
value layout, the calling convention, or the GC.

## Safety model

The interpreter uses raw pointer arithmetic throughout. Correctness rests on
three invariants held jointly by the compiler, the verifier, and the runtime.
Any change that could break one needs a matching verifier check or a proof:

1. **Frame metadata integrity** — saved `fp`/`pc`/`func_ptr` are written only by
   call/return, never by user micro-ops.
2. **Pointer-slot accuracy** — `Function::frame_layout` and the matching
   `safe_point_layouts` entries exactly describe the frame slots holding live
   heap pointers. The GC trusts them to find roots.
3. **Object header integrity** — `descriptor_id` and `size_in_bytes` sit at
   `obj_ptr - 8` and `obj_ptr - 4`, written by the allocator. User micro-ops
   address only offsets `>= 0`, so they cannot reach the header.

`verify_program` checks frame-access bounds, metadata overlap, jump targets, and
descriptor validity before execution. Everything else is the compiler's
responsibility.

## Conventions

- Every `unsafe` block carries a `// SAFETY:` comment naming the invariants it
  relies on.
- Bare `unwrap()` is banned outside tests. Use `expect()` only when the property
  is local and easy to prove; otherwise return an error. Tests may use
  `unwrap()` freely.
- All arithmetic is checked unless the absence of overflow is proven.
- New micro-ops follow the naming in `mono-move-core/src/instruction/`.

## Commands

```bash
cargo check -p mono-move-runtime
cargo test -p mono-move-runtime
cargo test -p mono-move-runtime -- <name>
```

## Pre-PR

- [ ] `cargo +nightly fmt -- --check` passes
- [ ] `cargo test -p mono-move-runtime` passes
- [ ] Affected docs in `../docs/` and this file updated — check for renamed
      modules, changed layouts, stale descriptions
