# Test organization and check ledger

Updated 2026-09-08 (checkpoint of the denotation route,
[`denotation.md`](denotation.md)). This is the ledger of the acceptance
fixtures under
[`leaner-e2e-tests/LeanerE2ETests/Check/`](../leaner-e2e-tests/LeanerE2ETests/Check/):
which pass exactly, which do not, and why. Chronology and the mappings of
the original v0 cases live in
[the history archive](test-organization-history.md), not here.

## Current result

**35 of 61 Check files pass exactly; 26 fail.** Every failure is a
construct the denotation does not yet carry or a fixture that still asserts
names of a retired route; none is a diagnostics mismatch. A file passes
when its whole output matches the adjacent `.exp` at the driver's caps
(180k verification heartbeats per target); a file with one failing target
fails, however many targets it proves.

| Gate | Result | Note |
|---|---|---|
| `leaner-ir` build | PASS | `LeanerLang` and the `Denote` modules. |
| `DenotePerformance` gate | PASS | Loop targets within 6% of the baseline; no target regressed past its budget. |
| `leaner-ir` `lake test` | FAIL (38 of 215 roots) | 35 `LeanerLang.Tests.Native*` roots and `Performance` assert artifacts of the retired routes (their removal is D4); `Frontend` needs vectors (`replace`); `CompositionPerformance` is a resource-composition residual. Every `LeanerIR.Tests.*` root passes. |
| `leaner-move`, `leaner-rust` builds | PASS | |
| `leaner-e2e-tests` `lake build` | FAIL (unrelated) | The `mono-move-lean-link` Rust crate does not compile (E0061); the ledger was taken per file with `lake env lean` at the driver's caps. |

The remaining failure classes, by size:

| Class | Files | What is missing |
|---|---|---|
| Vectors | 11 | Vector types, element loans, and vector primitives are not carried. |
| Generics | 4 | Generic locals, calls, constructors, and fields are not carried. |
| Resource invariants and sequential resource effects | 5 | `GlobalInv`, `CrossInv`, `LooseFrame`, `ResourceComposition`, and `Language/Loops` (`drain`) leave residual obligations. |
| Recursion and unspecified callees | 2 | A callee is verified before its callers; recursion and pure helpers used as summaries are not carried. |
| Returned and free-standing references | 2 | A mutable borrow outside a binding or call argument, and returned references. |
| Retired-route assertions | 2 | `Verification/Typed` and `Verification/EnumRefs` assert artifacts of the retired route. |
| Rust profile | 1 | The Rust profile's primitives have no denotation yet. |

## Per-file status

Each name is a `.lean` file under `Check/`. PASS means the entire fixture
matches its baseline; a PASS with no `verify` target is execution or
diagnostics coverage only, as noted.

### Language (19 files: 13 pass, 6 fail)

| File | Status | Remaining problem |
|---|---|---|
| `Language/Abilities` | PASS | No verification targets. |
| `Language/Addresses` | PASS | |
| `Language/Arithmetic` | PASS | |
| `Language/Attributes` | PASS | No verification targets. |
| `Language/ControlForms` | FAIL | `index_arithmetic` has a vector local. |
| `Language/EmptyModule` | PASS | No verification targets. |
| `Language/EnumPatterns` | PASS | Nested enums verify at seven goals' cost. |
| `Language/EnumPayloads` | FAIL | Vector locals and vector callee parameters. |
| `Language/EnumRefs` | PASS | Execution only, no `verify`. |
| `Language/Enums` | PASS | |
| `Language/Generics` | FAIL | Every target has a generic local. |
| `Language/Integers` | PASS | |
| `Language/Literals` | FAIL | `classify_bytes` has a vector local. |
| `Language/Loops` | FAIL | `drain` loops over a live global borrow; residual obligation. |
| `Language/PositionalStructs` | PASS | |
| `Language/Signed` | PASS | |
| `Language/Tuples` | PASS | |
| `Language/VectorOperations` | PASS | |
| `Language/Vectors` | FAIL | Vector results, locals, and the `length` primitive. |

### Verification (30 files: 13 pass, 17 fail)

