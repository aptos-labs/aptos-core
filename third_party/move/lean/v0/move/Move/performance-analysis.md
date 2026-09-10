# Complexity of the verification encoding

Status: current as of 2026-08-22.  Every optimization considered is listed
with an explicit verdict — done, rejected, or open — in the status table under
*Optimization strategies*; the outcome of each is recorded under its heading,
and the numbers behind the verdicts are under *Benchmarking proof cost*.

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

Status of every optimization considered, so that a later reader can tell what
was tried from what merely sounded good.  Detail follows in the numbered
sections; the heartbeat numbers behind the verdicts are under *Benchmarking
proof cost*.

| # | strategy | status |
|---|---|---|
| 1 | Make the automatic `verify` wp-based | **done** |
| 2 | Keep well-definedness structural everywhere | **done** (subsumed by 1) |
| 3 | A `move_step` tactic for manual proofs | **done**; corpus conversion **open** |
| 4 | Trim the simp sets to what fires | **partly done, partly rejected** |
| 5 | Eliminate prophecies that do not escape | **done** — but not by the lemmas it added |
| 6 | Cut the import floor | **open** — never attempted |
| 7 | Abort-encoding / `Spec.Total` well-definedness | **rejected** (unsound / too weak) |
| 8 | Per-view interface over the generic integer core | **done** |
| 9 | Fix `@[simp high]` inside custom simp sets | **done** |
| 10 | Remove lemmas the audit shows never fire | **done** |
| 11 | Make the two spellings of a value share one key | **done** |
| 12 | `data_invariants` as a tactic, not on the automatic path | **done** (folding it in: **rejected**) |
| 13 | Reduce `MoveInt S W` proof-term size | **open** — the whole residual |
| 14 | Stop `uint_bounds` re-scanning the context per call | **open** — unmeasured |

1–7 were derived from the cost analysis above and are ordered by expected
payoff over cost; 8–12 came out of the integer unification and were driven by
per-proof heartbeat measurement rather than by inspection; 13–14 are what is
left.

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

### 5. Eliminate prophecies that do not escape — done, by something else

`wp_withMutation` quantifies over the future and the reconciliation
equation then fixes it.  Rather than special rules per body shape, three
simp lemmas eliminate the quantifier the moment the equation appears under
the hypotheses a rule puts in front of it — `forall_imp_eq_left`
(`(∀ x, A → b = x → P x) ↔ (A → P b)`) and its two siblings in
`Contract.lean` — and they are part of the wp phase.  On `Account.deposit`
the post-wp goal has no `∀ future` left.

*Correction, from the audit below.*  The goal is met — no `∀ future` survives
— but **not by these three lemmas**: across the corpus they were tried 1651
times each and applied exactly zero times.  Something earlier in the wp phase
already eliminates the quantifier.  They are out of the explicit list (still
`@[simp]`, so a regression cannot hide) and the corpus got 4–8% cheaper.

### 6. Cut the import floor — open

~1.3s and ~1.5 GB per file is the cost of `import Move`, which pulls in the
whole `MoveModel` IR formalization (197 MB of `.olean`) although source
verification needs only `Move.Semantics` and `Move.Verify`.  Splitting the
library so that tests `import Move.Verify` (and only the compiler-facing
tests `import Move`) would lower every number in this note by a constant,
and is the only lever on memory.

### 7. Not recommended — rejected

- *Abort-encoding of well-definedness* (make a violated invariant an abort
  with a distinguished code).  Cheap, but unsound together with
  uninterpreted aborts.
- *Quantifying well-definedness over all states* (`Spec.Total` instead of
  the pointwise form).  It loses the reachability hypothesis and would make
  `OrderedMap.add`'s re-established invariant unprovable.

### 8–12. The unification-era optimizations

Each has its own section below with the measurements; in short:

- **8, per-view interface** (`unified-int-design.md`): state every user-visible
  obligation in the view's native domain (`Nat` unsigned, `Int` signed) while
  keeping one generic core.  1.184× → 1.014×.  **Done.**
- **9, `@[simp high]` in a custom set** — the priority does not carry; use
  `attribute [move_norm high]`.  8× on one proof.  **Done.**
- **10, audit-driven removals** — 88% of all simp tries in the corpus fail;
  two inventories accounted for 6,739 wasted tries with zero successes.
  1.014× → 0.922×.  **Done.**
- **11, two spellings of one value** — numeral vs `UInt.ofNat`, and the
  `U8.ofNat …` abbrevs.  **Done**; two orientations of the fix **rejected** by
  measurement (see the section for why neither can be keyed).
