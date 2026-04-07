# Specializer Design

Specializer is a conversion pipeline for making Move bytecode ready for
execution. It is made of two phases:
1. a "destack" pipeline converting Move bytecode to a stackless execution IR
   (stackless here refers to the removal of the implicit operand stack)
2. a "lowering" pipeline converting the stackless execution IR to monomorphized
   micro-ops, once all the type sizes and layouts are known

## Goals

Stackless execution IR:
- is polymorphic
- has instructions that can specify "named slots" or const values as operands

The destack pipeline should:
- eliminate the implicit operand stack, reducing "stack traffic"
- reduce number of instructions compared to stack-based bytecode
- keep conversion close to linear time
- preserve polymorphism until later monomorphization
- make dataflow explicit for optimizations
- do as much work as possible to make the later lowering pipeline very fast
- remain simple enough that correctness is easy to reason about

The lowering pipeline should:
- be very fast, as it can be invoked during runtime for lazy monomorphization
- convert "named slots" in stackless execution IR into "sized slots" based on
  fully concretized type information

## Destack pipeline

- convert stack-based Move bytecode into a partial SSA form, eliminating the
  implicit operand stack, assigning each intermediate value that would live on
  the implicit operand stack a unique value ID (SSA is intra block and on the
  value IDs only, not on params or locals, so no phi nodes needed as operand
  stack is empty at block boundaries)
- fusion of instructions
- greedy slot allocation mapping value IDs to home and transfer slots with
  type-keyed slot recycling
- optimization passes, such as identity move elimination, copy propagation, dead
  instruction elimination, slot renumbering

## Named slots

- value ID slots (v0, v1, ...): SSA value IDs that exist only in the pre-slot
  allocation IR
- home slots (r0, r1, ...): the function's frame-local storage. Encompasses
  params, declared locals, and     
  temporaries created by destacking. Each maps 1:1 to a frame slot. They persist
  for the function's lifetime.   
- xfer (transfer) slots (x0, x1, ...): the call interface region. Used to pass
  arguments to a callee and receive return values back. They overlap with the
  callee's parameter/return area, so producing a value directly into an transfer
  slot avoids a copy at the call boundary. Transfer slots are clobbered by every
  call.

Xfer slot candidacy: A value may be assigned directly to `Xfer(i)` as an
outgoing call argument if:
- it is a temp SSA value
- it has a single reaching def in the same block
- there is no intervening call between def and use
- its last use is that call instruction
- it is not already committed to a conflicting coalescing decision

A call result may stay in `Xfer(i)` if:
- it is only used within the same basic block
- no later call clobbers it before its last use
