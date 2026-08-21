# Complexity of the verification encoding

Status: analysis with measurements from 2026-08-20; strategies 1–5 below
were then implemented (or, for 4, measured and rejected) — the outcome of
each is recorded under its heading

This note examines the relational semantics that `spec`/`verify` prove
against, along two axes — how large proofs are and how long verification
takes — and derives optimization strategies from where the costs actually
come from.  The measurements were taken on the state of the tree at the end
of the data-invariant work (88 test jobs).

## The encoding

A source function is interpreted as a `Spec State Result`:

```lean
structure Spec (State Result : Type) where
  ok        : State → Result → State → Prop     -- successful executions
  aborts    : State → Nat → Prop                -- abort outcomes (effects rolled back)
  undefined : State → Prop := fun _ => False    -- proof owed but never checked at run time
```

Sequencing is relational composition — an existential over the
intermediate value and state:

```lean
bind a f:
  ok        s r s'  := ∃ v m, a.ok s v m ∧ (f v).ok m r s'
  aborts    s c     := a.aborts s c ∨ ∃ v m, a.ok s v m ∧ (f v).aborts m c
  undefined s       := a.undefined s ∨ ∃ v m, a.ok s v m ∧ (f v).undefined m
```

A mutable borrow is a RustHorn-style prophecy — an existential over the
future value that is reconciled when the loan dies — and a global borrow
checks a typed `ResourceStore` out and back in around the body.  Recursion
is the union of finite unfoldings (`fix` = `∃ fuel, fixApprox body fuel`).

A contract is three obligations over that relation, and `wp` mirrors them:

```lean
Satisfies f C := ∀ args s, C.requires args s →
    (∀ r s', (f args).ok s r s' → (¬C.mayAbort args s → C.ensures args s r s') ∧ C.frame args s s')
  ∧ (∀ c, (f args).aborts s c → C.aborts args s c)
  ∧ ¬(f args).undefined s
```

### What that costs, structurally

| Construct | Term after unfolding | Proof-side effect |
|---|---|---|
| `bind` (n in sequence) | linear in n, but the prefix's `ok` appears **once in `ok`, once in `aborts`, once in `undefined`** | three obligations per function, each traversing the body |
| `withMutation` | `∃ future, …` plus a reconciliation equation | one `∀ future` per mutable borrow, and one `output.current = future` to substitute |
| `withBorrowMutSpec` | `∃ value bodyWorld finalValue, lookup = some value ∧ …` | one lookup hypothesis per global borrow |
| checked op (`+`, `&v[i]`, insert, …) | a two-branch relation: success condition / abort with a code | one case split per operation (`checked_cases`) |
| `fix` | `∃ fuel` over a fuel-indexed approximation | fixed-point induction (`satisfies_fix_of_wp`) |
| `ite`/`match` | kept as is | one split per conditional (`move_cases`/`split`) |

Two consequences follow directly from the definitions:

1. **Unfolding the relations is the wrong basis for proof.**  Every `bind`
   introduces two existentials that have to be eliminated, and `aborts`
   re-states the prefix's `ok` relation.  The weakest-precondition rules
   (`wp_bind`, `wp_pure`, the per-primitive `wp_*`) eliminate all of that
   structurally: `wp (bind a f) ens abt s ↔ wp a (fun v m => wp (f v) ens abt m) abt s`
   has no existential, mentions each sub-term once, and is linear in the body.
   The manual proofs (`contract_intro` + `wp_norm`) already work this way; the
   automatic `verify` still unfolds the raw relations and lets `simp_all` and
   `grind` eliminate the existentials.
2. **Obligation count multiplies traversal count.**  `Satisfies` mentions
   `f args` three times, so anything that unfolds the body before splitting
   the conjunction does three traversals — or, after the shared-unfold
   fix, one traversal of a term three copies large.  Well-definedness is the
   odd one out: it is *structural* (every primitive is total; only
   `Spec.certified` owes anything), so it should never be unfolded at all.

## Where the time goes

Profiling the automatic `verify` (`set_option profiler true`) on the
resource-heavy `ResourceComposition.shift` — two global borrows, two checked
operations — gives, after the four fixes below:

