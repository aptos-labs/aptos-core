# Stackless Execution IR V2 Design

This document describes the `specializer` v2 pipeline from Move bytecode
to the final polymorphic stackless execution IR. It reflects the current
in-progress implementation in `specializer` and recommends the remaining
changes needed to make the pipeline coherent, analyzable, and ready for the
later micro-op lowering stage.

The scope of this document is the v2 pipeline only. The v1-only conversion path
is out of scope except where it provides prior art or highlights a design
boundary.

## Goals

The v2 pipeline should:

- eliminate the implicit operand stack
- keep conversion close to linear time
- preserve polymorphism until later just-in-time monomorphization
- make dataflow explicit enough for local optimization and allocation
- reduce home-register traffic around calls by introducing arg registers
- remain simple enough that correctness is easy to reason about

## Current Status

The in-progress implementation already changed the IR model in three important
ways:

1. `Reg` is no longer a flat integer. It is now split into:
   - `Reg::Home(u16)` for params, locals, and temps
   - `Reg::Arg(u16)` for call-window slots
2. `FunctionIR` now tracks:
   - `num_regs` for home registers
   - `num_arg_regs` for arg registers
   - `reg_types` only for home registers
3. The v2 allocator performs conservative arg-register precoloring inside
   `allocate_block`, instead of relying on a separate late rewrite pass.

Those changes are directionally correct and this design keeps them.

The current v2 pipeline still has two structural issues:

- field-access fusion happens after allocation, which increases pressure and
  hides the true live ranges from the allocator
- liveness, coalescing, and arg-slot candidacy analysis are embedded inside the
  allocator instead of being represented as an explicit pre-allocation phase

This document recommends fixing those two issues while preserving the new
`Home`/`Arg` register split.

## Representations

The v2 pipeline operates on three conceptual layers.

### Move bytecode

Properties:

- stack-based
- polymorphic
- parameters and locals are named, but intermediate values live only on the
  implicit operand stack
- the bytecode verifier guarantees stack balance, local initialization, and
  type compatibility

### Block-local SSA conversion form

Properties:

- stackless
- still polymorphic
- values produced from the operand stack receive fresh SSA IDs within a basic
  block
- params and declared locals are represented as pinned home registers
- no phi nodes exist because the operand stack is empty at block boundaries

The current implementation reuses `Instr` for this form. That is acceptable
short-term, but the design must treat these IDs as a separate conceptual layer
from final allocated registers.

### Final stackless execution IR

Properties:

- operands and destinations are explicit
- params, locals, and temps live in home registers
- call-window values may live in arg registers
- still polymorphic
- ready for later lowering to monomorphic micro-ops on sized slots

## Register Model

The register model should remain:

```rust
enum Reg {
    Home(u16),
    Arg(u16),
}
```

with these invariants:

- `Home(i)` has a stable function-wide type
- `reg_types[i]` is defined only for `Home(i)`
- `Arg(i)` has no entry in `reg_types`
- the type of `Arg(i)` is derived from the defining instruction or the relevant
  call signature at the program point
- `Arg(i)` is call-clobbered
- initial implementation: `Arg(i)` values do not cross basic-block boundaries
- initial implementation: `Arg(i)` is not live past the next call unless that
  call redefines the same arg slot as a return value

`num_regs` counts only home registers. `num_arg_regs` counts the maximum arg
slot index used by the function plus one.

## Current V2 Pipeline

The current v2 pipeline is:

1. bytecode verification
2. bytecode -> block-local SSA conversion
3. SSA immediate-binop fusion
4. greedy allocation to home/arg registers
5. post-allocation field-access fusion
6. identity-move elimination
7. dead-instruction elimination
8. home-register renumbering

### 1. Bytecode verification

Input: `CompiledModule`

Output: trusted module for conversion

Relied-on invariants:

- balanced operand stack
- empty stack at basic-block boundaries
- valid branch targets
- valid local initialization
- call arities and return signatures match
- reference safety and type correctness

This pass is already in the correct position.

### 2. Bytecode -> block-local SSA conversion

Input: verified bytecode

Output: stackless, block-local SSA-like `Instr` stream

Established invariants:

- stack is empty at block boundaries
- every stack-produced value has one SSA def inside the block
- params and declared locals remain pinned home registers
- labels are assigned before conversion

Correctness reasoning:

- the Move verifier guarantees that the operand stack is empty at block
  boundaries, so per-block SSA is sufficient
- locals are mutable across blocks, so they must retain stable identities
- no phi nodes are required because stack values never flow through joins

Judgment:

- keep block-local SSA
- do not introduce full-function SSA
- do not introduce phi nodes

### 3. SSA immediate-binop fusion

Input: block-local SSA stream

Output: same stream with `Ld* + BinaryOp` fused to `BinaryOpImm`

Established invariants:

- fused immediates preserve the original dataflow
- no block boundary is crossed during fusion

Correctness reasoning:

- in this SSA form, the loaded value is a single-use temporary produced by the
  stack-machine discipline
- the implementation already asserts that the temporary has no other uses in the
  block

Judgment:

- keep this pass before allocation
- this is the right abstraction boundary

### 4. Greedy allocation to home/arg registers

Input: normalized SSA stream

Output: final stackless IR using home registers and arg registers

Current behavior:

- pinned locals map to `Home(0..num_pinned-1)`
- temp SSA values are allocated by type-aware reuse from a free pool
- `StLoc`-style look-ahead can coalesce a temp into a pinned local
- `CopyLoc`/`MoveLoc` producers may coalesce back into the source local
- call args and returns are conservatively precolored to `Arg(i)` inside the
  allocator

Correctness reasoning:

- type-aware reuse is sound because home registers are function-wide typed
- local coalescing is sound only when last-use guarantees no conflicting live
  uses remain
- arg precoloring is safer inside allocation than as a late rename because the
  allocator can treat arg slots as a separate register class

Current weakness:

- analysis and allocation are mixed together
- field access is not yet normalized, so live ranges are longer than necessary

### 5. Post-allocation field-access fusion

Input: allocated IR

Output: fused `ReadField`/`WriteField` and variant equivalents

Current status:

- implemented in `optimize_v2` using the shared `fuse_field_access`

Judgment:

- this pass is in the wrong place
- it should move before allocation

Reason:

- field-borrow temp registers become allocator-visible even though they are
  immediately consumed
- that inflates live ranges and can block profitable home-register reuse or
  arg-slot placement

### 6. Identity-move elimination

This pass is correctly post-allocation. It removes `Move(x, x)` and `Copy(x, x)`
created by coalescing.

### 7. Dead-instruction elimination

This pass is correctly post-allocation as a cleanup pass, but it should remain
conservative and only remove writes to dead non-param destinations.

### 8. Home-register renumbering

The current shared renumbering logic already preserves arg registers and only
compacts home registers. That is the correct direction.

Correctness reasoning:

- home-register compaction is semantics-preserving because home-register indices
  are not ABI-visible
- arg-register indices are ABI-visible and must not be renumbered

## Target V2 Pipeline

The intended pipeline should be:

1. verifier-trusted bytecode intake
2. CFG/basic-block discovery and label assignment
3. bytecode -> block-local SSA conversion
4. SSA normalization and canonicalization
5. SSA analysis
6. allocation to home/arg registers
7. post-allocation cleanup
8. final IR validation

### 1. Verifier-trusted bytecode intake

No change from today.

### 2. CFG/basic-block discovery and label assignment

This step is logically already present inside conversion. It can stay there
implementation-wise, but the design should treat it as a separate stage because
subsequent passes rely on the block structure.

Established invariants:

- every branch target maps to a label
- block starts and block ends are known before SSA analysis

### 3. Bytecode -> block-local SSA conversion

No semantic change from today.

Required invariants:

- stack empty at block boundaries
- no SSA value live across a boundary via the operand stack
- locals remain pinned home registers
- each stack-derived SSA value has one def

### 4. SSA normalization and canonicalization

This stage should contain:

- immediate-binop fusion
- field-access fusion
- optional trivial SSA copy cleanup if introduced later

Required reordering:

- move field-access fusion here from `optimize_v2`

Correctness reasoning:

- these rewrites preserve value semantics
- they shorten live ranges and simplify later analyses
- any transformation that does not depend on physical register identity belongs
  before allocation

#### Optional trivial SSA copy cleanup

