# Basic-Block Gas Metering

## 1. Motivation

The current MoveVM gas metering charges per instruction inside the interpreter dispatch loop. While correct and simple, this design has several drawbacks:

1. **Overhead on the hot path.** Per-instruction metering adds a subtraction and branch to every iteration of the interpreter dispatch loop. As the VM becomes more optimised and the cost of each dispatch iteration shrinks, the relative overhead of per-instruction metering will grow.
2. **Performance coupling.** Gas metering logic is interleaved with instruction execution, preventing the VM from applying standard compiler optimisations such as instruction reordering, fusing, inlining, and IR lowering — all of which may change the number or identity of "instructions" that execute, risking a change in the observed gas cost.
3. **JIT compilation barrier.** A per-instruction metering model assumes a 1:1 correspondence between bytecode instructions and executed operations. JIT compilation breaks this assumption, making it impossible to adopt JIT without redesigning the metering layer.
4. **Cost model leaks implementation details.** While some coupling between gas costs and VM internals is unavoidable, the per-instruction metering model makes it easy for unnecessary implementation details to leak in — such as cache state, value representation, or runtime type structure — making it harder to evolve the VM without breaking gas semantics.

### Design Goals

- **Decouple metering from execution.** Gas is charged at basic-block granularity, freeing the VM to optimise the interior of each block arbitrarily.
- **JIT-compatible.** Charge ops are part of the instruction sequence, so no per-dispatch metering hook is needed. A JIT compiler can emit gas checks at basic-block boundaries without special-casing the metering layer.
- **Minimal hot-path overhead.** One subtract-and-branch per basic block rather than per instruction.

The current implementation instruments at the micro-op level using an ISA-agnostic framework — the metering pass has no dependency on any instruction set, and instruction sets plug in by implementing four traits. Gas costs are fully static (type costs are resolved during monomorphisation), but the choice of instrumentation point is not yet settled — see §3.

---

## 2. Terminology

| Term | Definition |
| --- | --- |
| **Basic block (BB)** | A maximal straight-line sequence of instructions with a single entry point and no internal branches. |
| **BB cost** | The sum of the static (`base`) costs of every instruction in the block. |
| **Budget** | The remaining gas available for execution; initialised from the transaction's `max_gas_amount`. |
| **Charge op** | A `Charge { cost }` instruction inserted at each basic-block entry by the instrumentation pass. The interpreter deducts `cost` from the budget when it reaches this op. |
| **Dynamic charge op** | An instruction with a runtime-variable cost component returns a fully-formed charge instruction in `InstrCost::dynamic`; the instrumentation pass inserts it immediately after that instruction. |

---

## 3. Where to Instrument

*This is an open design question; no decision has been made yet.*

The instrumentation pass can be applied at any layer of the compiler pipeline. The three main candidates are the original Move bytecode, the stackless execution IR, and the monomorphised micro-ops. Each presents a different trade-off between accuracy and stability.

### Move bytecode

**Pros:**
- Stable, well-defined public interface. Gas semantics are derived from what the user wrote, not from how the compiler chose to lower it.
- Isolated from compiler changes: backend optimisations cannot affect observed gas costs.
- Compatible with the existing gas schedule and gas profiler.

**Cons:**
- Less accurate. A single high-level bytecode instruction may expand to many micro-ops, so the charge may not reflect the actual work done.
- Generic bytecode is not yet monomorphised: type sizes are unknown, making it impossible to give precise costs to instructions whose work scales with the size of their operands (e.g. `move_from`, `move_to`, memory copies).
- If the compiler transforms the original instructions, the bytecode costs and the actual execution costs diverge.

### Stackless execution IR

**Pros:**
- More stable than micro-ops: insulated from low-level codegen decisions while still more accurate than raw bytecode.

**Cons:**
- Not a public or stable interface today. Gas semantics derived from it could still shift as the compiler pipeline evolves.
- Generic code is not yet monomorphised: type sizes are unknown, making it impossible to give precise costs to instructions whose work scales with the size of their operands.

### Micro-ops

**Pros:**
- Maximally precise: costs reflect the actual operations the interpreter executes.
- Types are concrete and sizes are known, enabling accurate static costs for size-dependent instructions without runtime probing.
- Instrumenting micro-ops is structurally simple — the ISA is flat and frame-pointer-relative.

**Cons:**
- Tight backward compatibility constraints. Any change to the compiler's codegen — a new optimisation, a different lowering strategy, adding or removing a micro-op variant — directly changes the gas cost of a function. Each such change would need to be feature-gated to preserve gas semantics across protocol upgrades.

