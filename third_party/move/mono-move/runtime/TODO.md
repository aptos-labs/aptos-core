# Runtime TODO

## Integer widths
- Support remaining unsigned integer types: u8, u16, u32, u128, u256
- Signed integers

## Comparison and boolean ops
- Compare-to-register: EqU64, LtU64, GtU64, LeU64, GeU64 (produce 0/1 in a dst slot)
- Boolean: And, Or, Not (operate on 0/1 values)
- Deep equality for complex objects (vectors, structs, enums)
  - Needs recursive comparison traversing heap objects
  - Open question: runtime support v.s. compiler-generated helper?

## Global storage 
- MoveFrom / MoveTo / BorrowGlobal(Mut) / Exists
- Need to figure out how to interact with the block-level cache

## Abort / error handling
- Abort instruction: exit with error code
- AbortMsg variant

## Gas metering
- Hybrid model: the re-compiler statically computes the cost of each basic block
  and emits a single charge at block entry -- absolutely crucial for minimizing
  dispatching overhead.
- Dynamic charges still apply to operations whose cost depends on runtime values
  (e.g. data size, global storage reads/writes). These are charged at the instruction 
  site.
- Out-of-gas exits cleanly (like Abort).

## Execution performance
- **Hoist interpreter state into locals**: refactor the interpreter loop so
  that pc, fp, and func_id are local variables (kept in machine registers)
  rather than fields read/written through `&mut self` each iteration. LLVM
  can't keep them in registers today because heap operations take `&mut self`,
  which may alias these fields.

Down the road, there are several techniques we can explore to improve
interpreter dispatch and execution speed:
- **Super-instructions**: fuse common instruction sequences (e.g. load + compare + branch)
  into single opcodes, reducing dispatch overhead
- **Threaded dispatch**: computed goto / direct threading to eliminate the
  central switch/match. Current match-based dispatch compiles to a jump table,
  which is decent but still has indirect branch misprediction costs
- **Copy-and-patch**: JIT-compile by copying pre-compiled machine code
  templates and patching in operands. Near-native speed with much less
  complexity than a full JIT compiler

## Deep copy
- CopyLoc on heap-allocated struct/enum needs recursive deep copy of entire object graph
- Can trigger GC mid-copy (allocating the copy)
- Open question: runtime instruction or re-compiler emits copy loops?

## Closures
- PackClosure: capture environment values into a callable heap object
- CallClosure: dispatch to captured function with environment

## Native function calls
- Need a native function table (function pointer + arity/signature metadata)
- Generics: natives are NOT monomorphized, so they need runtime type info
  - Need a way to pass type arguments to natives (type descriptor table? type tags?)
  - Some natives dispatch dynamically on type args (e.g., bcs::to_bytes<T> needs T's layout)
- GC safety: natives that allocate heap objects (e.g., vector::empty) need ensure
  those objects are being included in the root set during GC scans.
