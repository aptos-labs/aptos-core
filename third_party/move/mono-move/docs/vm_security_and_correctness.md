# VM Security & Correctness

## Core Principles

1. **Assume all vulnerabilities will be discovered.** Any exploitable flaw will eventually be found and exploited or reported — increasingly so with the aid of AI-assisted auditing tools.
2. **Security must be integral to the core design.** Retrofitting security guarantees into an existing implementation is far more difficult and error-prone than building them in from day one.

## Common Vulnerability Categories

The following are the primary categories of VM vulnerabilities:

1. **Minting / Data Transmutation / Loss of Funds** — Unauthorized creation, destruction, or corruption of on-chain assets. This is the most severe class.
2. **Non-determinism** — Bugs that result in chain halt.
3. **Crashing** — Unhandled errors that cause the node process to terminate.
4. **Reentrancy** — Unexpected re-entrant calls that result in contract-level bypasses or failures.
5. **Slow-down** — Adversarial inputs that degrade performance.

## Key Invariants

The following invariants must be upheld throughout the VM implementation.

### Arithmetic Safety

All arithmetic operations and numeric conversions must be checked. Unchecked arithmetic (wrapping, overflowing) is not permitted unless correctness can be proven — e.g., when the inputs are already known to be in range due to prior bounds checks or invariants.

### Type and Memory Safety

Values in MonoMove are stored as untyped raw bytes. The VM must prevent type confusion and data transmutation at all times.

**Pointer validity.** Every pointer dereference must satisfy the following:

- The pointer must target a memory region that the transaction is authorized to access — specifically, the transaction's own memory region, or frozen global state shared by a previously committed transaction.
- No use-after-free: pointers to deallocated or recycled memory must never be dereferenced.
- No off-by-one or other incorrect offset computations.
- No out-of-bounds access into containers (vectors, structs, etc.).

**Allocation failure handling.** Stack overflow and heap allocation failure must be treated as errors and handled gracefully (i.e., abort the transaction), never ignored.

**Centralized memory management.** All significant memory allocation must go through the VM's memory manager. Native functions, in particular, should not maintain independent shadow memory spaces, as this makes it difficult to enforce global memory limits and opens the door to untracked resource consumption.

### Gas Metering

Gas must be charged for every unit of work the VM performs.

**Asymptotic safety.** At any point during execution, the cumulative gas charged must be proportional to the cumulative work performed. This is the non-negotiable baseline.

**Charge-before-work.** As a general rule, gas should be charged before the corresponding work is carried out. This is not always feasible — the rule may be transiently violated when necessary — but any such violation must be bounded by a small constant.

### Boundedness

All data structures, algorithms, and resource consumption within the VM must be explicitly bounded. Unboundedness in any dimension (space or time) is a potential denial-of-service vector.

**Data and storage bounds.** The following must all have enforced upper limits:

- Loader data, including code size (accounting for monomorphization expansion).
- Memory consumption: stack, heap, and code regions.
- Type name lengths and data layout sizes.
- Cache sizes (for all caches in the global and per-transaction contexts).
- All items within the binary format (note: existing limits are enforced by the deserializer, but these must be reviewed for completeness).
- Write set size.

**Recursion bounds.** Recursion is particularly important and difficult to guard against. Three distinct sources of unbounded recursion must be addressed:

1. ***Move-level recursion:*** The transaction must be aborted if stack memory is exhausted.
2. ***Rust-level recursion:*** Any Rust algorithm that operates over deeply nested data is vulnerable to stack overflow.
    1. This includes `Drop` implementations on recursive types, which are especially dangerous because the programmer has limited control over when and how they are invoked.
    2. Derived traits (`Clone`, `Hash`, `Eq`, `PartialEq`, `Display`, `Debug`) on recursive types are similarly hazardous — they generate recursive implementations that can overflow the stack on deeply nested data, and can easily go unnoticed in otherwise innocuous code paths.
    3. The goal should be to eliminate recursive algorithms entirely, or at minimum to have a clear, documented mitigation plan before production.
