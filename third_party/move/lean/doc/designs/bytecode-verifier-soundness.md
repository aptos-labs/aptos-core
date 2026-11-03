# Proving the Move bytecode verifier sound in Lean 4

Status: revision 2 (2026-10-01). Phase 0 is done except human sign-off of the
trusted spec. Phase 1 is done and Phase 2a (data by value) nearly so in
[`bytecode-verifier/`](../../bytecode-verifier/): D1, C1–C2, K1–K2, N2 and
the local properties but one clause of L6 are proved for the model on the
supported subset (2026-10-02). Phase 2b's D2 and D3 are proved for
transactions on the same subset, pending sign-off of their statements
(2026-10-02). Phase 3 is under way (2026-10-04): the semantics, ghost layer
and D4 statements for references and global storage are in the spec, pending
sign-off; C1–C2 are statically consistent on the extended subset, D1 is
proved for programs without references and, with references, for programs
whose function handles are no struct-API mutable borrows: references cross
calls in both directions, entries take `&signer`, and code borrows global
storage (milestone M3, 2026-10-05; M2, no global borrows, and M1, functions
taking and returning values, are corollaries); D2 and D3 are proved over the
extended ghost layer. D4a follows from D4b′, D4c and D4e for any program
(`NoDanglingFromContainment`), and D4b′ is proved for the M3 programs
(2026-10-05). D1 for the struct-API borrows, and the rest of D4, are next. That package's `README.md` tracks what exists. Revision 2 incorporates a review of the first draft; appendix A lists
what changed and why.

## 1. Summary

The Move bytecode verifier (`third_party/move/move-bytecode-verifier`) decides
which modules may be published and loaded. Every Move VM, the legacy VM and
MonoVM alike, relies on its decision to skip checks at runtime. This plan
proves in Lean 4 what the verifier guarantees about Move bytecode, and either
proves that the verifier provides it or finds a concrete counterexample.

Four decisions shape the plan:

1. **The guarantees are independent of any VM implementation.** They are
   stated over the bytecode and a reference semantics of the bytecode
   language. Some guarantees are *joint*: the static verifier deliberately
   relies on a few dynamic checks that the language itself defines, such as
   the resource lock on closure reentrancy and the variant check when
   unpacking an enum. Those checks belong to the reference semantics, are
   listed in a catalogue (section 6), and every VM must implement them. No VM
   data structure or VM-specific check appears in a trusted statement.
2. **The specification is written from first principles.** The semantics and
   the property statements come from the Move language definition, not from
   the verifier's code, so they cannot inherit its bugs.
3. **The verifier is modeled faithfully, bugs included.** The Lean model of
   the verifier mirrors the Rust passes exactly. A bug in the Rust verifier
   then appears as a soundness proof that cannot be completed.
4. **Every claim carries its premises.** Bytecode versions, the exact
   configuration, whether module dependencies are acyclic, and validation
   done by the transaction layer are explicit hypotheses wherever a guarantee
   needs them.

## 2. Goals and non-goals

Goals:

- A precise, reviewable, VM-independent statement of what verified bytecode
  guarantees, split by what establishes each fact: the module alone, linking,
  transaction validation, or execution.
- A catalogue of the dynamic checks the language requires of every VM,
  because the static guarantees depend on them.
- Machine-checked proofs that a faithful model of the Rust verifier implies
  those statements, under the recorded production configuration.
- Bugs found in the Rust verifier, with concrete counterexamples, both during
  proof development and through differential fuzzing before the proofs exist.
- A mapping from every verifier guarantee that a VM relies on to a theorem,
  with gaps reported.

Non-goals:

- Verifying any VM. A VM's refinement of the reference semantics, including
  its implementation of the dynamic-check catalogue, is a separate, per-VM
  effort.
- Gas amounts, metering, and denial-of-service bounds, except bounds a
  consumer relies on for safety (L11, R7).
- The Move compiler, the Move Prover, or Leaner source verification.
- Proving the Rust source itself in the first phases (see A2 in section 11).

## 3. Architecture

```text
 TRUSTED (first principles; small; reviewed by spec owners)
 ┌──────────────────────────────────────────────────────────────┐
 │  Syntax            abstract syntax of CompiledModule/Script   │
 │  Semantics         reference semantics, incl. the dynamic-    │
 │                    check catalogue R1–R7 (section 6)          │
 │  Properties        L, K, E, C, D, W statements (section 7)    │
 │  Native contracts  hypotheses about native functions          │
 └──────────────────────────────────────────────────────────────┘
                 ▲ proved against (Lean kernel)
 UNTRUSTED (checked by the kernel; may contain anything)
 ┌──────────────────────────────────────────────────────────────┐
 │  Verifier model    faithful transcription of the Rust passes  │
 │  Proof layer       abstract-interpretation framework, per-pass│
 │                    abstractions and concretizations, lemmas   │
 └──────────────────────────────────────────────────────────────┘
                 ▲ connected by separate deliverables (section 11)
 ┌──────────────────────────────────────────────────────────────┐
 │  Rust verifier ⊆ model acceptance        (A2)                  │
 │  production config = Lean config         (A3)                  │
 │  production decoder ≈ Lean import        (A4)                  │
 │  VMs ⇄ reference semantics               spec validation only  │
 └──────────────────────────────────────────────────────────────┘
```

The trusted base is the only place where a mistake can turn into a false
guarantee. Everything else is either checked by the Lean kernel or connected
to the Rust code by an explicitly tracked deliverable.

## 4. Scope and Phase 0 decisions

### 4.1 Configuration

`aptos_prod_verifier_config`
(`aptos-move/aptos-vm-environment/src/prod_configs.rs`) is computed from the
gas feature version, on-chain feature flags and timed features. The plan
records the exact inputs and the resulting `VerifierConfig` at a pinned commit,
and Lean uses that value (deliverable A3). Its fields fall into three classes
that are treated differently:

| Class | Examples | Treatment |
|---|---|---|
| Correctness switches | `sig_checker_v2_fix_function_signatures`, `sig_checker_v2_fix_script_ty_param_count`, `check_function_type_abilities`, `enable_enum_types`, `enable_function_values`, `enable_resource_access_control` (permanently off; access specifiers are rejected) | Fixed to their production values. Turning a switch off changes what is checked, so this is not a "reject-only" choice: each setting is its own theorem instance, and settings not proved are out of scope. |
| Acceptance limits | loop depth, basic blocks, value-stack size, type nodes, push size, struct, field, variant and function counts, meter units | Modeled only as reasons to reject. Soundness does not depend on their values, except the bounds consumers rely on (L11). |
| Runtime bounds | layout depth, value nesting depth, call depth, gas | Not verifier configuration. They appear in the semantics as `limit` outcomes and in obligation R7. |

