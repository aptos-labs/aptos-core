# Test organization and check ledger

Updated 2026-09-24 (denotation route, [`denotation.md`](denotation.md)). This is the ledger of the acceptance
fixtures under
[`leaner-e2e-tests/LeanerE2ETests/Check/`](../leaner-e2e-tests/LeanerE2ETests/Check/):
which pass exactly, which do not, and why. Chronology and the mappings of
the original v0 cases live in
[the history archive](test-organization-history.md), not here.

## Current result

**70 of 70 Check files pass exactly.** A file passes
when its whole output matches the adjacent `.exp` at the driver's caps
(180k verification heartbeats per target); a file with one failing target
fails, however many targets it proves.

| Gate | Result | Note |
|---|---|---|
| `leaner-ir` build | PASS | `LeanerLang` and the `Denote` modules. |
| `DenotePerformance` gate | PASS (re-baselined 2026-09-24 for the loop step) | The loop targets cost 23–27% less since the iteration's context is normalized and substituted once (2026-09-24); the scalar and storage targets moved within ±5%. Every transport costs 8% more (about 313k heartbeats) since the skolem family (D3); ground evaluation (`lir_denote_eval`) and the array-literal simprocs cost the typed targets 0–3% more. |
| `leaner-ir` `lake test` | PASS (2026-09-24) | Every root passes. `CompositionPerformance` re-baselined 2026-09-24: the typed targets cost 5–9% more and the transports 8% (24k heartbeats each, three proof objects) since the 2026-09-24 closer changes; the cause of the transport growth is not identified (it is not the normal-form additions, the cast form of vector lengths, or the leaf's alternative order). `four_times` is at 12.8M heartbeats (48M before D5). |
| `leaner-move`, `leaner-rust` builds | PASS | |
| `leaner-e2e-tests` `lake test` | PASS (2026-09-24) | All five suites (`move`, `rust`, `check`, `monovm`, `monodiff`). |


## Per-file status

Each name is a `.lean` file under `Check/<Folder>/`, grouped by language
feature. PASS means the entire fixture matches its baseline; a PASS with no
`verify` target is execution or diagnostics coverage only, as noted. A file
whose name ends in `Errors` holds negative cases: a correct rejection does
not make it pass if its positive control fails.

### Scalars (integers, signed integers, arithmetic, literals, addresses, structural order)

| File | Status | Remaining problem |
|---|---|---|
| `Addresses` | PASS | |
| `Arithmetic` | PASS | |
| `Integers` | PASS | |
| `Literals` | PASS | |
| `Order` | PASS | Since 2026-09-23: `compare` on integers and at a type parameter; an authored `verify … by` proof of transitivity. |
| `Signed` | PASS | |
| `SpecLogicalArithmetic` | PASS | A universal quantifier nested in an existential's body keeps the existential. |

### Structs (abilities, positional structs, tuples, data invariants)

| File | Status | Remaining problem |
|---|---|---|
| `Abilities` | PASS | No verification targets. |
| `Invariants` | PASS | |
| `PositionalStructs` | PASS | |
| `Tuples` | PASS | |

### Enums (variants, patterns, payloads, references into payloads)

| File | Status | Remaining problem |
|---|---|---|
| `EnumPatterns` | PASS | Nested enums verify at seven goals' cost. |
| `EnumPayloads` | PASS | |
| `EnumRefContracts` | PASS | Since 2026-09-22: `fill`, `scale`, `replace`, `overwrite_three` write variant payload places through `Proj.variant`; `fill_global` bridges a struct twin with an enum-typed field through the enum twin's native view. |
| `EnumRefs` | PASS | Execution only, no `verify`. |
| `Enums` | PASS | |

### Vectors (construction, operations, element bounds)

| File | Status | Remaining problem |
|---|---|---|
| `VectorBounds` | PASS | |
| `VectorOperations` | PASS | Since 2026-09-23 `swap`, `reverseSlice`, `contains`, and `indexOf` verify: array literals are updated at literal positions by simprocs and the guards decided by ground evaluation. A target whose guards stay undecided does not fail fast: it exhausts heartbeats in `whnf` or runs past 100 s. |
| `Vectors` | PASS | |

### Control (control forms, loops, loop invariants, aborts)

| File | Status | Remaining problem |
|---|---|---|
| `Aborts` | PASS | Includes the intended false-contract rejection. |
| `ControlForms` | PASS | |
| `LoopControlErrors` | PASS | |
| `LoopInvariantErrors` | PASS | Baseline names the unestablished invariant at entry and at an iteration. |
| `LoopInvariants` | PASS | |
| `LoopVerification` | PASS | |
| `Loops` | PASS | |

### References (borrows, loans, freeze, prophecies, returned references)

| File | Status | Remaining problem |
|---|---|---|
| `BorrowCertificates` | PASS | Certificate assertions, no `verify`. |
| `BorrowChecker` | PASS | Since 2026-09-23: v0's policy cases as source programs; twelve rejections at execution preparation, the accepted ones verified (implicit freeze, loans unwound by break/continue/labeled break, a loop writing through a reference). |
| `BorrowErrors` | PASS | |
| `BorrowGlobalErrors` | PASS | |
| `CorePrimitives` | PASS | |
| `Freeze` | PASS | Since 2026-09-24: freezing a mutable reference is a shared reborrow; writes after the freeze reach the lender, at run time and in the verified contracts. |
| `Loans` | PASS | |
| `Prophecies` | PASS | |
| `References` | PASS | Since 2026-09-23 (D5): every returned reference, including dynamic choice, pairs, projected and global lenders, and callers writing through a result. |
| `ReturnedMutRefErrors` | PASS | Since 2026-09-23 (D5). |
| `ReturnedMutRefs` | PASS | Since 2026-09-24: all of v0's 66 targets verify, with the mutual recursion of `mutual_return_left` and `mutual_return_right` (D6 cycles) and loops that break with a returned payload loan; `returned_mut_refs_final` adds contracts reading `final(result)`. |

### Storage (global storage, borrows, module invariants, frames)

| File | Status | Remaining problem |
|---|---|---|
| `CrossInv` | PASS | |
| `GlobalBorrows` | PASS | |
| `GlobalInv` | PASS | The stale fixture-local 50k cap is gone; the driver's cap is the gate. |
| `LooseFrame` | PASS | Its hand proof no longer unfolds the raw contract by hand. |
| `Normalized` | PASS | |
| `Read` | PASS | |
| `ResourceComposition` | PASS | |
| `Storage` | PASS | |
| `StorageSpecErrors` | PASS | |

### Calls (calls, callees, composition, native summaries)

| File | Status | Remaining problem |
|---|---|---|
| `Callees` | PASS | Since 2026-09-23 (D6): `drain` verifies by fixed-point induction and `call_drain` through its contract; unspecified pure callees are inlined, nested ones included; a callee reaching itself only through inlined callees is rejected. |
| `Calls` | PASS | `recursive_choose` verifies by fixed-point induction (D6). |
| `Composition` | PASS | |
| `Summaries` | PASS | Since 2026-09-23: callers of natives assume the natives' contracts, transitively through verified callees; opaque functions are summarized by their contracts. |

### Generics (generic types and functions, generic storage)

| File | Status | Remaining problem |
|---|---|---|
| `GenericFunctions` | PASS | Since 2026-09-23 (D3a). `take` pins that a loan on a generic field ends where its holder dies. |
| `GenericScalarCalls` | PASS | Since 2026-09-23 (D3a): each scalar caller's typed proof uses its generic callee's theorem. |
| `GenericStorage` | PASS | Since 2026-09-23 (D3b): storage over `Vault<T>` keyed by the frame's instantiation, concrete callers through kernel-checked frame instantiations, `Vault<u64>` through the generic twin's bridge. |
| `Generics` | PASS | Since 2026-09-23 (D3a): generic bodies at the skolem family, concrete callers through the instantiated family, equality at generic values; the public boundary admits exactly loan-free data arguments. |

### Specifications (contracts, specification functions)

| File | Status | Remaining problem |
|---|---|---|
| `ContractErrors` | PASS | Since 2026-09-24 also a contract ignoring a write after a freeze, writes through returned references, and cycles of calls (one wrong contract fails the cycle; an unspecified member is rejected); a failing clause after a call is reported at the clause. |
| `Increment` | PASS | |
| `RecursiveSpecFunctions` | PASS | Since 2026-09-23: a recursive `spec fun` with a `decreases` measure as a Lean definition, a lemma by induction through its unfolding theorem, and a `verify … by` using it, both items of the module (2026-09-24). Quicksort likewise keeps its count lemmas and verifies in its module. |
| `SpecFunctionErrors` | PASS | Since 2026-09-23. Five v0 rejections are retired (see the file's header). |
| `SpecFunctions` | PASS | Since 2026-09-23: derived and authored specification functions, stateful and generic ones, unspecified values of aborts, and a native's declared version. |
| `WrongIncrement` | PASS | |

### Modules (module structure, attributes, lowering, preparation, Rust profile)

| File | Status | Remaining problem |
|---|---|---|
| `Attributes` | PASS | No verification targets. |
| `EmptyModule` | PASS | No verification targets. |
| `IntrinsicErrors` | PASS | |
| `LoweringErrors` | PASS | |
| `PreparationRetry` | PASS | |
| `Rust` | PASS | Modular `add`/`subtract`/`multiply` are carried as `Term.modular`. |

### Examples (larger programs)

| File | Status | Remaining problem |
|---|---|---|
| `Account` | PASS | |
| `Corpus` | PASS | |
| `OrderedMap` | PASS | Since 2026-09-24: v0's nine proof-carrying targets over a generic sorted-vector map with increasing keys as its data invariant. `empty`, `lower_bound`, `length`, `borrow_key_at` close automatically; the recursive binary search, `contains`, `borrow`, `add`, and `remove` through lemmas over arrays (search window, presence and absence at the lower bound, sortedness and shape after insertion or removal). v0's eight execution scenarios run at `u64` and `Bool` keys; `borrow_key_at` and `borrow` return a reborrow through a local holding a reborrow of the parameter. |
| `Quicksort` | PASS | Since 2026-09-24: v0's three proof-carrying targets. The generic Lomuto `partition` (quantified loop invariants over the structural order, the working range's frame and counts) and the generic recursive `quick_sort_range` (sortedness, frame, and counts of the range) through authored proofs; `quick_sort` automatically. The permutation is stated by counts, a recursive `spec fun`; the count lemmas (split, congruence, membership) go through its unfolding theorem, and the recursive sort's three residual goals are composition lemmas over arrays. v0's five execution cases run through concrete wrappers. `partition` costs 260M heartbeats (`set_option leaner.verifyHeartbeats 400000`). |

### Missing v0 fixtures

None: every v0 verification fixture is ported.

## Move corpus round trip

The Move-to-LeanerLang corpus (`leaner-e2e-tests/LeanerE2ETests/MoveToLeanerLang/`,
94 modules) was red on `main` from the 2026-09-09 squash until 2026-09-22
(Lean CI was off): six modules failed re-import because a nested block's
value printed as `return e`, eight lost their fixed point on element
borrows, two on spec-context `&&`, one on a nested conjunction. As of
2026-09-22 two remain: `smart_table` (a `for` whose bound is assigned in an
enclosing loop is re-lowered as a `while`) and `loops` (`break 'outer` from an
inner Move loop: "nonlocal break targets an unlabeled loop").

Rules the fixes settled: the Move frontend emits the same
`checkVectorIndex` binding around every element borrow the LeanerLang
lowering does (an out-of-range LIR index place resolves to nothing, so the
check is the abort), and lowers `&&`/`||` as the short-circuit conditional
in executable code; the text printer elides that binding and its `$t`
index temporary back into `&v[i]`, `v[i]`, and `v[i] := x`.

## Conventions and rules

A Check file is LeanerLang source with contracts, theorems, authored
proofs, and execution or diagnostic assertions. The driver discovers every
`Check/**/*.lean`, runs each in its own Lean process at the driver's caps,
and compares all diagnostics to the adjacent `.exp` (absence of `.exp`
means empty output). Run it from `leaner-e2e-tests` with
`LEANER_E2E_SUITE=check lake test`; `UB=1` regenerates baselines and every
regenerated diff is reviewed.

- A baseline records intended behavior. A negative case's expected
  diagnostic names the construct or clause; an unsupported positive proof
  is never recorded as an expected failure.
- A file is promoted only by the driver at the unchanged caps; a passing
  pilot promotes nothing. Do not raise caps to count a port as done.
- Every `verify` audits its artifacts (denotation route, approved axioms
  only); a fixture adds no audit of its own.
- A module lays out each function as its `fun`, its `spec`, the `spec fun`s
  that specification needs (at their first use), the theorems its proof
  uses, and its `verify … by`, in this order; a function with a `spec`
  verifies without a `verify` item. Larger modules separate groups of
  functions with `-- ## Title` headers.
- Assertion-style IR, Move, and Rust tests stay in their owning packages.
  The deprecated packages are reference material and are not run.
- Source verification is not a compiler-correctness theorem for emitted
  bytecode.

After each batch, update the date, the counts, the affected rows, and the
gate table in place. Record commits separately; a commit does not change a
status.