| File | Status | Remaining problem |
|---|---|---|
| `Verification/Aborts` | PASS | Includes the intended false-contract rejection. |
| `Verification/Account` | PASS | |
| `Verification/BorrowCertificates` | PASS | Certificate assertions, no `verify`. |
| `Verification/Callees` | FAIL | Unspecified pure callees (`plus_one`, `pure_predicate`) and recursion (`drain`). |
| `Verification/Calls` | FAIL | `recursive_choose` is recursive. |
| `Verification/Composition` | PASS | |
| `Verification/CorePrimitives` | FAIL | `vector_get`, `vector_set`. |
| `Verification/Corpus` | PASS | |
| `Verification/CrossInv` | FAIL | Cross-resource invariant leaves a residual obligation. |
| `Verification/EnumRefs` | FAIL | The fixture builds enum twins with anonymous constructors of the retired route. |
| `Verification/Generics` | FAIL | Generic locals and calls. |
| `Verification/GenericScalarCalls` | FAIL | Generic calls. |
| `Verification/GenericStorage` | FAIL | Generic fields have no carrier; generic calls. |
| `Verification/GlobalBorrows` | FAIL | `bump_first`, `bump_left` borrow vector elements; the rest passes. |
| `Verification/GlobalInv` | FAIL | Resource invariant: preparation times out at `whnf`. |
| `Verification/Increment` | PASS | |
| `Verification/Invariants` | PASS | |
| `Verification/Loans` | FAIL | `extend`, `independent_element`, `splice` have vector locals. |
| `Verification/LoopInvariants` | FAIL | `clear` mutates a vector in a loop; `count_to`, `sum_ones` pass. |
| `Verification/Loops` | PASS | |
| `Verification/LooseFrame` | FAIL | Resource-effect target leaves a residual obligation. |
| `Verification/Normalized` | PASS | |
| `Verification/Prophecies` | PASS | |
| `Verification/Read` | PASS | |
| `Verification/References` | FAIL | `reborrow` borrows outside a binding or call argument; returned references. |
| `Verification/ResourceComposition` | FAIL | Sequential resource writes leave a residual obligation. |
| `Verification/Rust` | FAIL | The Rust profile's `add` has no denotation. |
| `Verification/SpecLogicalArithmetic` | PASS | |
| `Verification/Storage` | PASS | |
| `Verification/Typed` | FAIL | Asserts `typedDenotation`/`Arguments` names of the retired route. |

### Negative and support (12 files: 9 pass, 3 fail)

A correct rejection does not make a file pass if its positive control fails.

| File | Status | Remaining problem |
|---|---|---|
| `Negative/BorrowGlobals` | PASS | |
| `Negative/Borrows` | PASS | |
| `Negative/IntrinsicUnsupported` | PASS | |
| `Negative/LoopInvariants` | PASS | Baseline names the unestablished invariant at entry and at an iteration. |
| `Negative/Lowering` | FAIL | Positive controls `receiver_get`, `two_reads`, `receiver_insert` have vector locals. |
| `Negative/ReturnedMutRefs` | FAIL | The parameter-derived returned reference leaves a residual obligation. |
| `Negative/Specifications` | PASS | |
| `Negative/Surface` | PASS | |
| `Negative/Verification` | PASS | |
| `Negative/WrongIncrement` | PASS | |
| `PreparationRetry` | PASS | |
| `VectorBounds` | FAIL | Every target has a vector local. |

### Missing v0 fixtures

| v0 file | Remaining work |
|---|---|
| `Verification/OrderedMap.lean` | Nine proof-carrying targets and their dependencies. |
| `Verification/Quicksort.lean` | Three proof-carrying targets and their dependencies. |
| `Verification/ReturnedMutRefs.lean` | The 66-target corpus; the hand `References` fixture does not replace it. |
| `Verification/SpecFunctions.lean` | Twelve targets. |
| `Verification/Summaries.lean` | Three targets. |
| `Negative/SpecFunctions.lean` | Diagnostic cases. |
| `Language/BorrowChecker.lean` | Borrow-checker cases. |

## Conventions and rules

A Check file is LeanerLang source with contracts, `verify` commands, and
execution or diagnostic assertions. The driver discovers every
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
- Successful verifies pass the automatic native audit; use
  `#leaner_require_native` for partial ports and
  `#leaner_require_native_all` for completed fixtures.
- Assertion-style IR, Move, and Rust tests stay in their owning packages.
  The deprecated packages are reference material and are not run.
- Source verification is not a compiler-correctness theorem for emitted
  bytecode.

After each batch, update the date, the counts, the affected rows, and the
gate table in place. Record commits separately; a commit does not change a
status.