Modules are re-verified on load with the current configuration
(`move-vm/runtime/src/storage/environment.rs`, cached by module hash and
config hash), so only the current configuration matters for current
execution. Historical configurations matter only for replaying history, which
is out of scope.

### 4.2 Bytecode versions

The first theorems cover `VERSION_10` modules. A production whole-program
claim must cover every module in the program, and a version-10 module may
depend on older modules. Those modules are re-verified through
version-specific paths (`control_flow_v5`, legacy script-signature rules
below version 5). Mixed-version coverage (A5) is therefore required before the
whole-program claim in Phase 5, not left as optional work.

### 4.3 Dependency cycles

With lazy loading, which is in the default-enabled feature list
(`types/src/on_chain_config/aptos_features.rs`), publishing runs local
verification and link checks against immediate dependencies only. It
explicitly does not check for cycles (`move-vm/runtime/src/storage/publishing.rs`).
Cyclic dependencies can be published. For example, the
`move-vm/transactional-tests/tests/lazy_loading/cyclic_structs.masm` fixture
publishes mutually recursive structs through an upgrade, and execution then
fails with the layout-depth limit (`VM_MAX_VALUE_DEPTH_REACHED`).

Consequences:

- Global non-recursion of structs and global finite monomorphization do not
  follow from verification. `struct_defs` and `instantiation_loops` check one
  module at a time; `instantiation_loops` skips external callees because it
  assumes there is no mutual recursion across modules.
- Static calls can re-enter a module, so the static `acquires` discipline no
  longer covers all reentrancy. The runtime resource lock (R3) covers it.

Options:

- **(a) Restrict to acyclic programs.** Every theorem takes `Acyclic P`. This
  is simpler, but it does not cover what production accepts.
- **(b) Cover cyclic programs.** The reference semantics handles recursive
  types naturally (values are finite trees) and needs no layouts; layout
  construction and specialization are bounded by `limit` outcomes (R7).
  Global non-recursion and finite monomorphization (K3, K4) are stated only
  under `Acyclic P`.

