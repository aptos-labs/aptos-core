# Basic-Block Gas Metering

## 1. Motivation

The current MoveVM gas metering charges per instruction inside the interpreter dispatch loop. While correct and simple, this design has several drawbacks:

1. **Overhead on the hot path.** Per-instruction metering adds a subtraction and branch to every iteration of the interpreter dispatch loop. As the VM becomes more optimised and the cost of each dispatch iteration shrinks, the relative overhead of per-instruction metering will grow.

Additionally, block-granularity metering makes it easier (though is not strictly required) to address several related design goals:

2. **Decoupling metering from execution.** When gas metering is interleaved with instruction execution, compiler optimisations such as reordering, fusing, and inlining risk changing the observed gas cost. Block metering reduces this coupling by giving the VM freedom to optimise within each block.
3. **JIT compatibility.** A JIT compiler can emit gas checks at block boundaries without special-casing the metering layer. Per-instruction metering does not preclude JIT, but block metering makes it more natural.
4. **Cost model stability.** Per-instruction metering makes it easy for implementation details — cache state, value representation, runtime type structure — to leak into gas costs. Block metering encourages a cleaner separation, though the choice of instrumentation point (§3) ultimately determines how much leaks through.

Gas is instrumented at the **stackless execution IR** level (§3), where basic blocks are already explicit. Per-block static costs are computed during lowering and charged when control transfers into each block, with no standalone charge op — see §4. Gas costs are fully static, since types are concrete by the time lowering runs.

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

**Decision: the stackless execution IR.** Its basic blocks are explicit (a `Vec<BasicBlock>` with stable `Label`s) and the lowerer already walks them block by block, so per-block costs can be computed during lowering with no separate CFG-reconstruction pass, no branch-index remapping, and no safe-point PC remapping. It is more stable than the micro-ops (insulated from low-level codegen decisions) while still able to resolve concrete type sizes, since lowering monomorphises before emitting code. The trade-offs that led here, across the three candidate layers, are below.

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
- The IR is polymorphic, so type sizes are not knowable from the IR alone. This is sidestepped by computing costs *during lowering*, which monomorphises first: size-dependent costs read the concrete widths the lowering context has already resolved.

### Micro-ops

**Pros:**
- Maximally precise: costs reflect the actual operations the interpreter executes.
- Types are concrete and sizes are known, enabling accurate static costs for size-dependent instructions without runtime probing.
- Instrumenting micro-ops is structurally simple — the ISA is flat and frame-pointer-relative.

**Cons:**
- Tight backward compatibility constraints. Any change to the compiler's codegen — a new optimisation, a different lowering strategy, adding or removing a micro-op variant — directly changes the gas cost of a function. Each such change would need to be feature-gated to preserve gas semantics across protocol upgrades.

The ISA-agnostic design of `mono-move-gas` means the instrumentation pass itself does not change regardless of which layer is chosen — only the impl file (the four trait implementations and the gas schedule) differs. The current micro-op plug-in is therefore a working prototype that can be evaluated against a bytecode-level or IR-level plug-in once the decision is made.

---

## 4. Fused Block-Charge Instrumentation

### 4.1 Overview

Instrumentation is folded into lowering rather than run as a separate pass. As the lowerer emits a block's micro-ops, it sums the IR-level cost of that block's instructions (§4.2). It charges no cost via a standalone op; instead each block's cost is charged by the instruction that transfers control into it:

- **Entry block.** Block 0's cost is stored on the function as `entry_gas`. The call protocol charges it before any of the callee's instructions run — in `call_unchecked` for callees, and at the start of `run` for the root invocation.

- **Every other block.** Its cost is stored on the predecessor jump that targets it. The unconditional `Jump` carries a single `gas` field (the target block's cost). Each conditional jump carries `gas_taken` and `gas_fallthrough` (the costs of the taken and fallthrough blocks); the interpreter charges exactly one, for the block it transfers into, before updating the pc.

A block's cost is therefore always debited before the block executes: on entry the interpreter has already charged the block's full cost (its body **and** its terminating jump). A terminator's own cost belongs to its block and is charged on entry to that block; the gas a terminator *charges* is its successor's. No gap, no double count.

Because no op is inserted, micro-op offsets match an uninstrumented lowering: branch targets resolve directly to block leaders, and safe-point PCs need no remapping. It also removes one dispatch per block on the hot path (the §1 motivation), since the charge is folded into a jump that was going to execute anyway.

### 4.2 Cost computation

Costs come from an IR-level gas schedule keyed on the stackless IR instructions, mirroring the work each lowers to. Size-dependent costs (e.g. `ReadRef`, `Move`, vector ops, copying call arguments and return values) resolve concrete byte widths from the lowering context, which has already monomorphised the function. All costs are therefore static — a block's total is a single constant — so no runtime-variable charge mechanism is needed.

### 4.3 Example

A simple accumulation loop. Each block's cost is charged when control transfers into it; the loop header's cost is split across the two jumps that reach it.

```
// Slots: sum = fp[0], i = fp[8]
//
//   L0 (entry):  StoreImm8 sum,0; StoreImm8 i,0; Jump L1            cost 6  → entry_gas
//   L1 (header): JumpGreaterEqualU64Imm L3, i, N  (else L2)         cost 3
//   L2 (body):   AddU64 sum,sum,i; AddU64Imm i,i,1; Jump L1         cost 9
//   L3 (exit):   Return                                            cost 3
//
// Fused gas:
//   entry_gas = 6                              (charged by the call protocol)
//   L0's `Jump L1`                 gas = 3      (cost of L1)
//   L1's conditional jump          gas_taken = 3 (L3),  gas_fallthrough = 9 (L2)
//   L2's `Jump L1`                 gas = 3      (cost of L1)
```

### 4.4 Dead code

Costs are summed only for blocks the lowerer emits, and the charge is attached only to real jumps and calls, so unreachable blocks add no standalone ops. The compiler should still eliminate dead basic blocks upstream to avoid emitting them at all.

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

### 5.6 Substitution and Canonicalization Cost

Type interning requires canonicalization after substitution. Consider:

```
vec<vec<vec<Bar<T>>>>   // generic type, node count N = 5
T = Bar<u64>            // concrete argument, node count M = 2
```

Substituting `T = Bar<u64>` produces `vec<vec<vec<Bar<Bar<u64>>>>>`. The interner must then re-canonicalize — traversing from the substitution point up to the root — which is at most O(N) work. The substitution itself is at most O(M).

Both costs are constants: the caller pays M for the concrete type argument, and the callee pays N for each generic type it constructs. This holds even when the same type parameter is used multiple times:

```
fun foo<T>() {
    bar<Bar<T>>();   // callee-side cost: 2 (Bar, T)
    bar<Bar<T>>();   // callee-side cost: 2
    bar<Bar<T>>();   // callee-side cost: 2
    ...              // 7 calls total
}
```

The caller pays once for the concrete `T`. The callee pays 7 × 2 = 14 for the seven `Bar<T>` constructions it introduces, which covers the canonicalization work for each substitution. The cost of the inner `T` is absorbed — the monomorphizer pays for canonicalization of each generic type it builds, and that cost is function-local.

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