The ISA-agnostic design of `mono-move-gas` means the instrumentation pass itself does not change regardless of which layer is chosen — only the impl file (the four trait implementations and the gas schedule) differs. The current micro-op plug-in is therefore a working prototype that can be evaluated against a bytecode-level or IR-level plug-in once the decision is made.

---

## 4. Instrumentation Pass

### 4.1 Overview

The instrumentation pass takes a flat instruction sequence and returns a new sequence with charge ops inserted. It runs once — the resulting sequence is stored and executed by the interpreter without any further consultation of the gas schedule.

The pass performs three steps:

1. **CFG construction.** Partition the instruction sequence into basic blocks by identifying leaders: instruction 0, every branch target, and every instruction immediately following a branch.

2. **Cost computation.** Look up the cost of every instruction in the gas schedule. Sum the static (base) costs within each block to get the block's charge amount. Identify instructions with a dynamic cost component that will need an extra charge op inserted after them.

3. **Emission.** Walk the original instruction sequence. At each basic-block leader, prepend a `Charge` op with the block's total cost. For each instruction with a dynamic cost component, append a dynamic `Charge` op immediately after the instruction. Remap all branch targets to account for the inserted ops (see §4.3).

For instructions with a runtime-variable cost, there are two options: emit a separate dynamic `Charge` instruction, or handle the variable charge inline in the interpreter for that specific instruction. The design supports both — the gas schedule decides per instruction.

### 4.2 Example

A simple accumulation loop compiled to micro-ops (each instruction costs 3 in the current placeholder schedule):

```
// Slots: sum = fp[0], i = fp[8]
//
// Before:
//  0: StoreImm8 { dst: sum, imm: 0 }
//  1: StoreImm8 { dst: i, imm: 0 }
//  2: JumpGreaterEqualU64Imm { target: 6, src: i, imm: N }   ← loop header (BB1)
//  3: AddU64    { dst: sum, lhs: sum, rhs: i }               ← loop body (BB2)
//  4: AddU64Imm { dst: i, src: i, imm: 1 }
//  5: JumpLessU64Imm { target: 3, src: i, imm: N }
//  6: Return                                                  ← exit (BB3)
//
// Basic blocks: BB0=[0,1] cost 6, BB1=[2] cost 3, BB2=[3,4,5] cost 9, BB3=[6] cost 3
//
// After:
//  0: Charge(6)
//  1: StoreImm8 { dst: sum, imm: 0 }
//  2: StoreImm8 { dst: i, imm: 0 }
//  3: Charge(3)
//  4: JumpGreaterEqualU64Imm { target: 9, src: i, imm: N }   ← remapped 6 → 9
//  5: Charge(9)
//  6: AddU64    { dst: sum, lhs: sum, rhs: i }
//  7: AddU64Imm { dst: i, src: i, imm: 1 }
//  8: JumpLessU64Imm { target: 3, src: i, imm: N }           ← remapped 2 → 3
//  9: Charge(3)
// 10: Return
```

Each basic-block entry is now prefixed with a `Charge` op that debits the total cost of that block from the budget. Branch targets point at `Charge` ops so the budget is always decremented before any block executes.

### 4.3 Branch-Target Remapping

Inserting charge ops shifts every instruction index, so all branch targets must be remapped. The new index of a target `t` is `t` plus the number of `Charge` ops inserted before it, which can be computed in a single pass over the original sequence.

### 4.4 Constraint: No Dynamic Cost on Branch Instructions

A branch instruction must not have a dynamic cost component. The dynamic charge op is inserted immediately after the instruction, so on the taken path execution jumps away and the charge is never reached. For unconditional branches it is completely unreachable.

### 4.5 Dead Code

The pass instruments every basic block, including unreachable ones. The compiler should eliminate dead basic blocks before this pass runs, both to avoid wasted allocation and to prevent dead `Charge` ops from polluting the instruction cache.

---

## 5. Load-Time Type Costing

*Not yet implemented.*

### 5.1 Problem

The current gas model charges for type construction and generic instantiation at runtime, which:

- Creates variable-cost instructions whose gas depends on the type arguments supplied.
- Couples metering to the VM's type cache and monomorphisation strategy.
- Makes it impossible to pre-compute BB costs statically.

### 5.2 Design Principle

**Type construction cost is separated from instruction cost and charged at load time.**

