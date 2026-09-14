# specializer

This crate defines a polymorphic stackless execution IR and performs conversion from Move bytecode to the stackless execution IR.
The stackless execution IR is then lowered into monomorphic micro-ops, when all types used in the function are fully concrete and thus type size and layout information is available.

## Goals of converting from Move bytecode to stackless-exec-ir

- eliminate the implicit operand stack (to reduce operand stack traffic to and from locals)
- keep conversion close to linear time
- preserve polymorphism until later just-in-time monomorphization
- make dataflow explicit enough for local optimization and allocation
- remain simple enough that correctness is easy to reason about
- carry each instruction's originating bytecode offset through every pass

## Argument-deserialization intrinsic

`lower/txn_arg.rs` resolves calls to the VM-provided `txn_arg::deserialize<T>`
at lowering, when `T` is concrete, into the module's deserializer for `T`'s
shape (`deserialize_vector<E>`, `deserialize_option<E>`, `deserialize_string`,
...), or for any other struct or enum into `deserialize$S` of the module the
loader generates for `S`'s defining module. The rewrite happens where call
sites are built in `lower/context.rs`; it is sound because `CallIndirect`
targets by name and every target has the intrinsic's signature. A type with no
deserializer fails lowering or loading with `NotATransactionArgument`.

## Test Infrastructure

The specializer pipeline is exercised by the **differential tests** in the `mono-move-testsuite` crate. See [`../testsuite/AGENTS.md`](../testsuite/AGENTS.md) for the harness, `// RUN:` directives, and baseline workflow.