- **12, `data_invariants`** — the tactic is **done**; folding it into
  `uint_bounds`, which is the obvious completion and reads better, is
  **rejected**: +522K net across the suite.

### 13. Reduce `MoveInt S W` proof-term size — open

This is what the entire residual now is.  Nothing above baseline is worse than
1.14×, and the largest absolute residues are hand-written loop proofs where no
rewrite fails to fire — the cost is kernel-side, in proof terms that carry two
class parameters and `numTypeOf S W` structure terms where `UInt W` carried
one.  It showed up as a consistent +9–12% in the profiler's `type checking`
when the unification landed.  Untried levers: making `numTypeOf` reduce to a
literal `NumType` at each concrete width so the terms close up, or `@[irreducible]`
on the parts of `MoveInt` the kernel need not see through.

### 14. Stop `uint_bounds` re-scanning the context — open, unmeasured

`spec_norm` runs `uint_bounds` and `u64_omega` runs it again inside a `first`
combinator, so a proof like `OrderedMap.lowerBoundLoop` scans the whole local
context and asserts bounds five or more times, each pass over a context the
previous one grew.  Nobody has measured what that costs; it is listed so the
next person does not have to rediscover it.

## A custom simp-attribute does not inherit `@[simp high]`

Worth its own heading because it is invisible and it cost 8× on one proof.

A lemma declared `@[simp high]` and separately registered in one of the
inventories (`attribute [move_norm] X`, or `@[simp high, wp_norm]`) is high
priority **in `simp` only**.  Inside `simp only [move_norm]` it sits at default
priority, so a more general lemma can beat it.

`UInt.toNat_ofNat_land` collapses `(ofNat (a &&& b)).toNat` to `a &&& b`
outright; the general `UInt.toNat_ofNat` rewrites it to `(a &&& b) % size`.
The collapse lemma has carried `@[simp high]` since it was written, but in
`move_norm` the general lemma won, so every bitwise proof was left with a
`% size` residue — dischargeable only by arithmetic that does not model
bitwise operations, which is why `Nat.and_le_left` had to be handed to `grind`
and why `grind`'s AC/ring machinery showed up in the profile of a file that
only masks two integers.  `Integers.masked` cost 1.99M heartbeats instead of
247K.

The fix is to state the priority in the set that will be used —
`attribute [move_norm high] …`, `@[simp high, wp_norm high]`.  With it, the
first `simp only` phase closes those goals, the `Nat.*` bitwise lemmas come
back out of the `grind` list, and `Tests/Language/Integers` drops to 0.80× of its
own pre-unification cost.

Symptom to recognise: `simp only [<attr>]` leaves a goal that
`simp only [<the one lemma>]` closes — and passing both explicitly makes the
linter report the specific lemma as unused.

## Auditing which lemmas actually fire

`scripts/simp-audit.sh` answers "is this lemma earning its place?" directly.
Lean's `diagnostics` option makes `simp` report, per call, every theorem it
*tried* and how often it *succeeded*, flagging with ❌️ any that were tried and
never applied:

```
scripts/simp-audit.sh [file ...]     # default: Move/Tests/**/*.lean
```

It aggregates those counts across files and prints the theorems that never fire
(pure cost — their discrimination key matches terms they cannot rewrite, or a
higher-priority lemma always wins) and the worst hit rates among those that do.

Two things it found immediately, both in inventories that had been carried for
a long time:

- `UInt.numeral_eq_ofNat` — the wildcard-keyed lemma this note already flags —
  was tried **1786 times with zero successes** across three files.  It is
  genuinely load-bearing, but only in the *fallback* path for functions with no
  `sourceSpec`; in the `wp`-based path it never fires.  Removing it from that
  path alone moved `GlobalInv` 1.04× → 0.94×, `Account` 1.07× → 0.97×,
  `Arithmetic` 1.01× → 0.93× and `Integers` 0.80× → 0.75× of pre-unification
  cost.  Removing it from the fallback path too breaks four files — which is
  exactly the distinction the audit makes visible and inspection does not.
- The three prophecy-elimination lemmas of strategy 5
  (`forall_imp_eq_left`, `forall_imp_eq_right`, `forall_imp_imp_eq_left`) were
  tried **1651 times each — 4953 tries, 59% of all wasted work in the corpus —
  and applied exactly zero times**.  They were carried in the explicit `wp`
  list; whatever eliminates the prophecy quantifier today, it is not them.
  Dropping them from that list (they remain `@[simp]`, so nothing can regress
  silently) moved `Account` 0.97× → 0.92×, `VectorOperations` → 0.86×,
  `GlobalInv` → 0.87×, `Vectors` → 0.93×.

