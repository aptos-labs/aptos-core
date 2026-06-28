# specializer

This crate defines a polymorphic stackless execution IR and performs conversion from Move bytecode to the stackless execution IR.
The stackless execution IR is then lowered into monomorphic micro-ops, when all types used in the function are fully concrete and thus type size and layout information is available.

## Goals of converting from Move bytecode to stackless-exec-ir

- eliminate the implicit operand stack (to reduce operand stack traffic to and from locals)
- keep conversion close to linear time
- preserve polymorphism until later just-in-time monomorphization
- make dataflow explicit enough for local optimization and allocation
- remain simple enough that correctness is easy to reason about

## Test Infrastructure

The specializer pipeline is exercised by the **differential tests** in the `mono-move-testsuite` crate. See [`../testsuite/AGENTS.md`](../testsuite/AGENTS.md) for the harness, `// RUN:` directives, and baseline workflow.