| phase | before the fixes | after |
|---|---|---|
| hoisted `simp only [sourceSpec, move_spec]` (unfold + normalize) | ~1.0s | ~0.09s |
| `simp_all` per obligation (×2) | ~0.15–0.25s each | ~0.15–0.23s each |
| `grind` | ~20–30ms | ~20ms |
| whole generated theorem | ~2.4s | ~0.7s |

After the fixes the remaining cost is the two `simp_all` passes with the
default simp set — which is the argument for strategies 1 and 4 below.
Import of the `Move` library costs a further ~1.2s and ~1.5 GB of RSS per
file before any proof runs; that floor is now larger than the proofs
themselves for every file in the corpus, and it is why the suite's wall
clock (88 jobs) barely moves when individual files get faster.

Four pathologies were found and fixed along the way; they are worth
recording because each could recur:

- **Duplicate unfolding.**  `constructor <;> simp_all [move_spec, …]`
  unfolded the whole semantics once per obligation.  Hoisting one
  `simp only [sourceSpec-unfolds, move_spec]` before the split:
  `ResourceComposition` 36.7s → 1.1s, the suite 93s → 27s.
- **A wildcard simp lemma.**  `UInt.numeral_eq_ofNat` is stated with
  `no_index (OfNat.ofNat n)`, so its discrimination-tree key is a wildcard:
  simp retried it 483,740 times on one file, never once successfully.  It is
  nonetheless load-bearing (it proves `UInt.ofNat 3 = 3`); after the hoist it
  costs ~200ms, so it stays.
- **Exponential unfolding of `bind`.**  `move_spec` used to contain the
  *definitions* of `Spec.bind`, `withMutation`, and `withBorrowMutSpec`.
  Unfolding `bind a f` produces a structure literal whose `ok`, `aborts`, and
  `undefined` fields each contain the continuation `f`; simp normalizes all
  three bottom-up before the projection discards two, so each level of a
  bind chain multiplies the work.  Diagnostics on a chain of four `x + 1`:
  `Spec.bind` unfolded 256 times, `Spec.ok` reduced 14,296 times, and simp hit
  its step limit — the automatic `verify` was silently failing for any
  straight-line sequence of four or more checked operations.  The fix is
  projection lemmas (`bind_ok`, `bind_aborts`, `bind_undefined`,
  `withMutation_ok/aborts/undefined`, `withBorrowMutSpec_ok/aborts/undefined`,
  `certified_ok/aborts`, and `ok_ite`/`aborts_ite`/`undefined_ite` so a
  projection distributes over a source conditional) in `move_spec` instead of
  the definitions: only the field actually asked for is ever produced.
- **Blind `apply` against a fixed point.**  A discharger of the form
  `repeat' first | apply L₁ | apply L₂ | …` on `¬(Spec.fix body args).undefined s`
  made every failed `apply` whnf the fuel-indexed body: 15 minutes and
  16.7 GB on `Loops.lean` before it was killed — with `maxHeartbeats` set so
  high the failure never fired.  The replacement (`spec_defined`) reads the
  head symbol of the goal and applies exactly one lemma, with `fix_defined`
  (induction on fuel) for recursion, and fails immediately naming any head it
  does not know.  `Loops` now takes 1.6s, and the generated theorem's
  heartbeat cap is back to a value where a runaway tactic fails in seconds.

## Where the proof text goes

The corpus has 28 hand-written `verify … by` proofs totalling 672 lines and
78 automatic ones.  The five largest (all in `OrderedMap` and `Quicksort`)
account for 430 of the 672.  Counting recognisable shapes across all manual
proofs:

| shape | lines |
|---|---|
| frame / guard plumbing (`⟨?_, trivial⟩`, `, rfl⟩`) | 35 |
| `rw [if_pos h]` / `rw [if_neg h]` after a split | 32 |
| `omega` | 29 |
| `wp_call` / `wp_of_satisfies` | 23 |
| `by_cases` / `move_cases` | 19 |
| `spec_norm` | 19 |
| `simp only [move_norm, …]` | 15 |
| `checked_cases` / `abort_clause` | 11 |

Two observations.  First, the largest single bucket is plumbing that states
nothing about the program: the frame conjunct (`trivial`/`rfl`), the
`¬mayAbort` guard binders, and reconciliation `rfl`s.  Second, the
`by_cases … rw [if_pos]/[if_neg]` pair — nineteen splits, thirty-two
rewrites — is pure symbolic execution of a source conditional, exactly what
a tactic should do.