3. ***Recursive library calls:*** Any call into a library function that is internally recursive must also respect depth limits. Examples include topological sort and strongly connected component analysis.

**Algorithm running time.** All algorithms — in the loader, monomorphizer, bytecode verifier, and elsewhere — must have bounded running time relative to their input sizes.

**Value bounds.** The current inclination is that values (including closures) do not require explicit depth or size bounds, provided that no recursive traversals operate over them — i.e., no recursive display, drop, or other recursive algorithms that walk value structures. If any such traversal is unavoidable, the traversal itself must be depth-limited rather than relying on value-level bounds. This decision is to be finalized once the set of operations over values is fully determined.

### No Undocumented Panics

A panic in VM code is equivalent to a crash, which is a vulnerability. The following rules apply:

- Bare `unwrap()` calls are banned.
- Any API that may panic must use `expect()`, `unreachable!()`, or equivalent, with a message documenting why the panic condition is believed to be unreachable.

### Strict Determinism

The VM must produce identical results for identical inputs across all nodes, platforms, and runs.

- **No floating-point arithmetic (IEEE 754).** Cross-platform determinism of floating-point operations is not guaranteed.
- **No OS or thread-local randomness.** The only permissible source of randomness is the block context (e.g., block-level randomness seed).
- **No unordered enumeration.** Hash maps and other data structures with non-deterministic iteration order must never be enumerated unless a deterministic ordering is explicitly guaranteed. Possible mitigation: provide a vetted safe data structure crate for the VM, e.g., a wrapper around `HashMap` that prohibits iteration entirely or requires `unsafe` to enumerate.

### Cache Consistency

All caches must maintain consistency at all times: a stale cache entry that is silently served is a correctness bug, and in the worst case a security vulnerability. The VM maintains multiple layers of cached state — verified modules, struct layouts, interned types, type tags, etc. — and the following are common areas where consistency is at risk:

- **Code upgrades.** Publishing or upgrading a module invalidates all cached data derived from it: the module itself, its definitions, type layouts, and any cross-module dependents.
- **Intra-block visibility.** Under Block-STM, when a transaction publishes a module, correctness must still be guaranteed for all speculatively executed transactions that read that module. The interaction between the global cache, per-block cache, and per-transaction read sets is a major source of complexity in the current VM.
- **Cross-block cache lifecycle.** Caches that persist across blocks must remain valid as conditions change (non-consecutive transaction slices, VM config changes, size limit breaches). Each such condition is a consistency hazard if missed. MonoMove's `MaintenanceGuard` (Section 1 of the main design doc) provides a single enforcement point for this.
- **Derived data coherence.** Caches often store data derived from other cached data (e.g., struct layouts depend on type definitions, which depend on loaded modules). Invalidating one layer without propagating to dependents leads to incoherence.
- **Enum layouts.** Enums deserve special attention: adding a variant is a compatible module upgrade, but it changes the type's layout. Layout caches must therefore be invalidated even when the upgrade itself is valid. In the current VM, this is handled by flushing the entire layout cache on any module publish.
- **Read-set tracking.** Every module or code artifact read during execution must be recorded in the transaction's read set for Block-STM correctness, even if served from a cache. Cache hits must be indistinguishable from storage reads in this regard.

### Reference Aliasing

Move's linear type system prohibits aliasing of mutable references. This invariant must be preserved at runtime. Violations can directly lead to loss of funds (e.g., double-spending via aliased mutable access to a coin resource).

### Safe Formatting and Display

Formatting values or internal data structures (e.g., for error messages or logging) is a hazard. Values and data structures can be very large, or worse, deeply nested — leading to excessive memory allocation, stack overflow, or stalling.

- Error messages should not include full dumps of values or complex internal state.
- `Clone` on values carries the same risks and should be treated with equal caution.
- Consider disallowing `#[derive(...)]` on certain VM internal types to prevent accidentally introducing recursive or expensive trait implementations.

### Transaction Argument Validation

Transaction arguments must not be exempt from validation. All inputs — including arguments supplied in the transaction payload — must pass through the same safety checks as any other data entering the VM.

TBA: Closure-specific things