A caution about the tool itself: Lean prints *two* blocks per `simp` call, a
"used theorems" summary and the "tried theorems" detail, and the first is a
subset of the second.  Counting both inflates every try total and invents
never-applied entries — the first version of this script did exactly that and
reported `IntWidth.size` as dead when it fires constantly.  Only lines carrying
❌️ or `succeeded:` are real.

The lesson is that a lemma's presence in an inventory is not evidence that it
is used, and "it is load-bearing" is not evidence that it is load-bearing *in
every path that pays for it*.

## Where two spellings of one value meet

A recurring shape behind several of the measurements above: the *same value* is
written two ways, so its discrimination-tree key splits and a lemma that should
fire does not.  Three instances, all found by benchmarking a proof that looked
like it should already be cheap.

**Numerals versus `ofNat`.**  The unsigned view lemmas (`add_eq_ofNat` and
friends) produce results as `Move.UInt.ofNat` of a natural-number expression;
specifications are written with numerals.  When the expression evaluates to a
literal the two meet, and every concrete postcondition — `ensures result = 27`
— is left one defeq step short.  Neither orientation works as a simp lemma:

- numeral-to-`ofNat` *must* key on `no_index (OfNat.ofNat n)`, because the
  discrimination tree collapses numerals to literal keys, so a precisely keyed
  pattern never matches at all (measured: a simproc keyed on the `MoveInt`
  instance of `OfNat.ofNat` is never invoked);
- `ofNat`-to-numeral read as a rewrite rule would also strip the `ofNat` shape
  off the compound `ofNat (a.toNat + b.toNat)` results the collapse lemmas and
  the `grind` patterns are keyed on.

Restricting to *literal* arguments is the distinction a rewrite rule cannot
make and a simproc can, so `uintOfNatLit` in `Move/Verify/Tactics.lean` does
exactly that, keyed on the `UInt.ofNat` head constant.  It settles the
canonical form — once the argument is a literal all the arithmetic is done, so
the value is the numeral — and lets two hand proofs drop their closing steps:
`VectorOperations.mutateAndRead` lost a six-lemma unfolding into raw `Int`
modular arithmetic plus a `decide` (884K → 831K heartbeats) and
`Vectors.removeMiddle` lost three trailing tactic lines (3.85M → 3.63M).

There is no companion simproc for `toNat` of a numeral, and this is not an
oversight: a probe simproc keyed on `MoveInt.toNat` fires for a variable
argument and never for a numeral one, because the tree evaluates the ground
term to a literal key.  `toNat_ofNat_numeral` therefore keeps its `no_index`
key; it is a wildcard, but a load-bearing one, and five proofs break without
it.

**Width-directed spellings.**  `U8.ofNat … U256.ofNat` and `I8.ofInt …
I256.ofInt` are named specification surface for the one `UInt.ofNat` /
`SInt.ofInt`.  They are `abbrev`s, and simp's discrimination tree does *not*
see through them, so which spelling a spec happened to use decided whether a
view lemma fired.  They are now unfolded in `move_norm`, next to the `U8.size …
U256.size` entries that were already there for the same reason.  Cost measured
at 8 heartbeats on a file that does not use them.

**A data invariant is not a width bound.**  `uint_bounds` asserts the certified
facts of every integer- and vector-typed local.  Extending it to also assert the
data invariant of every certified-typed local is the obvious completion — "the
invariant is available wherever the value is" — and it works: `Invariants.span`
drops its three-line prologue naming `Range.Invariant` and goes 590K → 305K,
**0.63× of its pre-unification cost**, because the proof can then use the
per-view `wp` rule instead of unfolding the raw relational `subSpec`.

Measured across the suite, though, it is a net **+522K**: a width bound is one
cheap atomic fact, while a data invariant can be an arbitrarily large predicate
— the ordered map's is a sortedness condition over the whole entry list — and
asserting one into every context the automatic cascade normalizes cost
`OrderedMap` +801K against span's −285K.  So it lives in its own tactic,
`data_invariants`, that a proof asks for.  The general principle survives; what
does not survive measurement is putting it on the automatic path.

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

The 2026-08-22 categorized-suite run covers **217 verified functions** at
**226.6765M heartbeats** (11.738s aggregate proof wall on the measurement
host). The largest proof remains `OrderedMap.lower_bound_loop` at 10.895M.
The revision-comparison figures below use the older common-115 corpus and are
retained as historical apples-to-apples measurements; they should not be
compared directly with the expanded whole-suite total.