## Optimization strategies

Ordered by expected payoff over cost.  The first three attack the dominant
costs identified above; the rest are smaller or speculative.

### 1. Make the automatic `verify` wp-based — done

Replace "unfold the relations, then `simp_all` + `grind`" by
`contract_intro` followed by `simp only [wp_norm]` (the per-primitive
`wp_*` rules), then the existing arithmetic finisher.  Effects:

- the body is traversed once, by rewriting, with no existentials to
  eliminate and no `ok` duplication — the `aborts` half comes out of the
  same rule applications as the `ok` half;
- `Satisfies` need never be unfolded: `satisfies_of_wp` / `satisfies_fix_of_wp`
  hand over a single `wp` goal, so the "three copies of the body" problem
  disappears structurally rather than by tactic arrangement;
- the automatic and manual paths share one proof engine, so improvements to
  `wp_norm` benefit both.

Risk: every primitive the translator emits needs a `wp_norm` rule with a
discharger-friendly right-hand side.  Most existed already (`WP.lean`,
`Borrow.lean`); the gaps were `wp_ite`/`wp_dite` (a source conditional under
`wp` splits) and three `Mutation` projection lemmas.  Callees are inlined as
before (their `sourceSpec`s unfolded into the caller), so behavior is
unchanged; modular use of `callee.verified` is a natural follow-up.

*Outcome.*  Automatic-verify elaboration per file fell to 0.15–0.28s
(`ResourceComposition` 0.7s → 0.24s, `Invariants` ~1.3s → 0.2s); the
scaling probes went from 1.23s/2.08s at eight chained operations to
0.19s/0.38s and are now close to linear.  `contract_intro` had to learn the
difference between a function that *is* a fixed point and one whose body
merely contains a loop (`fun n => Spec.fix loop ()`), which the old
head-symbol test conflated.

### 2. Keep well-definedness structural everywhere — subsumed by 1

With the automatic `verify` on wp rules, well-definedness never appears as
a separate goal at all: every primitive rule is total and discharges its own
conjunct, and `wp_certified` surfaces the one genuine obligation as
`Invariant … ∧ ∀ holds, …` inline, where the value is created.
`spec_defined` remains for proofs that open `Satisfies` by hand.

### 3. A `move_step` tactic for manual proofs — done

One tactic that does one symbolic step and leaves the user with the real
question, choosing by the shape of the goal: a source conditional is split
and its hypothesis named and normalized (`move_step inBounds`); a checked
operation is split into its success branch, with binders named
(`move_step entry atTarget`), and its abort branch, discharged against the
declared clauses; a recursive call is stepped through the contract being
established; otherwise the plumbing a rule leaves behind — `¬mayAbort`
guards, a trivial frame, reconciliation equations — is cleared.  Each step
rewrites only the leading construct, so the proof keeps the shape of the
source.  `move_hyp h` is the hypothesis normalizer on its own.

*Outcome.*  `OrderedMap.contains` 58 → 39 lines and `borrow` 63 → 43
(−32% and −31%) with no change in what the proofs say; the rest of the
corpus is unconverted and is where the remaining quarter lives.

### 4. Trim the simp sets to what fires — measured, partly rejected

The expensive phase was the unfolding simp, and strategy 1 replaced it by a
`simp only [wp_norm, …]` pass with a curated list — that part of this
strategy is done.  The remaining finisher (`simp_all` with the default set,
then `grind`) was then measured against a curated `simp_all only` built from
the 135 default-set lemmas diagnostics showed firing across six files: the
curated variant was 2–4× *slower* (`Account` 0.15s → 0.51s) and broke 21
proofs, because the default set's simprocs and rfl-lemmas matter beyond the
named theorems.  The finisher keeps the default set.  The symbolic-execution
inventory was split into `move_data` (data-level unfolds: references,
stores, vectors, monad shells) and `move_spec` (relational projections plus
`move_data`), so the wp phase can use the former alone.

### 5. Eliminate prophecies that do not escape — done

