# Bytecode verifier soundness: method, theorems, assumptions

A summary for reviewers of what this package proves about the Move bytecode
verifier, how, and under which assumptions. Status: 2026-11-04, end of plan
Phase 4. The plan is
[`../doc/designs/bytecode-verifier-soundness.md`](../doc/designs/bytecode-verifier-soundness.md);
[`README.md`](README.md) has the detailed status, every spec decision and the
open questions. Names in `code` are Lean definitions or files of this package.

Contents:

1. [The method](#1-the-method)
2. [The theorems](#2-the-theorems)
3. [The assumptions](#3-the-assumptions)

## 1. The method

### 1.1 The idea

1. Write, in Lean, a **small-step semantics of Move bytecode**: one step runs
   one instruction. The semantics checks every rule of the language as it
   runs, and when a rule is broken the step's outcome is `wrong`.
2. **Transcribe the Rust verifier** into Lean, pass by pass, with its
   algorithms and its accept or reject decisions.
3. **Prove** that when the transcribed verifier accepts every module of a
   program, no execution of the program reaches a `wrong` outcome (D1). Prove
   further properties of every execution the same way: ownership (D2),
   encapsulation (D3), reference safety (D4) and function values (D5).
4. **Test** what proofs do not cover: that the transcription decides as the
   Rust verifier does, and that the semantics agrees with the Move VMs.

The Lean kernel checks every proof.

### 1.2 What is trusted

| Layer | Files | Trust |
|---|---|---|
| Syntax | `Spec/Syntax.lean` | Trusted: must match the binary format. |
| Semantics | `Spec/Semantics/` | Trusted: must be the meaning of Move bytecode. |
| Statements | `Spec/Properties/` | Trusted: must say what is meant. The theorems' exact shapes are `Spec/Properties/Theorems.lean`. |
| Configuration | `Spec/Config.lean` | Trusted: must equal the production verifier configuration. |
| Verifier model | `Model/` | Untrusted by the proofs. It matters only for how the result carries over to Rust (assumption 7). |
| Proofs | `Proofs/` | Untrusted: the kernel checks them. |
| Import | `Import/`, `exporter/` | Tested: turn compiled modules into Lean values. |

A mistake in `Model/` or `Proofs/` cannot make a theorem false: the kernel
would reject the proof. A mistake in `Spec/` can: a wrong semantics or a
statement that is too weak. That is why `Spec/` is written from the language
definition only (the clean-room rule: its authors do not read the verifier),
and why reviewing it is where human attention matters most.

### 1.3 The semantics

**Programs.** A program (`Program`) is a fixed set of compiled modules.
Scripts are not modeled.

**State** (`Spec/Semantics/State.lean`):

- a call stack of frames. Each frame runs one function instance with closed
  type arguments and has a pc, its locals (each `available v` or
  unavailable) and its own segment of the operand stack;
- global storage: a map from (address, closed struct type) to a value;
- for each module, how many times it is active, and the module-lock depth
  (used by R3 and R4, below);
- a log of invalidations, which says which references are no longer valid;
- the values the transaction holds and lends by reference (`&signer`).

**Values** (`Spec/Semantics/Values.lean`) are finite trees: integers tagged
with width and sign, `bool`, `address`, `signer`, vectors, struct and enum
values, closures, references, and tables. A reference is a location (a root,
which is a local of a frame or a global resource or a held value, and a path
of field, element and entry steps), a mutability flag and a target type. A
table holds its fields and its entries.

**One step.** `step P s` (`Spec/Semantics/Step.lean`) is a deterministic
Lean function. It runs the current frame's instruction and checks types,
abilities, the availability of locals, stack heights, visibility, that the
caller's handle agrees with the callee's definition, and the validity of
references. The outcome is one of:

| Outcome | Meaning |
|---|---|
| `running s'` | the execution goes on in state `s'` |
| `returned vs st` | the first frame returned `vs`; global storage `st` is committed |
| `failed f` | a failure the language defines: `Abort`, or one of the checks R1 to R6 below; storage effects roll back |
| `limit` | out of a resource (gas, depth, memory); storage effects roll back |
| `wrong r` | the bytecode broke a rule of the language, for reason `r` (`WrongReason`) |

Only `wrong` is bad. An instruction outside the supported subset is
`wrong (unsupported _)`; every theorem assumes the program is in the subset
(assumption 14).

**Two sources of nondeterminism** (`Spec/Semantics/Run.lean`):

- *Natives* are a relation `NativeRel`, not code. A native call suspends with
  what the native sees of its arguments (the values its reference arguments
  designate, never the references), the relation gives any result, and the
  semantics checks the result against the native's shape: wrong-typed values,
  writes through immutable references, or places outside the arguments are
  `wrong .nativeContract`. Then it writes the new values back and makes the
  returned references.
- *Stopping*: an execution may stop with `limit` at any state. The statements
  quantify over reachable states, so a stopped execution is a prefix.

`StepRel P N s o` is one step, core or native. `Reachable P N s₀ s` says `s`
is reached from `s₀` by steps that keep running.

**Checks the language requires at run time** (R1 to R7). The verifier does
not establish these; the semantics performs them, and every VM must too:

| | Check | Outcome if it fails |
|---|---|---|
| R1 | Unpacking an enum or borrowing a variant field needs the runtime variant to match. | `failed` |
| R2 | `VecUnpack(n)` needs the vector's length to be `n`. | `failed` |
| R3 | Resource lock: each module has an active count, raised by a call into it from another module, by every closure call (even within one module), and by the transaction's entry. A global storage operation on a type of a module active more than once fails. | `failed` |
| R4 | Module lock: inside a `#[module_lock]` function (or native dynamic dispatch, outside the subset), re-entering an active module fails. | `failed` |
| R5 | `MoveFrom` and the global borrows need the resource to exist; `MoveTo` needs it absent. | `failed` |
| R6 | Arithmetic overflow, division by zero, bad casts and shifts, vector index out of range. | `failed` |
| R7 | Layouts, value depth and type sizes are bounded by the VM. | `limit` |

### 1.4 Soundness: no reachable `wrong`

D1 is the statement that no reachable state has a step to `wrong`
(`NoWrong`, `Spec/Properties/Safety.lean`):

```lean
def NoWrong (P : Program) (N : NativeRel) (s₀ : State) : Prop :=
  ∀ s, Reachable P N s₀ s → ∀ r, ¬ StepRel P N s (.wrong r)
```

It is proved with an invariant: a property of states that holds at the
start, that every running step keeps, and under which the next step is never
`wrong`.

A semantics that got stuck would make D1 hold for an empty reason. N1
(`progress`) rules this out: when natives always give some result, every
state has a step.

### 1.5 The ghost layer (for D2, D3, D4)

Ownership and reference safety talk about *which* value went where, which
plain values cannot show (two equal coins look the same). The ghost layer
(`Spec/Semantics/Trace.lean`) keeps a shadow beside the state and never
changes the state or any outcome:

- every node of every value gets an *identity*. Copying keeps identities;
  `Pack`, `VecPack`, `PackClosure`, loads, operator results, natives and the
  transaction create new ones;
- each step is annotated with *events*: identities created and consumed,
  references created (with the reference they derive from) and used,
  locations read, written or inspected, locals that become unavailable,
  frames entered and returning, and the privileged operations on a struct
  type (field borrows, `TestVariant`, global storage operations), with the
  module whose code ran them;
- a `Trace` is an annotated execution. D2, D3 and D4 are checks over traces.

Each theorem about traces also requires every step to be annotated
(`ShadowAccounts`). A shadow that drifted from the semantics would make the
theorems false, not empty.

### 1.6 The verifier model

`Model/` has one file per Rust pass, run in the order of
`verify_module_with_config`, plus the link checks
(`dependencies::verify_module`). Each pass's file names the Rust file it
transcribes. A Rust panic is a rejection. Two parts are not modeled:

- the verifier meter. It only rejects, so the model accepts more modules than
  Rust. That is the safe direction: the theorems then cover every module Rust
  accepts, provided the model agrees with Rust otherwise;
- `control_flow_v5`: the model rejects bytecode versions 5 and older.

The theorems are stated over a fixed interface, not over any predicate
(`VerifierInterface`: `verifyModule` and `verifyLinking`). A program is
accepted by it (`WellFormedBy`) when its module ids are distinct and every
module passes local verification and the link checks against the other
modules.

### 1.7 How the proofs go

**Local properties L1 to L11.** Each comes from the pass that checks it:
bounds give index integrity (L1), duplication gives uniqueness (L2), control
flow gives code structure, including reducibility from Tarjan's check (L3),
stack usage gives stack discipline (L4), the signature checker gives type
formation and struct facts (L5, L6), and so on.

**Linking K1 and K2** come from the link checks, duplication and the
signature checker (`Proofs/Linking.lean`).

**D1** is proved in two halves:

1. *Certificates imply safety* (`Proofs/Certified.lean`). Suppose some type
   certificate (the types on the stack and in the locals at each pc, over the
   function's type parameters) and availability certificate (which locals
   hold a value at each pc) are consistent, and calls resolve, successors stay
   in the code, constants decode, and declared abilities are justified. Then
   no execution goes wrong. This half is stated with the spec's notions only,
   so it holds for any consumer of such certificates, MonoVM included.
2. *The model's analyses produce the certificates.* Type safety gives the
   type certificate (C1) and locals safety gives the availability certificate
   (C2). The abstract interpreter is proved once (`Proofs/Scheduler.lean`,
   `Proofs/Traversal.lean`): on success, every block reachable from the entry
   has a recorded state that covers every way into it.

With references, the invariant also relates each frame to the
reference-safety analysis (`RefInvariant`, `Proofs/RefInv.lean`):

- the borrow graph is given a meaning (`Proofs/RefConc.lean`): its edges are
  true of the actual references, and a mutable reference with another
  strictly inside it has a chain of edges to it, so the analysis's
  writability check means what it must;
- every reference the analysis tracks is valid, of its type, and rooted in a
  local of its frame or an older one, a held value, or a global resource;
- each suspended caller agrees with the analysis's state at its call;
- global borrows across calls are protected by the acquires checks and by
  R3, for calls the analysis cannot see (closures, re-entry).

**D2 and D3** (`Proofs/Ownership.lean`, `Proofs/Accounting.lean`). Every
step that continues has an *effect* on the occurrences of identities: each
is kept (maybe elsewhere), dropped (only with `drop`), consumed by an event,
copied (only with `copy`), or created fresh. Uniqueness, no implicit
destruction and conservation follow from effects in general. Every core
instruction has such an effect, natives have one by their contract, and the
transaction's initial state creates every node legitimately by E1 and E2.
These proofs use D1 to rule out `wrong` steps.

**D4** (`Proofs/ReferenceSafetyAll.lean`). Each part reduces to a fact about
one step and the references still held (on operand stacks and in available
locals). The reference-safety analysis's checks give that fact. D4a (no
dangling use) follows from D4b′, D4c and D4e for every program
(`noDanglingFromContainment`): a reference becomes invalid only through an
event those three exclude before its last use.

**D5** (`Proofs/FunctionValues.lean`) follows from type safety's typing of
the closure instructions and the semantics' checks.

**Non-vacuity.** N1 holds because `step` is a total function. N2 builds an
initial state for every function. N3 builds natives that meet every
contract. N4 runs the model on the Aptos framework.

### 1.8 What is tested rather than proved

| Gap | Test | Size |
|---|---|---|
| Model vs Rust verifier (plan deliverable A2) | Same accept and reject decisions (`Tests/ModelAgreement.lean`) | 133 small test modules, accepted and rejected (36 of them each hold one constant) |
| | Same decisions on the framework's release bundle `head.mrb` (`scripts/check-framework.sh`) | all 191 modules (all accepted) |
| Configuration (A3) | The recorded value equals the Rust recorder's output (`Tests/Config.lean`) | every field |
| Import (A4) | Round trips and import tests (`Tests/Import.lean`); constant decoding by production, model and spec (`Tests/Differential.lean`) | 36 constants |
| Semantics vs VMs | Same outcome class, values and failures on the legacy VM and MonoVM (`Tests/Differential.lean`) | 73 calls: 57 core, 16 data by value |
| Semantics alone | Unit tests, including every `WrongReason` reachable in the subset (`Tests/Semantics.lean`) | — |

CI regenerates the fixtures from the current Rust verifier and VMs and fails
on any difference, and runs the framework check.

## 2. The theorems

### 2.1 List

Each theorem is proved for the model, `Model.verifier`, with the production
configuration. Premises, abbreviated (all in section 3.6):

- **WF**: `WellFormedBy Model.verifier VerifierConfig.production P`, the
  model accepts the program;
- **Ver**: `P.SupportedVersions`, bytecode versions 6 to 10;
- **Sub**: `P.Supported`, the program is in the supported subset;
- **NR**: `NativesRespect P N`, native results fit their shapes;
- **NO**: `NativesRespectOwnership P N ν π`, natives respect ownership, with
  flows `ν` and policy `π`;
- **AI**: `AdmissibleInitial P s₀`, the start is a call of any function
  instance with conforming arguments over well-typed storage;
- **EI**: `EntryInitial P auth A s₀`, the start is a transaction (E1, E2).

| ID | Lean theorem | Premises | What it says |
|---|---|---|---|
| L1–L11 | `localProperties_of_phantom` | the model accepts module `m`, version 6 to 10, and the hypothesis `PhantomUnderReferences` (assumption 19) | `m` has index integrity (L1), uniqueness (L2), code structure and stack discipline (L3, L4), well-formed constants (L8), instruction consistency (L9), type formation (L5), well-formed struct definitions (L6), friends (L10), finitely many instances through its own calls (L7), and is within the limits (L11). |
| K1–K2 | `linkedProperties` | WF, Ver, Sub | Every function and struct handle resolves to a definition it is a conservative view of (K1); every call and closure creation targets a visible function, and a handle marked `persistent` resolves to a storable function (K2). |
| C1–C2 | `certificatesHold` | WF, Ver, Sub | There are static type and availability certificates, consistent, and every execution agrees with them, whatever the natives do within NR. |
| D1 | `soundness` | WF, Ver, Sub, NR, AI | No reachable state steps to `wrong`. |
| D2 | `ownershipAccounting` | WF, Ver, Sub, NR, NO, EI | Every step is accounted, and: (D2a) a value without `copy` has one occurrence; (D2b) a value without `drop` disappears only by legitimate transfer or consumption; (D2c) after `returned`, every such value is in storage, in the return values, or was legitimately consumed. |
| D3 | `encapsulation` | WF, Ver, Sub, NR, NO, EI | Every step is accounted, and: (D3a) only code of a struct or enum type's module, or a native with its privileges, creates or takes apart values of the type, borrows their fields, tests their variant, or runs global storage operations on them; besides, the transaction creates values of accepted argument types, and values in storage at the start count as created earlier; (D3b) signers come only from authentication and the natives `π` allows; (D3c) closures are created only where their target is visible, or come from storage. |
| D4 | `referenceSafety` | WF, Ver, Sub, NR, NO, AI | Every step is accounted, and: (D4a) no reference is used dangling; (D4b′) no access reaches over a `&mut` that is still to be used; (D4b″) while a frame is active, no access conflicts with a live `&mut` parameter of it; (D4c) no write overlaps a live `&`; (D4d) no reference is stored in a value, in storage, or returned other than derived from a reference parameter; (D4e) no local or frame dies while a live reference points into it. |
| D5 | `functionValueSafety` | WF, Ver, Sub, NR, AI | (D5a) captured values hold no reference; (D5b) a closure's abilities are justified by its target and its captured values; (D5c) `CallClosure` enters its target with exactly its parameters, conforming; (D5d) only closures over storable functions are in global storage. |
| D4a | `noDanglingFromContainment` | Sub, AI, and D4b′, D4c, D4e | D4a follows from the other three, for any program: no verifier premise. |
| N1 | `progress` | natives always give some result | Every state has a step. |
| N2 | `entriesAdmissible` | none | Every function instance with code, type arguments satisfying its constraints, and constructible parameter types has an admissible initial state. |
| N2T | `transactionEntriesAdmissible` | none | Every entry function that returns nothing, with leading signers and constructible argument types, has a transaction initial state. |
| N3 | `nativeContractsSatisfiable` | none | For every program and policy, some natives and flows meet every contract, every native call has a result, and natives on plain data return. |
| N4 | (a check, `scripts/check-framework.sh`) | — | The model accepts all 191 modules of `head.mrb`, as Rust does; the theorems cover 121 of them. |

The main theorem, as stated in `Spec/Properties/Theorems.lean`:

```lean
def Soundness (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.Supported →
    NativesRespect P N → AdmissibleInitial P s₀ → NoWrong P N s₀

theorem soundness : Soundness Model.verifier
```

Every theorem above depends only on Lean's standard axioms
(`Tests/Axioms.lean`). Only the local properties take a hypothesis beyond
their premises (`PhantomUnderReferences`); D1 to D5 do not.

### 2.2 Coverage of the framework

Of the 191 modules of the framework's release bundle, the model accepts all,
as Rust does; 178 are in the supported subset; and 121 are in it together
with every module they depend on, so the theorems apply to programs built
from them. 38 natives with references outside the modeled ones keep the rest
out: aggregators, storage slots, dispatch, cryptography and others.

### 2.3 What the theorems do not say

- They are about the Lean model of the verifier. They carry over to the Rust
  verifier through assumption 7, which is tested, not proved.
- They are about the reference semantics. A VM inherits them only by
  behaving as the semantics does on verified code (assumption 24). No VM is
  proved to.
- No termination, gas or resource claim: `limit` is allowed at every state.
- D1, C1, C2, D4 and D5 hold from any admissible start, which allows calling
  any function with any conforming arguments. D2 and D3 hold only from a
  transaction's start: a forged value or signer at the start would falsify
  them.
- D2 and D3 are judged on the states before termination and on `returned`
  executions. A `failed` or `limit` execution rolls back.
- D3 is about integrity, not confidentiality: other modules may read a
  value (`Eq`, `bcs::to_bytes`, `cmp::compare`), and copy and drop it when
  its abilities allow.
- D4 is not full mutual exclusion of `&mut` references. Two `&mut`
  references a function borrows itself may alias at or below a target
  (D4b′); a function may still treat its `&mut` parameters as exclusive for
  the whole call (D4b″).
- C2 and the reference-safety analysis describe only code reachable from a
  function's entry: unreachable blocks are never analyzed. Executions only
  run reachable code, so D1 to D5 are unaffected; a consumer that translates
  unreachable blocks must not rely on them.
- Not proved yet (plan Phase 5): K3 (finite layouts), K4 (finite
  monomorphization), K5 (friends), W1 (modularity), W2 (robust encapsulation
  and invariant preservation), W3 (upgrades), W4 (entry points), A5 (older
  bytecode versions), and scripts.

## 3. The assumptions

Every assumption, numbered for reference. Each says where it lives and what
supports it.

### 3.1 Logic

1. **The Lean kernel (v4.32.2) checks proofs correctly.**
2. **Only the standard axioms are used**: `propext`, `Classical.choice`,
   `Quot.sound`. Checked: `Tests/Axioms.lean` prints the axioms of 73
   theorems, every theorem of section 2 included, and fails on any other.
   There is no `sorry`, `admit`, `axiom` or `native_decide` in `Spec/`,
   `Model/`, `Proofs/` or `Import/`.

### 3.2 The specification is right

3. **The syntax matches the binary format.** `Spec/Syntax.lean` follows
   `file_format.rs`. Supported by the import tests.
4. **The semantics is the meaning of Move bytecode.** Written from the Move
   book, the AIPs and the format's documentation, without reading the
   verifier. Supported by an independent clean-room review (Phase 0), unit
   tests, and agreement with both VMs on 73 calls. **Gap:** the VM
   comparison covers only the core and data by value. References, global
   storage, generics, closures and natives have unit tests but no VM
   comparison yet.
5. **The statements say what is meant.** Supported by the clean-room review,
   whose counterexamples to earlier, weaker statements are kept in
   `Tests/Review.lean`, and by the non-vacuity results N1 to N4.
6. **The semantics is defensive**: every violation a theorem excludes shows
   as `wrong` or in the trace. For example, copying checks the abilities of
   the actual value. N1 cannot catch a violation misclassified as `running`;
   the unit tests produce every `WrongReason` the subset can reach.

**Status:** human sign-off of `Spec/` is pending (README, "Awaiting
sign-off").

### 3.3 The model decides as the Rust verifier does (plan deliverable A2)

7. **Every module the Rust verifier accepts, the model accepts**, and the
   same for link checks. This is the assumption that turns the theorems into
   facts about the Rust verifier. Not proved. Supported by:
   - transcription pass by pass, with known transcription defects found and
     fixed (README, "Divergence ledger");
   - identical decisions on 133 test modules and on all 191 framework modules,
     rerun by CI against the current Rust verifier.

   **Gaps:** there is no check that flags a change to the Rust verifier's
   source; a change is caught only if it changes a decision on the recorded
   modules. The meter is not modeled (safe direction, section 1.6).

### 3.4 Configuration (plan deliverable A3)

8. **`VerifierConfig.production` is the configuration production uses.**
   Checked against the Rust recorder (`config-export/`, `Tests/Config.lean`).
   **Gap:** the recorded inputs are the default feature flags, not mainnet's
   on-chain flags. Each setting of a correctness switch is its own theorem
   instance; only this one is proved.
9. **Production verifies each module with the current configuration when it
   loads it** (plan §4.1), so only the current configuration matters.

### 3.5 Getting modules into Lean (plan deliverable A4)

10. **The export and import give Lean the module the Rust verifier saw.**
    `exporter/` reads the production deserializer's output; `Import/` reads
    the JSON. Tested, not proved (section 1.8).
11. **One deserializer fact is stated in the spec**: below version 10, no
    function handle carries a struct-API attribute, because the production
    deserializer rejects one (`Program.SupportedVersions`).

### 3.6 Premises of the theorems

A guarantee applies to a program and an execution only when these hold.

12. **WF, the model accepts the program** (`WellFormedBy`): module ids are
    distinct, and every module passes the model's local verification and its
    link checks against the program's other modules. The program is a fixed,
    closed set of modules: every import is present.
13. **Ver, versions 6 to 10** (`Program.SupportedVersions`), with
    assumption 11.
14. **Sub, the supported subset** (`Program.Supported`). Every instruction is
    in, including generics, references, global storage and closures. Every
    native takes and returns values only (no references, no function types)
    or is one of the 22 modeled natives (assumption 21). Outside: other
    natives with references, natives whose signatures name a function type,
    native structs, closures over natives, and, for the table natives, a
    `0x1::table` that breaks the table rules (assumption 22).
15. **NR, native results fit their shapes** (`NativesRespect`): returned
    values conform deeply to the declared return types; only an update-shape
    native writes back, one conforming value per mutable reference parameter;
    only a borrow-shape native returns places, each inside what one of its
    reference arguments designates, of the right type, never more mutable
    than that argument.
16. **NO, natives respect ownership** (`NativesRespectOwnership`, for D2, D3,
    D4). For each call that returns, the flow `ν` says which output nodes are
    argument nodes passed on, and:
    - a node passed on is unchanged;
    - a node without `copy` is passed on at most once;
    - a node the native creates is plain data, a vector, a value of its own
      module's types, or what the policy `π` grants. It creates no closure,
      and creates a signer only where `π` allows;
    - an argument node not passed on is consumed legitimately: dropped with
      `drop`, a vector, a value of a type it has the privileges of, or a
      value `π` lets it destroy whole. It takes a struct or enum value apart
      (passes on something inside it) only with the privileges of its type;
    - what an immutable reference argument designates, or a mutable one the
      native does not write back, stays where it is: nothing in it is
      consumed, and a node without `copy` in it is not passed on.

    **The policy `π`** (`NativePolicy`) lists the exceptions: natives with
    privileges over another module's types, natives that create signers, and
    natives that may destroy values without `drop`. D2 and D3 hold for
    every `π`, and say less the more `π` grants. To read them for production,
    `π` must be the real list of exceptions (for example `create_signer`, and
    `drop_unchecked_box` destroying entries without `drop`). That list is not
    written down in this package.
17. **AI, an admissible start** (`AdmissibleInitial`): one frame calling a
    function instance with code; its type arguments are closed types of the
    program satisfying its constraints; its arguments conform deeply to its
    parameter types; global storage is well typed (every resource conforms to
    its declared type); each reference argument designates a whole value the
    transaction holds, and no two designate the same one.
18. **EI, a transaction start** (`EntryInitial`), which implies AI:
    - E1: the function is an entry function returning nothing; its
      parameters are leading `signer` or `&signer` parameters followed by
      transaction argument types; the struct types the transaction layer
      accepts (`A`) each have `copy`, lack `key`, and have fields of argument
      types, recursively;
    - E2: each signer is for an authenticated address (`auth`).
19. **`PhantomUnderReferences`, a hypothesis of the local properties only**
    (`Proofs/Declarations.lean`). The model's acceptance does not establish
    L6's phantom-position rule for field types that mention a reference, so
    `localProperties_of_phantom` takes that rule as a hypothesis. A fix of
    the Rust verifier is tracked separately; `localProperties_of_switch`
    proves the local properties without the hypothesis once the fix is in the
    production configuration. D1 to D5 do not depend on it.

### 3.7 What natives are assumed to do

20. **A native is a relation with no other effect.** Given its function
    instance and what it sees of its arguments, it gives a result: a
    failure, or values, values written back and places. It changes nothing
    in the execution except through its mutable reference arguments (update
    shape only), and runs no Move code. Its result may depend on anything.
    Whether each real native fits this is not machine-checked. The six table
    natives' Rust implementations were read against their shapes by hand
    (2026-11-04).
21. **The 22 modeled natives and their shapes**
    (`Spec/Semantics/ModeledNatives.lean`). A native is modeled when its
    module is `0x1::<module>`, its name is listed, and its declared
    signature matches exactly, constraints included. The contracts promise
    the shape only: any behavior within it, failing included, is allowed.

    | Shape | Natives |
    |---|---|
    | read: reads through `&`, returns values, writes nothing | `vector::length`, `string::internal_check_utf8`, `internal_is_char_boundary`, `internal_sub_string`, `internal_index_of`, `bcs::to_bytes`, `bcs::serialized_size`, `cmp::compare`, `table::contains_box`, `table::destroy_empty_box` |
    | update: writes new values through its `&mut` arguments | `vector::push_back`, `pop_back`, `swap`, `move_range`, `mem::swap`, `table::add_box`, `table::remove_box` |
    | borrow: returns references inside its reference arguments | `vector::borrow`, `borrow_mut`, `signer::borrow_address`, `table::borrow_box`, `table::borrow_box_mut` |

    The natives of `std::vector` marked `#[bytecode_instruction]` are
    ordinary natives of their shape, not their instructions. A native call
    reads the whole value at each reference argument, and a mutable
    reference argument may overlap no other reference argument.
22. **Tables** (language-owner rulings, 2026-11-04):
    - a table's entries are inside the table value, for ownership,
      abilities and locations. The implementation keeps them in storage under
      the table's handle; the two agree as long as **no two tables share a
      handle**. This is the modeling assumption;
    - the table natives are modeled only when the program's `0x1::table`
      keeps the table rules (`CompiledModule.followsTableRules`), checked on
      its bytecode. Among other things, the rules say: tables are made only
      from `new_table_handle`'s result, never unpacked, never copied, and
      their handle is never rewritten. The framework's `0x1::table` keeps
      them (checked by N4);
    - keys count for equality only: they have no identity and no reference
      reaches them;
    - `drop_unchecked_box` consumes the table with its entries, which `π`
      must allow (assumption 16). `destroy_empty_box` promises nothing about
      emptiness.

### 3.8 What the VM and the transaction layer must do

These are obligations on the rest of the system. The theorems assume them
but nothing here proves them.

23. **The program as loaded passes the link checks** (assumption 12). In
    production, a module is link-checked against its dependencies when it is
    published, and dependencies may be upgraded later. That compatible
    upgrades keep the link checks and storage well typed is W3, not proved.
24. **A VM behaves as the reference semantics on verified code**, except that
    it may stop with a limit anywhere. Not proved for any VM; tested for the
    core and data by value (assumption 4).
25. **A VM performs the checks R1 to R7** (section 1.3), with R3 also counting
    closure calls within one module.
26. **`failed` and `limit` roll back** the transaction's effects on global
    storage. D2c relies on it.
27. **The transaction layer enforces E1 and E2** (assumption 18): only entry
    functions that return nothing, arguments it built and validated, and
    signers only for authenticated addresses.
28. **Global storage at the start is well typed.** Values stored by earlier
    executions of the same verified code are, by D1. Across upgrades this is
    W3.

### 3.9 Decisions about the language the spec encodes

The spec takes these as the language's meaning. Each was decided by the
language owners or maintainers on the date given; the README gives the
reasons.

| Date | Decision |
|---|---|
| 2026-10-02 | Cyclic module dependencies are allowed, as production allows them. No theorem assumes acyclicity; K3 and K4 would need it. |
| 2026-10-02 | Signed `MIN % -1` fails, as both VMs do. |
| 2026-10-02 | A struct handle may be a conservative view of its definition (fewer abilities, more constraints). |
| 2026-10-05 | Reference safety is D4b′ and D4b″, not full exclusion: `&mut` references a function borrows itself may alias at or below a target. |
| 2026-11-03 | A function is storable exactly when it is `public` or `#[persistent]`; `entry` alone does not count. |
| 2026-11-03 | A closure mask bit beyond the target's parameters is `wrong`. Two closures are equal when function instance, mask and captured values are equal. |
| 2026-11-03 | R3 covers all five global storage operations and is checked before R5. |
| 2026-11-03 | A transaction calls only entry functions that return nothing. |
| 2026-11-03 | Natives with references are described by three shapes (read, update, borrow), not per native. `signer::borrow_address` borrows the address inside the signer. |
| 2026-11-04 | Natives are identified by exact declaration. A native reads the whole value at each reference argument. A `&mut` argument overlaps no other reference argument. |
| 2026-11-04 | D3 is about integrity, not confidentiality, so read-only natives need no exception. |
| 2026-11-04 | Tables: assumption 22. |

One assumption is about the compiler and affects only which modules are
accepted, not soundness: a struct field of function type is written in
bytecode with the abilities it inherits from the struct.

### 3.10 Open questions

Not yet decided; the spec's current choice is in force:

- Should N3 also require natives of the update and borrow shapes to return?
- May a closure that is not storable be the key of a table in global
  storage? D5d does not look at keys.
- How may a module know that an imported function is storable, other than by
  a `persistent` attribute on its handle?
- Should conformance also check the constraints of type arguments that no
  value reaches (phantom ones)?

The README ("Open") lists the remaining, smaller questions.