Two historical comparison figures, and they are not interchangeable:

- **whole suite, 121 verified functions: 111.4M heartbeats** (~6s of proof wall
  in total — the actual verification work is a small fraction of the ~40s suite
  wall, the rest being the CLI and scheduling);
- **the 115 functions that exist in every revision: 107.5M.**  The other six are
  the new signed-integer proofs, which have no counterpart before the
  unification, so only this subset can be compared against history.

**Lower is faster** — a heartbeat is a unit of elaboration work.  The
trajectory of the comparable subset, oldest first, so the *last* row is where
the tree stands today:

| revision | common-115 heartbeats | vs. pre-unification |
|---|---|---|
| **before this work** — separate `UInt`/`SInt` op families | **117.0M** | 1.000× |
| after the integer unification, generic interface only | 138.6M | 1.184× |
| + per-view (`Nat`) interface for unsigned proofs | 127.3M | 1.087× |
| + attribute-priority fix (`move_norm high`) | 118.7M | 1.014× |
| + never-applied-lemma removals (audit below) | 107.9M | 0.922× |
| **now** — + the two-spellings fixes (section below) | **107.5M** | **0.918×** |

So the unification cost 18.4% when it landed and is now 8.2% *below* where the
tree started: **117.0M → 107.5M**, one generic core instead of two families,
covering twelve integer types where there were six.  The cost still concentrates in the
recursive and loop proofs and the ordered-map operations:

| heartbeats | function | vs. pre-unification |
|---|---|---|
| 10.9M | `OrderedMap.lowerBoundLoop` (recursive binary search) | 1.02× |
| 8.8M | `CrossInv.shift` | 0.86× |
| 6.3M | `ResourceComposition.shift` | 0.87× |
| 5.8M | `OrderedMap.add` | 1.00× |
| 5.1M | `Quicksort.partitionLoop` | 1.04× |
| 5.1M | `OrderedMap.borrow` | 0.95× |

What is left above baseline is diffuse: no function is worse than 1.14×, the
two largest absolute residues are `OrderedMap.lowerBoundLoop` (+213K) and
`Quicksort.partitionLoop` (+177K), and both are hand-written loop proofs whose
cost sits in larger `MoveInt S W` proof terms rather than in any rewrite that
fails to fire.  That is the kernel-side price of the extra abstraction layer,
recorded here as +9–12% in `type checking` when the unification landed; it is
not something a simp lemma can recover (strategy 13).

Per file, current against the pre-unification baseline — the unit of comparison
a change is most likely to move:

| file | pre-unification | now | ratio |
|---|---|---|---|
| `OrderedMap` | 28.59M | 28.39M | 0.99× |
| `CrossInv` | 10.23M | 8.82M | 0.86× |
| `VectorOperations` | 9.37M | 8.01M | 0.85× |
| `Quicksort` | 8.41M | 8.58M | 1.02× |
| `Vectors` | 7.85M | 7.10M | 0.90× |
| `Arithmetic` | 7.27M | 6.34M | 0.87× |
| `ResourceComposition` | 7.20M | 6.29M | 0.87× |
| `Calls` | 6.64M | 5.68M | 0.86× |
| `Account` | 6.49M | 5.98M | 0.92× |
| `Integers` | 6.10M | 4.23M | 0.69× |
| `GlobalInv` | 4.13M | 3.60M | 0.87× |
| `Invariants` | 3.96M | 3.49M | 0.88× |
| `Loops` | 3.67M | 3.51M | 0.95× |
| `Generics` | 2.85M | 2.98M | 1.04× |
| `EnumPayloads` | 1.90M | 2.02M | 1.06× |
| `Read` | 1.15M | 1.16M | 1.01× |
| `EnumPatterns` | 0.72M | 0.76M | 1.05× |
| `Enums` | 0.35M | 0.39M | 1.12× |
| `Modules` | 0.14M | 0.14M | 1.02× |
| `SourceVerification` | 0.02M | 0.02M | 1.01× |
| `Signed` (new) | — | 3.89M | — |

Eight files are still above 1.00×, but they account for **+0.51M against
−10.07M** everywhere else.  Six of the eight are the enum, generics, and module
tests, where the per-view integer rules have almost nothing to do so the extra
abstraction is paid without a matching saving; the exception that matters is
`Quicksort` (+165K, all of it in `partitionLoop`), which is strategy 13.

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