If later changes to the converter or normalization passes begin producing
temporary-to-temporary `Copy` or `Move` instructions that are only SSA renaming
artifacts, add a small cleanup pass here.

This pass is intentionally narrow. It should only remove copies that are true
SSA aliases, such as:

```text
t1 := copy t0
t2 := add t1, t3
```

which can be rewritten to:

```text
t2 := add t0, t3
```

and then delete the redundant `copy`.

The pass should initially be restricted to cases where:

- both source and destination are temporary home-register SSA values
- the destination is used only as a renamed alias of the source
- no pinned local/home register semantics are involved
- no arg registers are involved

The pass must not rewrite:

- copies or moves from pinned locals
- moves into pinned locals
- instructions that encode local move semantics rather than mere renaming
- arg-register uses, unless arg-register semantics are modeled precisely enough
  to prove the rewrite correct

This cleanup is optional because the current v2 pipeline does not yet appear to
generate enough of these pure SSA rename instructions to justify a dedicated
pass. But if they appear later, this pre-allocation stage is the correct place
to remove them, because doing so shortens live ranges before allocation and
improves home-register reuse, local-slot coalescing, and arg-slot placement.

### 5. SSA analysis

This phase is not explicit today and should become explicit.

Analysis products:

- per-block def/use tables
- per-block last-use table
- def position for temp SSA values
- next-call boundaries
- candidate local-slot coalescing opportunities
- candidate arg-slot placements

This phase does not need a new persistent IR structure on day one, but the
design should define it as a conceptual pass with well-defined outputs.

#### Arg-slot candidacy

A value may be assigned directly to `Arg(i)` as an outgoing call argument if:

- it is a temp SSA value
- it has a single reaching def in the same block
- there is no intervening call between def and use
- its last use is that call argument
- it is not already committed to a conflicting coalescing decision

A call result may stay in `Arg(i)` if:

- it is only used within the same basic block
- no later call clobbers it before its last use
- its uses are compatible with direct reads from `Arg(i)`

The use-then-def pattern at a call is valid:

```text
a0 := ...
[a0] := call f, [a0]
```

This is correct because the incoming `a0` is consumed by the call and the
outgoing `a0` is defined by the same call. The incoming value must have last use
at that call argument.

#### Local-slot coalescing

A temp value may coalesce into a pinned local when:

- the destination local is the temp's final consumer
- no conflicting use extends beyond that point
- no arg-slot decision has already claimed the same live range

### 6. Allocation to home/arg registers

This remains a greedy allocation pass, but it should consume the results of the
prior analysis phase instead of recomputing them inline.

Responsibilities:

- map pinned locals to their existing home registers
- map temp SSA values to reusable home registers by type
- honor precomputed local-slot coalescing decisions
- honor precomputed arg-slot placement decisions
- keep arg slots out of the home-register free pool

Required invariants:

- home registers are the only registers eligible for `reg_types`
- arg registers are never inserted into the home-register free pool
- values assigned to the same home register have identical type
- values assigned to `Arg(i)` satisfy the arg-slot liveness restrictions

Correctness reasoning:

- precoloring is allocation-aware, so it avoids the invalid late-rename problem
- separating home and arg classes prevents the allocator from reusing ABI slots
  as ordinary temporaries
- same-block-only arg liveness avoids join semantics and path-sensitive typing

### 7. Post-allocation cleanup

This stage should remain intentionally small:

- identity-move elimination
- dead-instruction elimination
- home-register renumbering

Field-access fusion should no longer occur here.

### 8. Final IR validation

Add a final validation pass, at least in debug builds.

Validation checks:

- all `Home(i)` indices are `< num_regs`
- all `Arg(i)` indices are `< num_arg_regs`
- `reg_types.len() == num_regs`
- every `Home(i)` used in the body has an associated type
- `Arg(i)` is never relied on across a forbidden boundary in the initial design
- every instruction satisfies its operand class constraints

## Arg Registers In The Overall Design

Arg registers are not a separate late optimization pass in v2. They are part of
allocation.

This is the key distinction from the old prototype approach:

- `src/arg_regs.rs` is still acceptable as a v1-specific late pass
- it is not the right model for v2

Reasons:

- a late rename pass cannot enforce register-class constraints as cleanly as the
  allocator can
- renaming after allocation loses the true interference picture
- arg-slot decisions interact directly with home-register reuse
- v2 already has the right abstraction boundary: SSA analysis followed by
  allocation

The current in-progress v2 code already moved in the correct direction by
embedding conservative arg precoloring inside `allocate_block`. The next change
is not to move arg handling out again, but to factor its analysis out of the
allocator into an explicit pre-allocation analysis pass.

### Initial arg-register restrictions

The initial implementation should keep these conservative restrictions:

- same basic block only
- no propagation across labels or joins
- no carrying arg-register values across unrelated calls
- `CallClosure` remains unsupported for arg-slot promotion until the closure ABI
  slot order is fixed for micro-op lowering

These restrictions are enough to capture the common profitable patterns:

- producer -> call argument
- call result -> return
- call result -> next call argument
- wrappers and forwarding chains

## Pass Invariants and Correctness Summary

Each pass must establish invariants that the next pass can rely on.

### After SSA conversion

- stackless explicit dataflow
- pinned locals preserved
- block-local single-def temps
- empty stack at block boundaries

### After SSA normalization

- no immediate fusable pair remains
- no field-borrow/read or borrow/write pair remains when fusion is valid
- live ranges are closer to semantic minimum

### After SSA analysis

- last-use and def-position maps are available
- local-slot and arg-slot decisions are explicit
- allocation no longer needs to infer policy

### After allocation

- every temp value has a concrete home or arg register
- no arg register entered the home free pool
- home-register typing is preserved
- arg-slot liveness restrictions hold

### After cleanup

- no identity moves remain
- dead moves/copies are removed conservatively
- home-register numbering is compact
- arg-register numbering is unchanged

### After validation

- the final IR is internally consistent and ready for later micro-op lowering

## Recommended Changes From Today

The implementation should change in this order:

1. Move field-access fusion from `optimize_v2` to the SSA normalization stage in
   `convert_v2`.
2. Introduce an explicit SSA analysis phase that computes:
   - `def_pos`
   - `last_use`
   - `call_positions`
   - local-slot coalescing
   - arg-slot candidacy
3. Refactor `allocate_block` to consume analysis outputs instead of recomputing
   them inline.
4. Add a final validator for the home/arg register model.
5. Keep post-allocation cleanup small and arg-aware.

The following choices should remain:

- keep block-local SSA
- keep greedy type-aware home-register reuse
- keep arg registers inside allocation
- keep home-register-only renumbering

## Test Plan

The v2 pipeline should be tested at the conversion, normalization, allocation,
and cleanup levels.

### Conversion

- arithmetic and unary operators
- locals, `CopyLoc`, `MoveLoc`, `StLoc`
- struct and variant pack/unpack
- references and global borrows
- vectors
- calls, generic calls, returns
- branch-heavy control flow

### Normalization

- immediate-binop fusion
- field-access fusion before allocation
- no fusion when the borrow temp is reused later

### Allocation

- type-aware reuse of home registers
- coalescing into locals
- no reuse when live ranges conflict
- home-register typing preserved

### Arg registers

- producer -> call arg
- call result -> return
- call result -> next call arg
- mixed promoted and unpromoted args/results
- generic calls
- rejection across branches or later non-call uses

### Cleanup and validation

- identity moves removed
- dead instructions removed conservatively
- home-register compaction preserves semantics
- arg-register numbering preserved
- invalid home/arg register usage rejected by validator

## Non-Goals

This design does not:

- add full-function SSA
- introduce phi nodes
- define the later monomorphic micro-op IR in detail
- optimize across basic-block joins for arg-register liveness
- support closure-call arg-slot promotion in the initial version

## Summary

The in-progress code has already made the most important architectural move:
home registers and arg registers are now distinct classes, and v2 handles arg
placement inside allocation rather than with a late rewrite. The correct next
step is to finish the architecture around that decision:

- normalize more before allocation
- make analysis explicit
- keep allocation greedy but policy-driven
- keep cleanup minimal
- validate the final IR

That yields a v2 pipeline whose invariants match the eventual micro-op calling
convention while preserving simple correctness reasoning and close-to-linear
conversion cost.