`wp_withMutation` quantifies over the future and the reconciliation
equation then fixes it.  Rather than special rules per body shape, three
simp lemmas eliminate the quantifier the moment the equation appears under
the hypotheses a rule puts in front of it — `forall_imp_eq_left`
(`(∀ x, A → b = x → P x) ↔ (A → P b)`) and its two siblings in
`Contract.lean` — and they are part of the wp phase.  On `Account.deposit`
the post-wp goal has no `∀ future` left.

### 6. Cut the import floor

~1.3s and ~1.5 GB per file is the cost of `import Move`, which pulls in the
whole `MoveModel` IR formalization (197 MB of `.olean`) although source
verification needs only `Move.Semantics` and `Move.Verify`.  Splitting the
library so that tests `import Move.Verify` (and only the compiler-facing
tests `import Move`) would lower every number in this note by a constant,
and is the only lever on memory.

### 7. Not recommended

- *Abort-encoding of well-definedness* (make a violated invariant an abort
  with a distinguished code).  Cheap, but unsound together with
  uninterpreted aborts.
- *Quantifying well-definedness over all states* (`Spec.Total` instead of
  the pointwise form).  It loses the reachability hypothesis and would make
  `OrderedMap.add`'s re-established invariant unprovable.

## Benchmarking proof cost

The suite wall is not a proof-cost metric (above).  To measure proof work
directly there is `scripts/bench-proofs.sh`, built on a deterministic figure
rather than wall time.

Every generated and hand-written `verify` proof is wrapped in a `move_bench`
tactic that is **inert unless** the environment variable `MOVE_PROOF_BENCH`
is set — so it costs nothing in normal builds and needs no per-proof
annotation.  With the variable set, each proof logs

```
‖MOVE_BENCH‖  <function>.verified  <heartbeats>  <elapsed-ms>
```

Heartbeats count elaboration work and are independent of the machine, the
load, and the `aptos` CLI the suite otherwise spends its wall time in.  They
are stable to about 0.01% run to run (a little jitter from hashmap iteration
inside `grind`), which is far below any regression worth catching; wall-ms,
by contrast, swings 2–4× per proof and is reported only as a hint.

```
scripts/bench-proofs.sh [top-N]     # rebuilds the suite with benchmarking on
```

Whole-suite result (2026-08-21, after strategies 1–5): **110 verified
functions, 99.7M heartbeats, 5.4s of proof wall in total** — i.e. the actual
verification work is a small fraction of the ~40s suite wall, the rest being
the CLI and scheduling.  The cost concentrates in the recursive and loop
proofs and the ordered-map operations:

| heartbeats | function |
|---|---|
| 10.7M | `OrderedMap.lowerBoundLoop` (recursive binary search) |
| 6.8M | `ResourceComposition.shift` |
| 5.7M | `OrderedMap.add` |
| 5.3M | `OrderedMap.borrow` |
| 4.9M | `Quicksort.partitionLoop` |
| 3.8M | `Vectors.removeMiddle` |

This is the reference point for future changes: re-run the script and diff
the per-proof heartbeats.

### The suite wall sees none of this (function-by-function, 2026-08-21)

Benchmarking the previous commit (`fce58bd548`, HEAD~1) against the current
working tree, per verified function, shows proofs got **3.9x cheaper** —
328.6M -> 84.8M heartbeats across the 60 common functions.  The
automatic-`verify` proofs dropped 3-30x (strategies 1-5); the hand-written
proofs are unchanged (~1.0x, a control group that also validates the metric);
`OrderedMap.borrow`/`contains` rose ~3x, the deliberate `move_step`
conversion trading heartbeats for a 31% line reduction.

Yet **both commits produce a 40s suite wall.**  A 4x change in total proof
work is invisible in the suite time, which is set by the `aptos` CLI and
build scheduling.  (The ~20-27s figure remembered from mid-session was a
transient *uncommitted* state after the duplicate-unfolding fix and before
data invariants; it is not recoverable as a commit, and both neighbouring
commits are ~40s.)  Measure proofs with `scripts/bench-proofs.sh`, never the
suite wall.

## Measurements

All numbers are wall-clock on an otherwise idle machine; `lake env lean` on a
single file includes ~1.2s of import before any proof runs.

### Scaling with body length