Decision (Phase 0, 2026-10-02): (b). Under either option, a consumer that
needs finite layouts or finite monomorphization (MonoVM's lowering) must
enforce R7 itself, because the verifier does not guarantee it in production.

### 4.4 Supported subset and exclusions

Theorems take a `Supported P` hypothesis naming the instructions and features
covered so far; each phase widens it. Out of scope: gas amounts, and the
binary encoding until deliverable A4. Resource access control is out of
scope because the production verifier rejects access specifiers.

## 5. The trusted specification

### 5.1 Syntax

Lean inductive types for `CompiledModule` and `CompiledScript`. These cover:

- module, struct, function and field handles;
- instantiation tables, signatures, and the constant, identifier and address
  pools;
- struct and enum definitions, function definitions with attributes (for
  example `persistent`, `module_lock`), and friend declarations;
- the full `file_format::Bytecode` instruction set.

Taking the syntax from `file_format.rs` is acceptable: it describes the object
being checked, not a judgment about it.

### 5.2 Reference semantics

The semantics of linked Move bytecode that every VM must implement on
verified code. It is *defensive*: every violation that a guarantee rules out
must be representable and detectable in it, because a semantics that cannot
express a violation makes the matching theorem vacuous.

**Structure.** A deterministic core step function, plus two explicit sources
of nondeterminism:

- a **stopping relation**: any state may stop with `limit` (gas, call depth,
  value depth, layout size, memory). Any VM's limit policy is therefore a
  refinement;
- a **native relation** given by the native contracts.

The core is a Lean function, so the semantics is executable.

**State.** The state consists of:

- a call stack of frames, each holding a function, its type arguments, a pc,
  locals, and its own operand-stack segment;
- global storage, mapping (address, closed struct type) to a value;
- the set of active modules used by the reentrancy rules (R3, R4);
- a ghost event trace.

Each local slot is either `available v` or `unavailable` (never assigned, or
moved out). The segmented operand stack means a callee cannot see or pop its
caller's values; popping below a segment is `wrong`. A VM with one shared
stack refines this on verified code.

**Values and typing.** Integers tagged with width and signedness, bool,
address, signer, vectors, struct and variant values, closures, and
references. Every value has an intrinsic runtime type. A value *conforms* to a
declared type through an assignability relation written from the language
definition:

- a function value with more abilities may be used where a function type with
  fewer abilities is expected;
- an immutable reference conforms covariantly in its target;
- otherwise types must be equal.

The spec defines this relation independently. If it differs from the Rust
`SignatureToken::is_assignable_from`, that difference is a finding.

**Recursive types.** Allowed. Values are finite trees, so the semantics needs
no layouts and handles cyclic programs (4.3).

**References.** A reference is a location plus a mutability flag and a ghost
provenance (a unique id and the id of the reference it was derived from).

- A location is a root plus a path of field, variant-field or vector-index
  steps.
- The root is a local slot of a specific frame instance, or a global
  (address, type).
- Locations can become invalid: the local is moved or its frame returns, a
  vector element is popped or swapped away, the resource is moved out, or an
  enum variant changes.
- Using an invalid location is `wrong`.

**Calls and visibility.** Direct `Call`/`CallGeneric` and
`PackClosure`/`PackClosureGeneric` must respect visibility: same module,
declared friend, or public, plus the persistence rules for storable
closures. `CallClosure` invokes whatever function the closure targets,
including a private one; a module handing out a closure over its private
function is intended.

**Outcomes.** The classification below is complete; every step result falls
into exactly one class.

| Outcome | Meaning |
|---|---|
| `running s` | normal step |
| `returned vs` | top-level return; payload effects commit |
| `failed f` | a language-defined failure: a Move `Abort`/`AbortMsg`, or one of the checks R1–R6 (section 6). Payload effects roll back. |
| `limit k` | resource exhaustion through the stopping relation, or a bound from R7. Payload effects roll back. |
| `wrong r` | the bytecode broke the language's rules: stack underflow; type non-conformance; reading an unavailable local; an invalid jump or table index; falling off the end of a function; using an invalid or wrong-mutability reference; copying a value whose type lacks `copy`; discarding one that lacks `drop`; storing one that lacks `store` or `key`; struct or global access outside the defining module; a direct call or closure creation that violates visibility; a closure call that doesn't match its signature. |

Only `wrong` is bad. `failed` and `limit` are legitimate outcomes of verified
code, and both roll back the transaction payload's effects on global storage.
The conservation property (D2c) and differential write-set comparisons rely on
this rollback rule. Gas charging and the epilogue are outside the model.

**Natives.** Natives are not given an implementation. Each theorem takes a
hypothesis `NativesRespect contracts`. It says each native returns values
conforming to its declared types and respects abilities, encapsulation,
references and the reentrancy rules, apart from explicitly listed exceptions.
The contracts must be satisfiable (N3).

**Event trace.** The semantics records ghost events: reference creation, use
and derivation, local and global reads and writes, value creation and
consumption with ghost identities, moves, calls and closure dispatch. The
trace-based properties in D2 and D4 are stated over it, so the spec contains
no borrow graph or static liveness algorithm.

**Sources.** The Move language reference, the AIPs that define closure and
dispatch reentrancy, the bytecode format documentation, and the papers in
section 17. The semantics authors do not read `move-bytecode-verifier/src`.

**Validation.** Validation checks the spec; it does not make the guarantees
depend on any VM.

- *Positive:* run the semantics on corpora of verified programs side by side
  with the legacy VM and MonoVM. Outcome classes, return values, and the
  write sets of `returned` executions must agree, ignoring gas. A divergence
  is triaged as a semantics bug or a VM bug.
- *Negative:* known-bad programs reach `wrong`. Programs that rely on a
  dynamic check reach the expected `failed` outcome.

## 6. Dynamic checks required by the language (R catalogue)

The static verifier does not establish these facts and relies on them being
checked at runtime. They are part of the language definition, not a choice
made by one VM, so they belong to the reference semantics. Every VM must
perform them or prove them redundant. Every theorem lists the R-checks it
depends on, and this catalogue is the list of runtime checks MonoVM must not
skip.

| ID | Check | Why the verifier cannot do it statically |
|---|---|---|
| R1 | Unpacking an enum, or borrowing a variant field, requires the runtime variant to match; otherwise `failed` (`STRUCT_VARIANT_MISMATCH`). | The variant is a runtime value. |
| R2 | `VecUnpack(n)` requires the vector length to equal `n`; otherwise `failed` (`VECTOR_OPERATION_ERROR`). | The length is a runtime value. |
| R3 | **Resource lock on reentrancy.** Each module has an active count, incremented by a cross-module call, by every closure dispatch (even within one module), and by top-level entry. A global operation on a type of module `M` fails when `M`'s count exceeds 1 (`RUNTIME_DISPATCH_ERROR`). | `CallClosure` has no static `acquires` set. The reference-safety pass checks acquired resources for static calls only (`call` versus `call_closure` in `reference_safety/abstract_state.rs`). Cyclic dependencies (4.3) also allow static reentry. |
| R4 | **Module lock.** Inside native dynamic dispatch or a `#[module_lock]` function, re-entering any active module fails. | Dispatch targets are dynamic. |
| R5 | Global existence: `MoveFrom` and `BorrowGlobal` require the resource to exist; `MoveTo` requires it to be absent. | Storage contents are dynamic. |
| R6 | Arithmetic overflow, division by zero, out-of-range casts and shifts, and out-of-range vector indices fail. | Values are dynamic. |
| R7 | Layout construction, value nesting and type specialization are bounded and stop with `limit` instead of misbehaving. This is an obligation on every VM; the semantics permits `limit` everywhere. | Cyclic dependencies (4.3) admit recursive types and growing instantiations, and type-size limits apply only to syntactic tokens (L11). |

The current VM implements R3 and R4 in `move-vm/runtime/src/reentrancy_checker.rs`.
The regression fixture
`move-compiler-v2/transactional-tests/tests/no-v1-comparison/closures/reentrancy_local.move`
shows R3 at work: the function holds `&mut R`, calls a closure that accesses
`R`, and the closure fails with `RUNTIME_DISPATCH_ERROR`. Publication
succeeds.

## 7. Property catalogue

Properties are grouped by what establishes them. Identifiers are stable so
that VM documentation, tests and the divergence ledger can cite them.

### L: Local facts (from `verifyModule` on one module)

| ID | Property |
|---|---|
| L1 | **Local index integrity.** Every index into the module's own tables is in range, as are local and type-parameter indices in code. Whether imported handles resolve is K1. |
| L2 | **Uniqueness.** Identifiers, handles and definitions are pairwise distinct, so an index names one entity. |
| L3 | **Control flow.** Code is non-empty; every branch target is in range; no path falls off the end; the CFG is reducible; loop depth is within the configured bound. |
| L4 | **Stack discipline.** There is a unique height function `h : pc → ℕ` with `h(entry) = 0`, consistent along every edge. No instruction pops more than `h(pc)`; `h = 0` at every block boundary; `Ret` executes with exactly the declared return count; `h ≤ max_value_stack_size`. This holds for every block, reachable or not. |
| L5 | **Type formation**, by recursive rules. Reference types may appear only at the top level of parameter, return and local types, and at the immediate parameter and result positions of function types, at any nesting depth of function types. They are never vector elements, type arguments, or field types except inside a function type's signature. Type-constructor arity, type-parameter indices and instantiation ability constraints are respected; function types carry no `key` ability. |
| L6 | **Local struct facts.** Declared abilities are justified by the fields' abilities. Containment among the module's own struct definitions is acyclic. Variant and field tables are consistent. |
| L7 | **Local instantiation.** There is no type-growing cycle among the module's own generic functions, through calls or closure creation. |
| L8 | **Constants.** Every constant's bytes decode to a value of its declared type, which uses only primitive and vector types. |
| L9 | **Instruction consistency.** Generic and non-generic instruction forms match their operands. `VecPack`/`VecUnpack` counts and struct-API functions are well-formed. |
| L10 | **Friend declarations** are well-formed (same address, not the module itself). |
| L11 | **Syntactic bounds**, each with its measured object: counts of struct definitions, fields, variants, function definitions, basic blocks, parameters, return values and loop depth; operand-stack height; and the weighted size and depth of *syntactic* type tokens (structs and type parameters weigh more because they may expand, `limits.rs`). No bound after substitution or layout expansion is guaranteed statically; that is R7. |

### K: Linked-program facts

Premise: every module passes local verification, and link checks against its
immediate dependencies (`dependencies::verify_module`) succeed for the fixed
module set.

| ID | Property |
|---|---|
| K1 | **Handle resolution.** Every imported struct or function handle resolves to a definition with the same name and signature, whose type-parameter constraints the handle's include. A struct handle is a conservative view of its definition: no ability the definition lacks, the same number of type parameters, phantom only where the definition's is, and at least the definition's constraints (`StructHandle.ConservativeFor`; corrected from equality by triage, 2026-10-02). |
| K2 | **Visibility.** Every direct call and every closure creation targets a function visible from the calling module, and only storable functions (public or `persistent`) are packed into storable closures. Invoking an already obtained closure is not subject to visibility. |
| K3 | **Global struct well-foundedness.** Struct containment across the program is well-founded, so every closed type has a finite layout. Holds only under `Acyclic P`. |
| K4 | **Global finite monomorphization.** For every function and closed type arguments, the set of reachable (function, type arguments) instances is finite. Holds only under `Acyclic P`; whether closures invoked across modules can break it even then is open question 4. |
| K5 | **Friends.** Every declared friend exists, and friend-only functions are called only from friends. |

### E: Transaction-entry premises (not verifier facts)

| ID | Premise |
|---|---|
| E1 | Arguments to entry functions and scripts are validated by the transaction layer (`aptos-move/aptos-vm/src/verifier/transaction_arg_validation.rs`). The verifier contributes no entry-signature rules for modern bytecode: it passes `no_additional_script_signature_checks`, and the legacy rules in `script_signature.rs` apply only below version 5. |
| E2 | Signers in the initial state come only from transaction authentication. |

### C: Static certificates

Facts that a consumer can compute once and rely on in every execution.

| ID | Property |
|---|---|
| C1 | **Static types.** There is a map from (function, pc) to the types of the frame's own operand-stack segment and the declared local types, expressed over the function's type parameters. In every reachable state at that pc, each value in the segment conforms (with subsumption) to its mapped type under the frame's type arguments, and each *available* local conforms to its declared type. |
| C2 | **Static local availability.** There is a map from (function, pc, local) to `available`, `unavailable` or `maybe`, valid at every pc reachable from the entry. Every reachable state at that pc agrees with it, and no instruction reads a `maybe` or `unavailable` local. |

`locals_safety` and `reference_safety` run on the abstract interpreter, which
visits only blocks reachable from the entry. `stack_usage_verifier` and
`type_safety` check every block. C2 and the reference-safety facts therefore
say nothing about unreachable code. A consumer that translates unreachable
blocks must not assume them (section 10.4).

### D: Dynamic safety (over the reference semantics, including R1–R7)

Premises: `WellFormed V cfg P` (K's premise), `Supported P`, E1–E2, native
contracts, and a well-typed initial state (global values conform to their
declared types).

| ID | Property |
|---|---|
| D1 | **No wrong state.** No reachable state is `wrong`. Every execution runs forever, returns values conforming to the declared return types, fails through `failed`, or stops with `limit`. |
| D2 | **Ownership accounting** (definitions below). **D2a uniqueness:** for types without `copy`, each identity has at most one occurrence at any time. **D2b no implicit destruction:** for types without `drop`, an occurrence disappears only through a legitimate transfer or consumption. **D2c conservation:** after `returned`, every identity of a type without `drop` that existed initially or was created is in global storage, in the return values, or was consumed by a legitimate consumption. A type may have `copy` without `drop`: D2a does not apply to it, but D2b and D2c do. |
| D3 | **Bytecode-level encapsulation and unforgeability.** **D3a:** every `Pack`, `Unpack`, field or variant borrow, `TestVariant`, `MoveTo`, `MoveFrom`, `BorrowGlobal` and `Exists` at struct type `T` executes code of `T`'s defining module, under every instantiation. **D3b:** no instruction creates a `signer`; signers come only from E2 and native contracts. D3 does not restrict copying, dropping or comparing values of `T` in other modules when `T`'s abilities allow it. |
| D4 | **Reference safety** (definitions below). **D4a no dangling:** every use of a reference targets a valid location. **D4b′ containment:** for every `&mut` reference `r` to `ℓ`, from its creation to the last use of `r` itself, no instruction on a local or a resource reads or writes a location overlapping `ℓ`, and no read or write through any reference is of a location strictly enclosing `ℓ`; reads and writes at or inside `ℓ` through other references, and inspections, are allowed. **D4b″ exclusivity at calls:** for every frame and every `&mut` reference parameter `p` of it, while the frame is active, any access to a location overlapping one on which `p` is live is a use of `p` or of a reference derived from `p`. (D4b′ and D4b″ replace full mutable exclusivity, which the language does not guarantee among references a function borrows itself: aliasing at or below a target cannot invalidate it, whole values are not read or written while a part is lent out through a live `&mut`, and a function may assume its `&mut` parameters exclusive for the whole call; `MutableContained`, `ParametersExclusive`; spec-owner decision, 2026-10-05.) **D4c immutable stability:** while an `&` reference to `ℓ` is live, no step writes a location overlapping `ℓ`. **D4d no escape:** no reference is stored in a field, vector element, closure capture or global storage, and a returned reference derives from a reference parameter. **D4e dead slots are unborrowed:** when a local becomes `unavailable` or its frame returns, no live reference points into it. The cases involving global resources depend on R3 and R4 for code reached through closure dispatch or reentrant calls. |
| D5 | **Function values.** Captured values are never references. A closure's `copy`, `drop` and `store` abilities are justified by its target and every captured value. `CallClosure` supplies exactly the remaining parameters, conforming to their types, and receives conforming results. Only closures over storable functions reach global storage. |

Definitions used by D2:

- **Identity.** Each value created during execution gets a ghost identity:
  created by `Pack`, `PackVariant`, `VecPack`, `PackClosure`, loading a
  constant, or a native.
- **Occurrence.** A position holding an identity: a local, an operand-stack
  slot, a field, a vector element, a closure capture, or global storage.
  Nested occurrences count.
- **Legitimate transfer.** A move (`MoveLoc`, `StLoc`, argument passing,
  return), packing into a container, vector push, pop and swap, `MoveTo` or
  `MoveFrom`, or a native transfer allowed by its contract.
- **Legitimate consumption.**
  - `Unpack`/`UnpackVariant` in the defining module, which transfers the
    fields;
  - `VecUnpack`, which consumes the vector and transfers its elements;
  - `CallClosure` on a moved closure, which consumes it and transfers the
    captured values to the callee's parameters;
  - `Pop`, `Eq`/`Neq`, or overwriting, but only for types with `drop`.
- **Rollback.** On `failed` or `limit`, the payload rolls back, so D2b and
  D2c are judged only on `returned` executions and on the states before
  termination.

Definitions used by D4:

- **Use.** `ReadRef`, `WriteRef`, `FreezeRef`, deriving a field, element or
  variant borrow, copying or moving the reference, passing it as an argument,
  returning it, or comparing it.
- **Derivation** is transitive. `r'` derives from `r` if `r'` was borrowed
  through `r` or through a reference derived from `r`, or was returned by a
  call that received `r` and whose result provenance includes `r`.
- **Liveness.** `r` is live from its creation to the last use of `r` or of
  any reference derived from it; a live child keeps its parent's borrow
  alive (D4b″, D4c). D4b′ asks less: each `&mut` reference constrains its
  own location only, until the last use of the reference itself (copies of
  a reference are that reference). A `&mut` derived from `r` constrains its
  own location in turn; an `&` derived from `r` is D4c's.
- **Overlap.** A read or write of `ℓ'` overlaps a reference to `ℓ` when they
  have the same root and one path extends the other; an inspection of a
  node (a variant, a length) overlaps it only at or inside `ℓ`. `ℓ'`
  strictly encloses `ℓ` when `ℓ` is strictly inside the value at `ℓ'`.
- **Active frame.** A frame is active from its entry to its return, while
  its callees run too.
- D4b′, D4b″ and D4c constrain every step in a live interval, not only the
  steps between creation and the next use.
- **D4b′, D4c and D4e imply D4a.** A reference becomes invalid only through
  a write strictly above its target, the removal of its root, or the end of
  its frame. Each is excluded before the reference's last use: a write
  strictly above a `&mut` by D4b′, a write overlapping an `&` by D4c, a
  removal (a direct write) by D4b′ or D4c and for a local also by D4e, a
  frame end by D4e (`NoDanglingFromContainment`). Full exclusivity implies
  D4b′, since an access through a reference derived from `r` is at or
  inside `r`'s location; D4b′ is the part of it this argument uses.

### W: Whole-program and open-world properties

| ID | Property |
|---|---|
| W1 | **Modularity.** Local verification and link checks imply L, K1, K2, K5, C and D for the linked program. K3 and K4 also need `Acyclic P`; link checks do not establish acyclicity. |
| W2a | **Robust encapsulation.** D3 holds for a module `M` linked with any set of other modules that pass verification (attacker code). |
| W2b | **Invariant preservation**, a reasoning principle with explicit premises, not a property of the verifier alone. A predicate `Inv` over `M`'s state holds at every observation boundary, meaning whenever control is outside `M`, provided that: (i) `Inv` holds initially; (ii) every function of `M` callable from outside (public, friend, entry, or the target of a closure that escapes `M`) preserves `Inv` from boundary to boundary; (iii) no mutable reference into `Inv`-relevant state escapes `M`, established by an encapsulator such as escape analysis, as in *Robust Safety for Move*; (iv) `M` invokes no unknown code (closures, dynamic dispatch) while `Inv` is temporarily broken, unless R3 or R4 rule out the reentrant access; (v) `Inv` is insensitive to what other modules may legitimately do with `M`'s types, namely copying and dropping values whose abilities allow it, equality comparisons, and moving values. |
| W3 | **Upgrade safety.** Premises: (i) the old program satisfies W1; (ii) the replacement module passes local verification and link checks against its new dependencies; (iii) `compatibility::check` passes under a pinned policy (struct and public-function linking, struct layout, friend linking); (iv) the upgrade takes effect between executions, so no frame of the old code is live (to be confirmed against the publishing flow). Conclusions: modules that depended on the old version still satisfy K1 and K2; global storage that conformed to the old definitions conforms to the new ones, including persisted closures (target function still present, persistent and signature-compatible; type arguments, capture mask and captured values still conform); and D holds for executions starting from storage written before the upgrade. |
| W4 | **Entry points.** With E1 and E2, D holds for transactions that invoke entry functions and scripts. |

### N: Non-vacuity

Without these, a theorem could hold for empty reasons.

| ID | Property |
|---|---|
| N1 | **Progress of the semantics.** Every state is terminal or has a next step (core, native, or stopping). A silently stuck state would make D1 hold vacuously. |
| N2 | **Admissible initial states exist** for every entry point. |
| N3 | **Native contracts are satisfiable.** A simple reference implementation in Lean meets every contract, so the hypothesis is not contradictory. |
| N4 | **`WellFormed` is non-trivial.** The model accepts the Aptos framework and the standard library. |

## 8. Main theorems

In Lean-like notation, over the verifier model `V` and the recorded
configuration `cfg`:

```lean
-- Local facts from one module.
theorem verify_local (hver : SupportedVersion m)
    (h : V.verifyModule cfg m = .ok) : L1 m ∧ … ∧ L11 cfg m

-- Linked program; acyclicity only where needed.
theorem verify_linked  (hP : WellFormed V cfg P) : K1 P ∧ K2 P ∧ K5 P
theorem verify_acyclic (hP : WellFormed V cfg P) (hA : Acyclic P) : K3 P ∧ K4 P

-- Certificates and safety. `Trace` includes R1–R7, the stopping relation
-- and the native relation.
theorem verify_certificates (hP : WellFormed V cfg P) (hS : Supported P) :
    ∃ types avail, C1 P types ∧ C2 P avail

theorem verify_sound (hP : WellFormed V cfg P) (hS : Supported P)
    (hN : NativesRespect P contracts) (hE : EntryPremises P s₀)
    (h₀ : AdmissibleInitial P s₀) :
    ∀ t, Trace P s₀ t → NoWrong t ∧ D2 t ∧ D3 P t ∧ D4 t ∧ D5 t

theorem verify_robust_encapsulation : …   -- W2a
theorem invariant_preservation      : …   -- W2b, premises (i)–(v)
theorem verify_upgrade              : …   -- W3, premises (i)–(iv)
theorem semantics_progress          : …   -- N1
```

`WellFormed V cfg P` holds when every module of `P` passes `V.verifyModule`
and `V.verifyLinking` against its dependencies. Its definition comes from the
model; its meaning comes only from these theorems.

## 9. The untrusted verifier model and the proof structure

### 9.1 Verifier model

- One Lean file per Rust pass, in the order `verifier.rs` runs them,
  including the orchestration itself. Each file uses the same algorithm,
  abstract domains and accept or reject decisions as the Rust pass, and its
  header records the Rust path and content hash it transcribes.
- **Bugs are transcribed too.** A modeler who suspects a bug records it in the
  divergence ledger and keeps the model faithful (section 10).
- Correctness switches are fixed to production values; acceptance limits and
  the meter are reasons to reject (4.1).
- Rust panic sites (indexing, `unwrap`) are explicit `panic` results in the
  model. Rust converts panics to `VERIFIER_INVARIANT_VIOLATION` through
  `catch_unwind`, which is a rejection, so this does not affect soundness. An
  optional totality theorem shows no panic is reachable.
- The model is executable and runs in the differential harness (10.2).

### 9.2 Abstract-interpretation framework

`absint.rs` is modeled once and proved once. Its obligations go beyond
transfer and join soundness:

- **Initial state:** the entry pre-state covers every admissible concrete
  entry state: parameters available and conforming, other locals unavailable,
  reference parameters as distinct roots.
- **Transfer:** each instruction's transfer function is sound for the pass's
  concretization `γ`.
- **Join:** the join over-approximates both inputs, and `JoinResult::Unchanged`
  is reported only when the old pre-state already covers the new post-state.
  An incorrect `Unchanged` stops the analysis early.
- **Canonicalization:** renaming reference ids before a join (reference
  safety's canonical states) preserves `γ`.
- **Scheduling:** the scheduler visits blocks in reverse post-order and
  revisits a loop head only when a back-edge join changed it. The theorem: on
  success, every block reachable from the entry has a pre-state covering all
  incoming post-states. This depends on the depth-first back-edge
  classification and on reducibility (L3).
- **Reachability:** unreachable blocks are never analyzed, so the conclusions
  are about reachable states only (see C2).

This follows Klein and Nipkow's verified JVM bytecode verifier.

### 9.3 Per-pass obligations

| Rust pass | Abstraction or algorithm | Main proof obligation | Properties |
|---|---|---|---|
| `check_bounds` (binary format) | table scans | direct | L1 |
| `check_duplication` | table scans | direct | L2 |
| `features`, `limits`, `meter` | counters | reject-only; syntactic bounds with measured objects | L11 |
| `signature_v2` | recursive formation and ability checking | ability constraints preserved by substitution; nested function-type rules | L5, L6 (abilities) |
| `instruction_consistency` | per-instruction checks | direct | L9 |
| `constants` | decode constants | decoding agrees with the semantic value encoding | L8 |
| `friends` | declaration checks | direct | L10 |
| `struct_defs` | cycle detection within the module | local acyclicity; global only with `Acyclic P` | L6, K3 |
| `instantiation_loops` | type-growing cycles in the module's instantiation graph | local finiteness; global only with `Acyclic P` | L7, K4 |
| `control_flow` | CFG, reducibility, loop depth | CFG agrees with the semantics' successor relation | L3 |
| `stack_usage_verifier` | per-block heights, all blocks | yields `h` | L4 |
| `type_safety` | per-block abstract stack of types, all blocks; assignability | conformance with subsumption is preserved | C1, D1, D2 (abilities), D3a, D5 |
| `locals_safety` | absint over availability, reachable blocks | slot state ∈ γ(availability) | C2, D1, D2b |
| `reference_safety` | absint over a borrow graph, reachable blocks | γ from borrow graphs to concrete reference configurations; per-instruction soundness; with R3/R4 for closure dispatch and reentry | D4a–e |
| `acquires_list_verifier` | acquires consistency | whether D4 needs it is decided by the proof | D4 (globals) |
| `struct_api_checker` | attribute checks | struct-API functions preserve encapsulation | D3a |
| `script_signature` | legacy rules below version 5 only | direct | E1 (legacy) |
| `dependencies` (link time) | interface agreement with immediate dependencies | local facts combine into K | K1, K2, K5 |
| `compatibility` (upgrade time) | old and new interfaces and layouts | together with verification of the replacement, preserves linking and storage conformance | W3 |

Assigning passes to properties is a working hypothesis; the proofs determine
the real dependencies. Later Rust passes assume that earlier ones succeeded,
and the proofs make those assumptions explicit hypotheses. A pass that
silently depends on an invariant no earlier pass establishes is itself a
finding.

### 9.4 Assumption hygiene

- `#print axioms` on every main theorem lists only Lean's standard axioms.
  Native contracts, entry premises and `Acyclic P` are hypotheses in theorem
  statements.
- No `sorry` in `Proofs/` on the main branch, apart from a tracked, shrinking
  list during development, which CI reports.

## 10. Bug-finding methodology

### 10.1 Triage

A disagreement between the model, the semantics and the Rust verifier has
four possible explanations, and triage must choose among them explicitly:

| Classification | Meaning | Action |
|---|---|---|
| Verifier defect | The Rust verifier accepts bytecode that violates the language contract. | Disclosure process (10.6); fix Rust, then the model. |
| Transcription defect | The model differs from the Rust code. | Fix the model; record it in the ledger. |
| Specification defect | The semantics, a property, or the R catalogue is wrong or too strong (as several first-draft properties were). | Correct the spec under the change rules (10.5). |
| Missing proof argument | The property holds, but the proof needs a stronger invariant or lemma. | Strengthen the proof layer. |

Procedure:

1. Construct a candidate counterexample: a module the model accepts and an
   execution that reaches `wrong` or violates a property.
2. Run it in the executable semantics.
3. Run it through the Rust verifier.
   - If Rust rejects it, look for a transcription defect.
   - If Rust accepts it, **adjudicate against the language contract** (the
     language reference, the relevant AIPs, the Move maintainers' intended
     semantics) before classifying it as a verifier defect. Rust accepting a
     module that goes `wrong` in the Lean semantics only shows a
     disagreement.
4. As supporting evidence for triage only, execute the counterexample on the
   VMs with paranoid checks enabled. An invariant violation or paranoid
   failure is strong evidence of a real defect. The guarantees do not depend
   on this step.
5. Failing to find a counterexample does not show that none exists. Search
   further, strengthen the proof, or reconsider the property.

### 10.2 Differential fuzzing (from Phase 0 onward)

Generators: the `bytecode_verifier_compiled_modules` fuzz target
(`testsuite/fuzzer/fuzz/fuzz_targets/move/`), the `invalid-mutations` crate,
and the `proptest_types` generators in `move-binary-format`.

- **Soundness:** generated modules that the Rust verifier accepts are executed
  in the Lean semantics on generated inputs. Reaching `wrong` makes the module
  a candidate for triage.
- **Acceptance correspondence (A2):** what transfers the proofs to Rust is that
  every module the Rust verifier accepts, the model also accepts. A module
  that Rust accepts and the model rejects is a gap in the soundness argument
  and is a priority. A module the model accepts and Rust rejects is a
  fidelity issue, not a soundness risk.
- Every deployed mainnet module is part of the corpus.

### 10.3 Calibrating against known bugs

Each regression in `move-bytecode-verifier/src/regression_tests/`, the
`bytecode-verifier-tests` unit tests, and past security fixes is matched to
the property it concerns. Not every regression should reach `wrong`.

| Regression kind | Expected calibration result |
|---|---|
| Runtime unsafety | The exploit reaches `wrong`; reintroducing the bug into the model breaks the matching D proof. |
| Static violation | The matching L or K property is false for the module; reintroducing the bug breaks that proof. |
| Verifier crash | A reachable `panic` in the model (totality). |
| Excessive rejection | A completeness issue (A6), not soundness. |
| Resource limit | L11 or R7. |
| Relies on a dynamic check (for example `reentrancy_local.move`) | Reaches the expected `failed` outcome, not `wrong`. |

If a calibration fails, the spec or the matching property is too weak or too
strong.

### 10.4 Coverage of VM assumptions

Every verifier guarantee that a VM relies on maps to a property ID, or is
reported as a gap. Starting point (MonoVM):

| MonoVM assumption (source) | Property | Notes |
|---|---|---|
| Stack balance; `Ret` drains exactly the declared returns (`destack/translate.rs`) | L4 | holds for every block |
| Operand types match each instruction (`translate.rs`) | C1, D1 | with subsumption for function types |
| Pool indices in bounds (`translate.rs`) | L1, K1 | imported handles need K1 |
| `Pack`/`Unpack`/variant instructions target declared fields and variants (`translate.rs`) | L1, L9 | the variant itself is R1 |
| Branch targets valid; code ends in a terminator (`translate.rs`, `ssa_conversion.rs`) | L3 | |
| Locals assigned before use; no use after move (`translate.rs`) | C2 | **reachable code only**; check whether destack translates unreachable blocks |
| Argument count matches the callee (`translate.rs`) | C1, D1, K1 | |
| Type-parameter indices in range (`translate.rs`) | L1, L5 | |
| Freed slots hold dead values (`translate.rs`) | D4e | reachable code only |
| Closure pack visibility (`closure_design.md`) | K2 | |
| Captured values conform and are not references (`closure_design.md`) | D5 | |
| Closure call arity and returns match (`closure_design.md`) | D5 | |
| Function-type depth and size limits (`closure_design.md`) | L11 | **syntactic tokens only**; post-substitution bounds are R7 |
| Lazy monomorphization terminates; layouts are finite (specializer design) | K3, K4 | **only under `Acyclic P`**; production allows cycles, so MonoVM must enforce R7 |
| (implicit) | R1–R4 | MonoVM must implement these checks |

The same exercise is repeated for the legacy VM's skipped checks, to show the
guarantee list is complete.

### 10.5 Rules for changing the specification

- **Clean room.** The spec owners (Syntax, Semantics including the R
  catalogue, Properties, native contracts) work from the language definition
  and do not read `move-bytecode-verifier/src`. The verifier modelers do the
  opposite.
- Every change to a trusted file needs a written first-principles reason, the
  triage record that prompted it, and review by a spec owner. "The verifier
  does it this way" is not a reason.
- Weakening a property *to make a proof go through* is not allowed.
  Correcting a property that triage showed to be false against the language
  contract is allowed and expected.

### 10.6 Disclosure

A confirmed verifier defect is a security vulnerability on a live chain.
Counterexamples go through the Aptos security process, not public issues,
pull requests or CI logs. Differential fuzzing that can produce
counterexamples runs on private infrastructure, and the divergence ledger
keeps suspected defects in a private section until they are fixed.

## 11. Assurance deliverables

These are tracked separately because each closes a different gap, and none of
them substitutes for another.

| ID | Deliverable | Evidence |
|---|---|---|
| A1 | **Model soundness** | The Lean proofs in sections 8 and 9. |
| A2 | **Rust-to-model acceptance**: every input the Rust verifier accepts, the model accepts. | Differential fuzzing and every deployed module. Later, pass-by-pass translation of the Rust code to Lean (for example with Aeneas); a translated pass covers only itself, so the remaining passes and the orchestration in `verifier.rs` still rely on testing. |
| A3 | **Configuration correspondence** | The Lean configuration equals the computed production `VerifierConfig` for the recorded inputs. |
| A4 | **Decoding correspondence** | The Lean import (exporter, or a Lean deserializer) agrees with the production deserializer from bytes to `CompiledModule`. A Lean deserializer does not remove this obligation. |
| A5 | **Mixed-version coverage** | The version-specific verification paths are modeled and proved; required before the Phase 5 whole-program claim. |
| A6 | **Exact agreement** (optional) | Needed only when an independent implementation must accept exactly what Rust accepts, as consensus would require. This is different from semantic completeness: two sound verifiers may reject different safe programs. |

## 12. Phases

Differential fuzzing (10.2) starts in Phase 0 and grows with the supported
subset, so bug-finding does not wait for the proofs.

| Phase | Scope | Results and exit criteria |
|---|---|---|
| 0. Foundations and scope | Record and classify the configuration (A3). Decide on dependency cycles (4.3) and version scope (4.2). Write the R catalogue, the outcome classification with rollback, and the semantics for the core subset with N1. Write the property statements; build the exporter and differential harness. | Spec signed off by clean-room reviewers. Semantics agrees with both VMs on the corpus. The negative corpus reaches `wrong`, and fixtures that rely on R-checks reach the expected `failed`. |
| 1. Core code | Integers of every width, bool, address, locals, branches, non-generic calls, aborts, constants. Proof of the absint framework with all section 9.2 obligations. | L1–L4, L8, L9, C1, C2, D1 for the subset; N2. |
| 2. Data and abilities | Structs, enums, vectors, signer, abilities; signature and instruction-consistency passes; struct definitions; R1, R2. | L5, L6, L10; D2 with its accounting; D3. |
| 3. References and storage | Borrow instructions, `ReadRef`/`WriteRef`/`FreezeRef`, global operations, acquires; reference-safety model and borrow-graph concretization; R3–R5. | D4a–e with the trace definitions. Expected to be the largest phase. |
| 4. Generics and function values | Generic instruction forms, full `signature_v2`, instantiation loops, `PackClosure`/`CallClosure`, subsumption; R6, R7. | L7, L11; D5; D1–D4 extended to generic code and closures. |
| 5. Whole program and versions | Linking, friends, entry points, upgrades, robust encapsulation; older bytecode versions (A5). | K1–K5, W1–W4, N3, N4. Exit: the production whole-program claim, stated with all premises. |
| 6. Strengthening correspondence (optional) | Lean deserializer with decoding correspondence (A4); translation of selected passes (A2); exact agreement (A6). | Each item narrows one specific gap; none closes the others. |

## 13. Repository layout and CI

A new, independent Lake package, for example
`third_party/move/lean/bytecode-verifier/`. It uses the tree's pinned
toolchain and has no dependency on the `leaner-*` packages, which keeps the
trusted base small and separate from source-level Leaner.

```text
Spec/Syntax/        trusted: abstract syntax
Spec/Semantics/     trusted: reference semantics, R catalogue, event trace
Spec/Properties/    trusted: L, K, E, C, D, W, N statements; native contracts
Spec/Config/        trusted: recorded configuration inputs and value (A3)
Model/              untrusted: one file per Rust pass, with Rust path and hash
Proofs/             untrusted: absint framework, per-pass proofs, main theorems
Import/             module import and round-trip tests (A4)
Tests/              semantics tests, calibration corpus, model-agreement tests
```

On the Rust side, a small exporter emits `CompiledModule` and
`CompiledScript` for Lean, and a harness drives differential runs against the
Rust verifier and both VMs.

CI runs:

- `lake build`;
- the axiom audit and the `sorry` report;
- model and Rust agreement on the corpus;
- a **drift check**: a change to any transcribed Rust file, or to
  `prod_configs.rs`, whose hash no longer matches the recorded one fails CI
  until the model or configuration is reviewed;
- CODEOWNERS on `Spec/` that requires a spec owner's approval.

Nightly fuzzing that can produce counterexamples runs privately (10.6).

## 14. What the result means

When Phases 0–5 are complete, the following statement is machine-checked:

> For every program in which each module is accepted by the model of the
> production verifier (recorded configuration, supported versions) and passes
> link checks, executed by the reference semantics, including the
> language-required dynamic checks R1–R7, with natives that respect their
> contracts and entry arguments validated by the transaction layer: no
> execution goes wrong, and ownership accounting, bytecode-level
> encapsulation, reference safety and function-value safety hold. They hold
> robustly against arbitrary other verified code and across upgrades that are
> verified and compatible. Finite layouts and finite monomorphization hold
> additionally when module dependencies are acyclic.

It rests on:

- the trusted base (syntax, semantics with the R catalogue, property
  statements, native contracts), which is kept small, reviewed by a clean
  room, and tested against both VMs and known regressions;
- deliverables A2–A5, until each is proved rather than tested;
- the Lean kernel.

A VM inherits the guarantees by showing, separately, that it refines the
reference semantics on verified code, which includes implementing R1–R7. The
verifier proofs do not change when a new VM is added.

## 15. Relationship to Leaner

The bytecode reference semantics is also the natural target semantics for
Leaner's open compiler-correctness work: a source theorem transported to
emitted bytecode. That connection is future work, and nothing in this plan
depends on it.

## 16. Open questions

1. **Exclusivity formulation.** D4b′, D4b″ and D4c now have trace
   definitions. An instrumented permission model is the alternative; pick
   whichever the spec owners find easiest to review.
2. **Native contracts.** Choose between one generic contract and per-native
   contracts. Some natives may deliberately produce or inspect values of
   other modules' types (for example deserialization- or reflection-style
   natives). Each needs an explicit exception, and D3/W2 are stated modulo
   those exceptions.
3. **Dependency cycles.** Decided in Phase 0: option (b) of 4.3.
4. **Closures and K4.** Under `Acyclic P`, do the local instantiation checks,
   which include closure creation, imply global finiteness when closures are
   invoked across modules? This is an obligation of K4 and an early target for
   a differential test.
5. **Upgrade timing and persisted closures.** Confirm against the publishing
   flow that upgrades take effect only between executions, and pin down which
   compatibility rules apply to persisted closures.
6. **MonoVM's R-checks.** Which of R1–R7 MonoVM implements today, and whether
   its destack phase translates unreachable blocks (C2, D4e).
7. **Configuration evolution.** Parameterize theorems over the configuration,
   or re-prove each production configuration as feature flags change.

## 17. References

- S. Blackshear et al., *Resources: A Safe Language Abstraction for Money*, 2020.
- M. Patrignani and S. Blackshear, *Robust Safety for Move*, CSF 2023
  (arXiv:2110.05043).
- S. Blackshear et al., *The Move Borrow Checker*, 2022 (arXiv:2205.05181).
- G. Klein and T. Nipkow, *Verified Bytecode Verifiers*, TCS 2003; and *A
  Machine-Checked Model for a Java-Like Language, Virtual Machine, and
  Compiler*, TOPLAS 2006.
- S. Ho and J. Protzenko, *Aeneas: Rust Verification by Functional
  Translation*, ICFP 2022.
- Code:
  - `move-bytecode-verifier/src/verifier.rs` (pass order);
  - `move-bytecode-verifier/src/absint.rs`;
  - `move-bytecode-verifier/src/reference_safety/abstract_state.rs`;
  - `move-binary-format/src/{file_format,check_bounds,compatibility}.rs`;
  - `move-vm/runtime/src/storage/{environment,publishing}.rs`;
  - `move-vm/runtime/src/reentrancy_checker.rs`;
  - `aptos-move/aptos-vm-environment/src/prod_configs.rs`;
  - `aptos-move/aptos-vm/src/verifier/transaction_arg_validation.rs`;
  - `mono-move/specializer/src/destack/translate.rs`;
  - `mono-move/docs/closure_design.md`.

## Appendix A. Changes from the first draft

| First draft | Problem found in review | Revision |
|---|---|---|
| D4 relied on the static verifier alone | `CallClosure` carries no `acquires` set; the runtime resource lock compensates (`reentrancy_local.move`) | R catalogue (section 6); D4 depends on R3/R4 |
| W1 implied global acyclicity; S6/S7 global | Lazy publishing skips cycle checks; `cyclic_structs.masm` publishes recursive structs | Phase 0 decision (4.3); K3/K4 need `Acyclic P`; R7 |
| W2 claimed exclusive control and invariant preservation | Other modules may copy, drop and compare when abilities allow; escaping `&mut` breaks invariants | W2a (encapsulation) separated from W2b (invariants with premises) |
| `verify_static` derived linked and entry facts from one module | External handles and visibility need linking; entry rules come from transaction validation | L / K / E split |
| W3 relied on compatibility alone | Compatibility does not check new function bodies | Replacement verification, pinned policy, timing, persisted closures |
| C1 required exact type equality over the whole stack | Function-ability subtyping; shared operand stack | Conformance with subsumption; per-frame segments; availability-conditioned locals |
| S5 banned references inside nested types | Function types may have reference parameters and results | Recursive formation rules (L5) |
| D2c conservation | `copy` without `drop`; `VecUnpack` and `CallClosure` consume legitimately | Identity and occurrence accounting |
| Outcomes omitted legitimate failures; "deterministic" | Variant mismatch and `VecUnpack` length failures; nondeterministic limits and natives | Complete classification; deterministic core plus stopping and native relations; rollback |
| Configuration = commit + two features | Config depends on gas version, flags, timed features; switches are not limits | Recorded inputs (A3); three field classes; measured objects for L11 |
| Rust acceptance + `wrong` = confirmed bug | Could be a specification defect | Four-way triage with adjudication |
| Phase 6 closed the gaps | Partial translation, decoding correspondence, mixed versions, completeness ≠ agreement | Deliverables A1–A6; mixed versions before Phase 5 |
| Absint: transfer + join only | Scheduling, `Unchanged`, canonicalization, initial state; unreachable blocks are never analyzed | Section 9.2; C2 and D4e limited to reachable code |
| No non-vacuity checks | A stuck semantics or contradictory contracts would make theorems vacuous | N1–N4 |
