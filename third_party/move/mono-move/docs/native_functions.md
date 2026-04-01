# Native Functions

> **WIP**: This document lays out the high-level design direction for native functions. The actual native function interface has not been implemented yet.

## Core Principle

Native functions are first-class citizens in MonoMove. They should have direct access to VM internals — stack and heap memory, the loader, the gas meter, etc. — and conceptually behave like VM instructions rather than opaque external calls. This ensures that native functions are efficient, composable with the rest of the execution engine, and subject to the same safety and metering guarantees.

## Calling Convention

Native functions follow the same calling convention as Move functions (see `docs/stack_and_calling_convention.md`):

- **Arguments** are read directly from the stack frame via `fp + offset`, just as with any other function call.
- **Return values** are written to the beginning of the callee's frame, following the same layout as Move function returns.

This uniformity means the interpreter does not need a separate dispatch protocol for native calls — the only difference is that the "body" is a Rust function pointer rather than a sequence of Move instructions.

## Error Handling

All errors produced by native functions are transaction-aborting — they terminate execution immediately. Because the entire transaction's execution state (stack, heap, locals) is discarded on abort, there is no need for natives to clean up the stack or restore frame data.

Native functions can produce the following categories of errors:

- **User abort.** The Move-level equivalent of `abort`: the native signals a user-facing failure with an abort code. Gas is charged for the work performed. Example: a deserialization native receiving malformed input.
- **Out of gas.** The native exhausts (or would exhaust) the transaction's gas budget. A partial gas charge may apply for work already done.
- **Invariant violation.** An internal error indicating a broken assumption — e.g., receiving an argument of unexpected type, arity mismatch, or a failed internal assertion. These should never be reachable from valid user code.
- **Limit exceeded.** An execution, memory, or dependency limit is breached. Depending on the specific limit, this may be treated as out-of-gas or as an invariant violation.

Conversely, if a native function returns successfully, it must guarantee that the correct number of return values have been written at the expected offsets in the callee's frame. The interpreter proceeds unconditionally after a successful return — there is no post-call validation of the return layout.

## Gas Metering

- **Constant-cost natives.** Some natives have a fixed cost that is known before execution (e.g., `signer::borrow_address`). These can charge gas upfront before doing any work, and may even be batched with the surrounding basic block's gas charge — though the added complexity may not be justified for the marginal savings.
- **Data-dependent cost.** Many natives have costs proportional to their input size (e.g., vector operations, serialization). These can inspect the input size, compute the charge, and pay upfront before performing the work.
- **Iterative or unpredictable cost.** Some natives have no closed-form formula for their gas cost, or the formula is expensive to compute ahead of time. These must charge gas incrementally — e.g., per iteration within a loop. Charging per iteration/step can still satisfy the charge-before-work rule (see `docs/vm_security_and_correctness.md`, Gas Metering), but there are cases where the cost is most naturally computed after the work is done. Such transient violations should be minimized, bounded by a small constant, carefully documented and reviewed.

## Generics

By default, native functions are **not monomorphized**.

- When the caller (a Move function) is monomorphized, the type arguments to the native callee are fully resolved, but the native itself remains a single generic Rust implementation.
- These resolved type arguments are passed to the native at call time, either as explicit type descriptors or via the execution context.

**Selective native monomorphization.** As an optimization, a small number of performance-critical natives may be monomorphized — i.e., the VM selects a specialized Rust implementation based on the concrete type arguments at load time or call time.

- This is practical only when the set of possible instantiations is bounded and known ahead of time. For example, the crypto algebra natives operate over a fixed set of curve types and can be statically dispatched.
- However, some generic natives (e.g., `bcs::to_bytes<T>`) accept an open set of types and cannot be meaningfully monomorphized. For these, common patterns (e.g., primitive types) may be specialized as fast paths while the generic fallback handles the rest.

**Dynamic dispatch on type arguments.** Some natives perform fundamentally different logic depending on their type arguments (e.g., serialization format selection, type tag construction, layout-dependent operations). These require runtime type inspection regardless of whether monomorphization is used. The native function interface should make this type introspection ergonomic without sacrificing safety — e.g., by providing structured access to type metadata.

## Security Considerations

Native functions are trusted Rust code that bypasses the bytecode verifier. Every native effectively extends the VM's trusted computing base. A bug in a native can violate any of the VM's invariants (see `docs/vm_security_and_correctness.md`) — type safety, memory safety, gas metering, determinism — without any static check catching it. This makes natives a disproportionate source of risk relative to their code volume.

The following concerns are specific to native functions:

- **Memory and type safety.** Natives access the stack and heap directly. An incorrect read or write can silently corrupt execution state.
- **Reference safety.** Borrow semantics for natives are modeled but not verified at runtime. The number of natives that receive or return references should be minimized.
- **Gas metering.** Unlike Move bytecode, natives are responsible for their own gas charges. A native that undercharges is a denial-of-service vector. The asymptotic safety invariant must hold for every native.
- **Determinism.** Natives must not introduce non-determinism. This is especially relevant for natives that call into external libraries (e.g., cryptographic crates), where determinism depends on the library's guarantees.
- **Boundedness.** Any loop, recursion, or allocation within a native must be bounded. External library calls that may internally recurse or allocate unboundedly are especially dangerous. Natives that manage their own shadow memory spaces are particularly hazardous — they bypass global memory limits and make it impossible to account for resource consumption centrally.
- **Panic safety.** A panic inside a native crashes the node. This includes panics from dependencies — e.g., `unwrap`, out-of-bounds indexing, or assertion failures in third-party crates.
- **Calling convention violations.** A native that fails to honor the calling convention — e.g., writing return values at incorrect offsets, returning the wrong number of values, or leaving the frame in an inconsistent state — can corrupt the caller's execution. The interpreter trusts the native unconditionally on successful return.

**Distributed ownership.** Unlike the rest of the VM, not all natives are maintained by the core VM team. For example, cryptographic natives are typically owned by the cryptography team, and other domain-specific natives may be contributed by their respective teams. Yet every native runs inside the VM's trust boundary with full access to execution state. This creates a gap: the teams writing the code may not be deeply familiar with the VM's invariants, and the VM team may not be deeply familiar with the domain logic. Clear documentation of the native interface contract, along with review processes that bridge this gap, are essential.

**Mitigation.** A constrained native interface (akin to the current `SafeNative` layer) can help — e.g., safe memory accessors, structured return value writing. However, because natives are by design low-level and powerful (they are first-class citizens of the trusted computing base), the protection such an interface can provide is inherently limited. Careful auditing remains essential.