`localₙ` is `n` sequential `let vᵢ := vᵢ₋₁ + 1` on a `U64` argument;
`resₙ` is `n` sequential `value := *value + 1` through `&mut Counter[addr]`.
Each is verified by the automatic `verify` with a contract stating the sum
and the overflow abort.  "elab" is the profiler's elaboration time for the
generated theorem.

| n | local, before | local, after | resource, before | resource, after |
|---|---|---|---|---|
| 1 | 0.17s | 0.09s | 0.74s | 0.26s |
| 2 | 0.48s | 0.26s | 2.28s | 0.41s |
| 4 | *fails* (simp step limit) | 0.49s | *fails* | 0.81s |
| 8 | *fails* | 1.23s | *fails* | 2.08s |
| 16 | *fails* | *fails* (grind) | *fails* | *fails* (grind) |

"before" is with the combinator definitions in `move_spec`; "after" with the
projection lemmas.  The before-column is exponential and hits simp's
default step limit at n = 4.  The after-column is superlinear but
polynomial — roughly ×2.5 per doubling, i.e. about n^1.3 — and is dominated
by `simp_all` re-simplifying an `n`-long chain of equation hypotheses
against each other, plus the default simp set being tried on every subterm
(`UInt.numeral_eq_ofNat` 18,529 tries for 16 successes at n = 16).  At
n = 16 the arithmetic finisher (`grind`) no longer closes the 16-deep
`toNat (ofNat (… + 1))` normalization; no real program in the corpus comes
close to that shape, but it marks the limit of the unfold-and-normalize
design, and strategy 1 is how to move it.

### Real files

Per file, `lake env lean`, projection lemmas in place:

| file | wall | before projection lemmas |
|---|---|---|
| Read | 1.2s | |
| Arithmetic | 1.6s | |
| Account | 1.6s | 2.5s |
| Loops | 1.6s | 1.6s (15 min before the dispatcher) |
| Calls | 1.7s | 2.1s |
| ResourceComposition | 1.8s | 3.2s |
| Quicksort | 1.8s | 1.8s |
| Vectors | 2.0s | 2.7s |
| OrderedMap | 2.4s | 1.9s |

`OrderedMap` is almost entirely hand-written wp proofs and is unaffected
either way; the automatic-verify files shrink by a third to a half.

### After strategies 1–5

Per file (`lake env lean`, ~1.1–1.2s of import included): `Read` 1.5s,
`Arithmetic` 1.3s, `Account` 1.3s, `Loops` 1.6s, `Calls` 1.5s,
`ResourceComposition` 1.5s, `Quicksort` 2.0s, `Vectors` 1.4s,
`VectorOperations` 1.5s, `OrderedMap` 2.0s, `Invariants` 1.4s,
`Integers` 1.5s — i.e. 0.2–0.8s of proof per file.  Scaling probes (elab):
local chain 0.02/0.06/0.09/0.19/0.63s and resource chain
0.07/0.13/0.20/0.38/0.99s for n = 1/2/4/8/16 (n = 16 still fails only in
`grind`'s arithmetic).

### Suite

Cold `lake test`, 88 jobs, wall clock: 89s at the pre-session baseline
(`d72ab94ec7`, before any of this session's fixes); 27s at the mid-session
low after the duplicate-unfolding fix; 44s at the worst point after adding
well-definedness; 40s now.

**This wall is a poor proxy for verification cost and should not be read as
one.** Measured 2026-08-21: the 39 timed jobs sum to 75s of CPU but the wall
is 40s — the suite runs at only ~1.9x effective parallelism (peak 6
concurrent `lean` workers), and 25 of the 40 test files shell out to the
external `aptos` CLI during elaboration to compile Move (peak 11 concurrent
`aptos` processes).  The CLI compilation and the limited build parallelism
dominate the wall; the verification proofs the rest of this note is about are
0.2–0.9s of elaboration each.  The import floor is unchanged from the
baseline (both 1.07–1.23s for `import Move`), so imports explain neither the
baseline number nor any change over the session.  The 27→40 movement is one
added CLI file (`Invariants`), a grown `OrderedMap`, and CLI/scheduling
noise — not per-proof regression.  To measure verification specifically, time
individual files' elaboration (`set_option profiler true`) or exclude the
CLI-compiling files; strategy 6 lowers the per-file floor but not this wall.