All instruction costs become constants, independent of type parameters. Type costs are charged once per unique instantiation, at the point where the type is first "mentioned" in a call chain, not when it is constructed at runtime. Both the per-call-site cost and the per-function-body cost are constants folded into BB costs by the instrumentation pass.

### 5.3 Cost Decomposition: Caller vs. Callee

The type cost is split between two sites, both computed at **code-loading time** (not at monomorphisation or execution time):

### Caller Side: Concrete Type Argument Cost

The caller knows the concrete types it passes. It pays for the **node count** of each type argument it supplies. This cost is folded into the BB cost of the block containing the call.

Example — the call site:

```
foo<A<A<u64>>>
```

The type `A<A<u64>>` has 3 nodes: `A`, `A`, `u64`. The caller is charged a type cost of **3**.

### Callee Side: Internal Type Construction Cost

The callee's code may construct new types using its type parameters. It pays for the **additional structure** it adds, counting each type parameter occurrence as 1.

Example — the callee:

```
fun foo<T>() {
    bar<T>();           // type arg is just T → 1 node (T counts as 1)
    bar<B<A<T>>>();     // type arg is B<A<T>> → 3 nodes (B, A, T)
}
```

The callee-side type cost of `foo` is **1 + 3 = 4**. This is a fixed constant per function, independent of what `T` is instantiated with. It is computed once at code loading and folded into the cost of the function's first basic block, so it requires no separate metering point.

### Combined Cost

For the call `foo<A<A<u64>>>`:

| Site | Charge | Rationale |
| --- | --- | --- |
| Caller (in the calling BB) | 3 | Pays for the concrete type tree it supplies |
| Callee (at `foo` entry) | 4 | Pays for the type structure it constructs internally |
| **Total type cost** | **7** |  |

This total is the same regardless of whether `T = u64` or `T = A<A<u64>>`. The caller absorbs the complexity of the supplied type; the callee absorbs the complexity of what it builds on top.

### 5.4 Why This Works

- **No runtime type inspection needed for gas.** Costs are constants derived from the code structure.
- **Generic and non-generic code have identical instruction costs.** The type cost is a separate, additive charge.
- **Composable.** Nested generic calls accumulate type costs through the call chain, each level paying for exactly the structure it introduces.
- **Cache-independent.** Whether a type is cached or freshly constructed has no effect on gas.

**Assumption: type interning.** This design assumes that types are interned, making type copying and comparison O(1) operations (pointer/index equality). Without interning, passing or copying a type like `A<A<A<u64>>>` would have cost proportional to its depth, reintroducing a runtime type-size dependency into instruction costs. Type interning is therefore a prerequisite for this cost model.

### 5.5 Propagation Through Nested Calls

A subtlety: when `foo<T>` calls `bar<T>()`, the caller-side cost at that call site is **1** (just the `T` node), *not* the depth of whatever `T` was concretely instantiated with. The original outer caller already paid for the concrete type's depth. Each level in the call chain pays only for the type structure *it introduces*.

This means that if `caller()` invokes `foo<A<A<u64>>>()`, and `foo<T>` invokes `bar<T>()`, the cost of the concrete type `A<A<u64>>` is charged once (at the `caller` site), not again when it flows through as `T`.

---

## 6. GC and Memory Cost

*Not yet designed.*

### 6.1 Problem

The MonoMove runtime uses a garbage-collected heap. A program can trigger unbounded memory allocation or expensive collection cycles, so a cost model is needed that charges for heap allocation and accounts for GC overhead without requiring per-object tracking at execution time.

---

## 7. Storage Costs

*TBD.*

### 7.1 Problem

Reading from and writing to global storage (e.g. `move_from`, `move_to`, `borrow_global`) involves IO and state-tree operations whose cost depends on factors like value size and proof path length. These costs are separate from instruction execution costs and need their own charging model.

---

## 8. Module Loading Cost

*Not yet designed.*

### 8.1 Problem

Modules are cached, so the expensive work — deserialization, verification, and compilation — only happens on cache misses. Charging on every access overcharges cache hits, but charging only on misses makes gas costs depend on cache state.

---

## 9. Costs for Size Computation (Layouts)

*Not yet designed.*

### 9.1 Problem

Certain operations need to know the serialized or in-memory size of a value (e.g. for storage cost calculations or bounds checks). Layouts are cached, so the traversal to compute them only happens on cache misses. As with module loading, charging on every access overcharges cache hits, but charging only on misses makes gas costs depend on cache state.
