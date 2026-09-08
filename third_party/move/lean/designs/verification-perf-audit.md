# Verification performance audit

The bar is v0: seconds per `verify` target.

## Regression tracking

[`LeanerLang/Tests/Performance.lean`](../leaner-ir/LeanerLang/Tests/Performance.lean)
is the benchmark: one curated target per cost class, each proved by a bare
`verify`, compared against
[`Performance.exp`](../leaner-ir/LeanerLang/Tests/Performance.exp) and run
as part of `lake test`.  Two numbers gate it and one does not:

* **heartbeats** — the elaborator's own work counter, reproducible to
  within a few units for the same source and toolchain.  It measures
  *search*: how hard the tactics worked.
* **objects** — distinct subterms of the finished proof, sharing
  respected.  It measures *term size*: how large the goals were that the
  tactics carried.
* **wall time** — reported beside them, never compared, because it is the
  number a reader wants and a machine cannot reproduce.

Keeping search and term size apart is what makes a failure actionable, and
the report says which one moved: term growth means the goals carried more,
so a frame or state record stopped being consumed; search growth with
flatter terms means a rewrite stopped firing and its result is being
re-derived. The report also prints the profiler command for a phase
breakdown of a single target, and the `UB=1` command that accepts a
deliberate change.  A tolerance of 10% absorbs counter jitter without
hiding a real regression.

## A baseline must be regenerated against a fresh build

The baseline committed with the native-reference and loop work did not
reproduce from the source it was committed with: a clean tree at that
commit measured 19–150% above it on six targets and 1–8% above on the
rest, reproducibly to within 0.01% across runs, on the pinned toolchain.
The pre-amend commits in the reflog carry byte-identical trees, so there
was no earlier code state to recover and nothing had regressed after
recording; the numbers were simply recorded against different imports.

The cause is in the command this file used to recommend: `lake env lean`
sets the module path and elaborates one file, but it does **not** build
what that file imports. Regenerating with `UB=1` after editing a
dependency therefore records the cost of the previously built imports and
commits a baseline that its own source cannot reproduce — which is worse
than a stale number, because the gate then reports a regression that no
change caused, and the reader looks for it in the wrong place. The
command is now `lake build && UB=1 lake env lean …` in the report text,
in [`../CLAUDE.md`](../CLAUDE.md), and here.

The baseline was regenerated against a fresh build on 2026-09-01; all
twelve targets moved, which is itself the signature of a whole-file
re-record rather than a targeted change. The figures in the next section
predate that regeneration and are kept as the relative picture they were
written to show; the current absolute numbers are in
[`Performance.exp`](../leaner-ir/LeanerLang/Tests/Performance.exp).

One further measurement rule, now also in the report text: never compare
a `-Dprofiler=true` run against the baseline. Profiling inflates
heartbeats non-uniformly — up to +150% on one target here — so it is a
phase breakdown only.

## Where the time goes

Measured 2026-08-31 by the benchmark above, after V6:

| Target | Heartbeats | Objects | Wall |
|---|---|---|---|
| `guarded` (checked add) | 37.5M | 6.6k | 0.8s |
| `bump` (scalar `&mut`) | 50.2M | 10.1k | 1.2s |
| `deposit` (field-focused borrow) | 86.8M | 64.3k | 3.2s |
| `carry_u64` (generic callee) | 88.1M | 10.4k | 2.0s |
| `replace` (whole-resource borrow) | 116.2M | 18.7k | 2.5s |
| `take_and_bump` (call + arithmetic) | 183.2M | 22.7k | 4.4s |
| `bump_twice` (two nested calls) | 713.5M | 28.2k | 16.1s |

The two dimensions separate the outliers cleanly, and they are *different*
outliers:

* `bump_twice` is **search-bound**: 25k heartbeats per proof object, six
  times the ratio of `deposit`.  Its proof is not large; the closing's
  `simp_all` passes and `subst_vars` reprocess the whole context — every
  hypothesis as both a rewrite rule and a target — once per residual goal,
  and a second call boundary doubles the context they walk.
* `deposit` is **term-bound**: the largest proof in the set by a factor of
  two, at a middling search cost.  A field-focused borrow carries the
  holder, the focused loan, the loan-location registry, and the keyed
  global hole through every subsequent goal.

The findings below are ordered by that split.

## Findings, ranked

**F0 — the drive rebuilt its rule set at every program point (fixed).**
The two tactics the drive spends its time in were measured directly, by
timing each alternative of the drive loop on `guarded`, the cheapest target
in the benchmark. Of 484ms of drive time, `leaner_denotation_reconcile`
took 234ms over 22 calls and `leaner_denotation_reduce_equation!` 222ms
over 18: **94% in two alternatives, both of which mostly fail** (4 wins of
22, and 3 of 18). The rest of the loop — sixteen other alternatives, 285
attempts in all — accounted for 28ms.

The cost was not the rewriting. It was the rule set: both tactics carried
their inventory as a list written inside the tactic, and a list is
elaborated into a discrimination tree on **every** application, while the
drive applies them at every program point. Cutting the inventory from 68
lemmas to 4 dropped that alternative from 234ms to 65ms with an
identical drive path, which puts the rebuild at about 7.7ms a call, or
0.11ms per lemma per call. `leaner_denotation_reconcile_at` builds two
such sets per call, which is why it was the second-largest cost.

The inventory is now the registered `lir_reconcile` attribute set
(declared in [`SimpAttrs.lean`](../leaner-ir/LeanerIR/Proofs/SimpAttrs.lean),
membership listed in one `attribute` command beside the tactics that use
it), so the environment maintains the tree once.
`leaner_denotation_reconcile_at` still adds inline the four
returned-reference rows it alone carried. Membership is unchanged, and
**every benchmark target produced a byte-identical proof** — the object
counts did not move — so this is search removed, not proof changed:

| Target | Before | After | |
|---|---|---|---|
| `guarded` | 39.4M | 15.9M | **-59%** |
| `replace` | 173.0M | 104.9M | **-39%** |
| `set_then_read` | 82.6M | 51.2M | **-37%** |
| `bump` | 62.4M | 39.7M | **-36%** |
| `carry_u64` | 114.9M | 88.9M | -22% |
| `forward_reborrow` | 118.0M | 94.1M | -20% |
| `reborrow` | 73.3M | 59.5M | -18% |
| `take_and_bump` | 190.8M | 160.5M | -15% |
| `set_through_forward` | 327.2M | 281.1M | -14% |
| `bump_twice` | 258.7M | 227.2M | -12% |
| `withdraw` | 928.2M | 825.8M | -11% |
| `deposit` | 511.2M | 464.0M | -9% |

The gradient is the finding's own confirmation: the saving is a fixed cost
per program point, so it is largest where the proof is smallest. It also
generalizes — **any lemma list written inside a tactic the drive applies
repeatedly is paying this**, and the remaining inline lists in the drive
and the closing are the next place to look.

F0 through F0d share one shape, and it is worth naming because it will
recur: **the drive pays for every question it asks at every program point,
so a question whose answer is already determined is the expensive kind.**
The rule set was already determined and was rebuilt (F0); the goal's
logical shape was already visible and was probed by unification (F0b) or
normalized before being taken apart (F0c); the defeq of a reduction's
result was already established and was re-decided (F0d). None of these is
about how hard the mathematics is.

**F0b — the drive asked expensive questions it could answer syntactically
(fixed).** The same per-alternative timing, run on `withdraw`, found the
next layer. Of about 20 seconds of drive time over 384 goals:

* `exact True.intro` was attempted 141 times, cost **2.8 seconds, and
  succeeded zero times**. On its own that tactic asks whether the goal is
  *defeq* to `True`, and these goals carry whole frame and state records,
  so the negative answer is the expensive one.
* `apply And.intro` cost 956ms over 70 attempts for 8 wins, for the same
  reason: `apply` unifies against the goal.
* `leaner_denotation_reconcile_returned!` cost 581ms over 256 attempts for
  zero wins, because it evaluated its two preconditions in the wrong
  order — the cheap one is a search of the target, the expensive one scans
  every hypothesis, and it computed the expensive one unconditionally.

All three now decide syntactically first: `leaner_denotation_trivial`
requires the target to be spelled `True`, `leaner_denotation_split_conjunction`
requires an `And`, and the returned-reborrow guard checks its pending
write-back before scanning the context. The drive reduces a head before
reaching these alternatives, so a post that means `True` arrives spelled
that way.

This is worth stating as a rule, because it is the same rule as F0: **a
tactic in the drive's alternative list is invoked at every program point,
so anything it does before deciding it does not apply is paid across the
whole body.** A syntactic precondition costs microseconds; unification and
context scans cost milliseconds.

The gain lands on exactly the targets with the most program points — the
two the benchmark cares most about — with proofs again byte-identical:
`withdraw` 825.8M → 674.0M (**-18%**) and `deposit` 464.0M → 387.9M
(**-16%**), and no other target moved by even one percent.

**F0c — reconciliation was normalizing the logical shape the drive was
about to take apart (fixed).** With the guards in, the head constant of
each goal was recorded alongside the outcome. Two classes stood out, and
both were the drive doing one thing while another of its own rules was
waiting to do the right one:

* 44 goals whose target is quantified cost `leaner_denotation_reconcile`
  1.7 seconds and were reconciled **zero** times; they are exactly the 44
  goals `leaner_denotation_intro_guarded` went on to win.
* 34 conjunctive goals cost it 3.9 seconds for 7 rewrites, with
  `leaner_denotation_split_conjunction` waiting behind it.

Reconciliation is normalization of a proposition; a binder and a
conjunction are structure, and the drive carries its own rules for those.
The alternative is now `leaner_denotation_reconcile_guarded`, which
declines a quantified or conjunctive target. Reordering was considered
first and rejected: the alternative list documents that a visible storage
read must be resolved before logical introduction, so introduction cannot
simply move ahead of it.

Unlike F0 and F0b this one changes proofs, because it changes which rule
fires, and it is the trade the benchmark exists to make visible: search
falls sharply while some proofs grow a little. `bump_twice` **-62%**
(226.9M to 84.8M), `take_and_bump` -49%, `replace` -45%, `deposit` -32%,
`withdraw` -24%; proof size moves between -9% and +12%, the one increase
being `deposit`. `VerificationStorage` fell from 40s to 8.4s. All four
suites stay green.

**F0d — head reduction re-decided by `change` what `whnf` had just
established (fixed).** With the waste removed, the largest alternative left
was `leaner_denotation_reduce_head` at about 25ms a call, and it was
expensive even where it succeeded — 104ms a call on `consValues`, 114ms on
`wpExpr_localVar.match_1`, both of which win every time they are tried. The
cost was not the reduction. The tactic finished with `goal.change reduced`,
and `change` verifies by default that the new target is defeq to the old
one — over goals carrying whole frame and state records. But `reduced` came
out of `Meta.whnf target`, so it is defeq to `target` **by construction**,
and the check re-decides what reduction just did. Passing
`checkDefEq := false` halved the alternative, 24.7ms to 12.0ms a call, and
the same argument and fix apply to the two `change`s on the native loop
path.

This one is behavior-preserving by construction, and the measurement agrees
— every proof is byte-identical: `withdraw` 508.9M → 379.0M (**-25%**),
`deposit` 260.1M → 217.5M (**-16%**), nothing else moved. `withdraw`'s wall
time is now 10.1s against 28.8s before F0c.

Together F0 through F0d take the benchmark from 2,880M heartbeats to
1,394M, **-52% overall**. Only F0c changed any proof:

| Target | M0 baseline | F0 + F0b | F0c | F0d | |
|---|---|---|---|---|---|
| `bump_twice` | 258.7M | 226.9M | 84.8M | 84.5M | **-67%** |
| `replace` | 173.0M | 104.2M | 56.7M | 56.3M | **-67%** |
| `guarded` | 39.4M | 15.8M | 14.6M | 14.6M | **-63%** |
| `withdraw` | 928.2M | 674.0M | 508.9M | 379.0M | **-59%** |
| `deposit` | 511.2M | 387.9M | 260.1M | 217.5M | **-57%** |
| `take_and_bump` | 190.8M | 160.3M | 81.3M | 81.2M | **-57%** |
| `bump` | 62.4M | 39.6M | 35.3M | 35.2M | **-44%** |
| `set_then_read` | 82.6M | 51.0M | 48.1M | 47.9M | -42% |
| `forward_reborrow` | 118.0M | 94.0M | 81.0M | 81.0M | -31% |
| `carry_u64` | 114.9M | 88.8M | 83.3M | 83.3M | -28% |
| `set_through_forward` | 327.2M | 280.7M | 256.2M | 256.0M | -22% |
| `reborrow` | 73.3M | 59.4M | 58.7M | 58.6M | -20% |

### Where the cost sits after F0d, and what the closing will not give up

The drive/closing split was re-measured after F0c and F0d, by admitting
the closing with `sorry`. The drive is now **62%** of the benchmark, down
from about 85%, and it has concentrated rather than shrunk evenly:

| Target | Total | Drive | Closing | Closing share |
|---|---|---|---|---|
| `withdraw` | 379.0M | 349.4M | 29.6M | 8% |
| `deposit` | 217.5M | 192.8M | 24.7M | 11% |
| `bump_twice` | 84.5M | 63.6M | 20.9M | 25% |
| `replace` | 56.3M | 34.6M | 21.7M | 39% |
| `take_and_bump` | 81.2M | 40.3M | 40.9M | 50% |
| `guarded` | 14.6M | 6.9M | 7.7M | 53% |
| `set_through_forward` | 256.0M | 100.7M | 155.3M | 61% |
| `set_then_read` | 47.9M | 18.8M | 29.1M | 61% |
| `forward_reborrow` | 81.0M | 29.1M | 51.8M | 64% |
| `bump` | 35.2M | 7.7M | 27.5M | 78% |
| `reborrow` | 58.6M | 8.8M | 49.8M | 85% |
| `carry_u64` | 83.3M | 10.6M | 72.7M | 87% |

Two readings. First, **the remaining drive cost is four functions**:
`withdraw`, `deposit`, `set_through_forward`, and `bump_twice` are 82% of
all drive time, and the first two alone are 63%. Second, **the closing has
a floor of roughly 20–50M per function that barely varies with the
function**, which is why it dominates every small target;
`carry_u64` — whose body is one call and whose contract is
`result == value` — spends 72.7M there against 10.6M in the drive.

That floor was investigated and did not yield. Profiling `carry_u64` in
isolation shows the whole closing is **one `simp_all` call of 1.84s**, the
first one inside the per-goal block. Four hypotheses about its rule set
were tested against it, and all four were rejected:

* dropping the preceding whole-goal pass, on the theory it was redundant:
  **worse**, 2.96s — the later call does more without it;
* substituting the drive's execution equations first, so the passes walk a
  smaller context (this is F1c's proposal): **no change at all**, though
  the storage proofs grew 1%, so substitution did happen and simply did
  not pay;
* replacing the five semantic-operation *definition* unfolds with the
  closed `lir_reconcile` rows: **worse**, 2.51s;
* removing the argument and result codec unfolds, on the theory they
  produce the `dite` chains simp then fights: **no change**, 1.80s.

The closing is a tuned equilibrium, and every local perturbation of it is
neutral or worse — including the goal-directed rewrite recorded below.
The conclusion is a scheduling one: **the closing will not be improved by
adjusting it, only by not generating its goals in the first place**, which
is the executor's job in
[`certifying-execution.md`](certifying-execution.md). Effort spent tuning
the `simp_all` chain is effort spent against a local optimum.

That conclusion has since been confirmed from the other side. A
constructive closing — one that walks the obligation grammar and assembles
the proof term rather than searching for it — closes `guarded` at 7.4M
against 14.6M, **-49%**, on a target whose closing was 53% of its cost. So
the floor is an artifact of searching, not a property of the obligations.
It is implemented and gated in
[`Certify.lean`](../leaner-ir/LeanerIR/Proofs/Certify.lean); the gate
matters, because an ungated attempt on the eleven targets it cannot yet
close cost 2–4% each and made the prototype a net loss. What blocks the
next target is recorded there: not the grammar, but the loan-registry side
conditions on the closed rows, which the drive discharges from facts it
accumulates while stepping and the closing does not reproduce.

What remains is genuine work rather than overhead, and it is now measured
per target. Admitting the closing with `sorry` splits each target into
drive and closing: `withdraw` and `deposit` are about 96% drive,
`bump_twice` 91%, while the mid-sized reference targets are 55–84%
*closing* and `guarded` is about half each. So the drive is roughly 85% of
the benchmark and the two dominant alternatives left in it are
`leaner_denotation_reconcile` (44ms a call on `withdraw`) and
`leaner_denotation_reduce_head` (23ms a call) — both now doing real
rewriting and reduction over oversized frame and state records, which is
F7's term-size problem and the case for the certifying executor in
[`certifying-execution.md`](certifying-execution.md).

A further experiment is recorded because it failed. Replacing the closing's
`simp_all` chain with one goal-directed `simp` plus a structured finisher
was tried on the premise that `simp_all` is quadratic in the context. On
`guarded` it was break-even (40.4M against 39.4M), and admitting the
closing with `sorry` showed why: the closing is about 1M of that target's
39M, and **97% of a scalar target's cost is the drive**. On the reference
and storage targets the same closing was catastrophic — `reborrow` +770%,
`replace` +726% — with proof-object counts unchanged, meaning it searched,
failed, and left the fallback to redo the work. It was removed. F1b and
F1c remain the correct reading for `bump_twice`, whose context is large;
they are not the reading for the rest of the benchmark.

**F1 — context-wide substitution once per program point (fixed).** The
drive normalized one evaluator equation per step and then ran `subst_vars`,
which re-walks every hypothesis in the accumulated context to find the
value that equation names — once per step, over a context that grows with
every step. Stepping now substitutes only the equation it just normalized
and the closing runs the context-wide pass once, where the clauses that
need those facts are proved. `bump_twice` fell from 713M to 528M
heartbeats (**-26%**, 16.1s to 13.3s) with no proof growing and no other
target moving; the profiler's tactic time dropped from 6.3s to 3.2s.

The narrow substitution alone is not enough: the abort fixtures read facts
from equations other than the one each step normalizes, and dropping the
wide pass entirely leaves their arithmetic clauses unproved. Placing it in
the closing keeps them and still pays for it once.

**F1b — the remaining simp time is per-step, not per-clause.** After F1,
simp is 10.8s of `bump_twice`'s 13.3s, in about twenty calls of
100–830ms. The closing's rule set is not the cause: dropping the WP
inventory from its lists, which execution has finished with by then,
moves nothing (0% to -1%). What is left is the drive's per-step
normalization of one evaluator equation, whose cost scales with the size
of that equation — every goal past a call boundary carries complete frame
and state records. That makes F1b a term-size problem, and it is the same
problem as F7 below.

**F1c — closing passes scale with a context the goal no longer needs.**
The closing clears only `prepared`/`hu`/`hw`/`hn`; consumed evaluator
equations, routing facts for keys the clause does not read, and superseded
write-back equations all stay, and each `simp_all [big list, defaults]`
walks them. This is the search cost `bump_twice` still pays: its proof
is small, but every residual goal drags the whole accumulated context
through the passes.  After F1 the profiler puts 10.8s of its 13.6s in
simp, spread over about twenty calls of 100–830ms. *Fix:* shrink the context the passes walk —
clear spent equations, and split the closing lemma list by goal class (an
ensures goal never needs the loan-bookkeeping rows). A cheaper variant was
tried and rejected: a light-first closing (`Obligation_iff`/witnesses/`omega`
with `done`, falling back to the full passes) changes nothing, because
every surviving goal needs the heavy normalization to crunch its run
equations first.

**F2 — string identity in hot comparisons (phase A done).**
`RuntimeValue.nominal` now carries the validated `StructHandle`, so
constructor binds, decodes, and evaluator checks decide interned `Nat`
equality through `DecidableEq` instead of character-wise string equality
(and the handle types dropped derived `BEq`, so `==` is lawful by
construction — the F5 workaround retires for them). The pattern binder
resolves names through `resolveNominal?` (the unit is threaded through
`bindPattern`/`lowerPattern?`), and the agreement script transports the
binder premise via `letNativeValue_agrees_of_unit` — inside the generated
script only `_of_unit`-style transports fire; the `rw [$unitRule]` tier
does not. Measured after: `bump_twice` 18.4s, `take_and_bump` 4.9s,
`VerificationStorage` 25.9s, `VerificationAborts` 27.0s — a 4–10% trim on
the storage/abort fixtures, none on nominal-free `bump_twice`, matching
the prediction that term size (F1) dominates today. Phase B remains:
`StorageKey.address` still keys global memory on raw strings, and variant
identity is still a string.

**F3 — array spelling multiplicity.** `Array.mk`-of-list, `List.toArray`
literals, `set!` vs `setIfInBounds`, push-form vs literal registries: each
pair silently splits the rewrite space, and the reconcile set now carries
normalizers plus per-spelling closed lemmas. The tactical patch works; a
representation cleanup (one constructor form everywhere the semantics
builds arrays) would delete the normalizer zoo and its residual risk.

**F4 — linear loan registries.** `activeLoans`, `loanLocations`,
`globalLoans`, `pending` are scanned with `find?`/`filter`. The closed
lemmas bypass the scans in proofs, so this is interpreter-side cost and
lemma-surface (each new frame shape needs a closed row); keyed structures
would shrink both.

**F7 — a focused borrow runs on the generic route, and making it native
needs an IR decision.** `deposit` and `withdraw` produce the two largest
proofs in the benchmark (64k and 99k objects against 19k for the
whole-resource `replace`) for one reason: their borrow places are
projected, `lowerDerefLocalBorrow` accepts only `deref(localVar)`, so both
fall out of the shallow subset and neither gets a V3 wrapper.

A native descriptor for the projected shape is blocked on where field
resolution happens. `Place.field` carries only a field *name*, and
`resolvePlaceFuel?` resolves the index from the runtime value's nominal
handle. A descriptor that checked an expected handle would disagree with
that walk whenever the runtime tag differs from the static type — which a
validated unit rules out, but the evaluator equality the agreement
interface consumes is unconditional, so ruling it out is not available.
Two ways forward:

1. **Carry the owner in `Place.field`.** Place resolution then decides
   statically, exactly as the value-level select already does:
   `.data (.select reference field)` carries a `QualifiedRef` and fails on
   an owner mismatch, so places are the outlier and this restores the
   symmetry. It is an IR schema change, reaching the frontends, the
   printer, validation, and the e2e baselines.
2. **Weaken agreement to a conditional equality.** Cheaper to write and
   it gives up the property that makes the shallow route cheap, so it is
   the wrong trade.

Route 1 is implemented: `Place.field` carries its owner, and
`resolvePlaceFuel?` resolves the position from the declaration and rejects a
value of another shape. On top of it, `resolvePlace?_fieldOfDerefLocal`
resolves the three-level shape natively — which needed the resolution fuel
to allow a third peel, the constant having been sized for the two-level
shapes lowering used to emit — and the reborrow descriptor carries a field
step whose evaluator equality, `evaluator_eq_field`, is now unconditional
because both sides fail together on a mismatch.

What is not done is routing the storage functions through it. Enabling it
puts them on the shallow route — the agreement side works, `typedVerified`
is produced — but symbolic execution stalls before entering the body: the
closed initial-frame rows cover one and two locals, and a synthesized
holder makes four, so `nativeInitialFrame?` stays folded and the drive
never starts. Unfolding the definition generically is the obvious
generalization and did not by itself make a one-field probe converge, so
at least one further step needs diagnosis. Until then the acceptance in
`lowerDerefLocalBorrow` stays off and the two functions verify on the
generic route with their proofs unchanged (64k and 99k objects).

**F5 — derived `BEq` on nested inductives is `partial`.** Equality of
interned references runs through opaque `partial` instances, unprovable
and un-unfoldable; the resolver equalities work around it via the
simp-owned executable route. An interned identity (F2) makes most of
these comparisons `Nat` equality and retires the workaround.

**F6 — symbolic-registry lookups (done).** `semanticProfile?` over the
quantified registry is sealed and treated as routing; this removed
20–100s pathologies during V3 stabilization.

## Recommendation

F1 and F2 phase A are in. F1b resolved into F7: what is left of
`bump_twice`'s time is simp over oversized per-step equations, which is
the same term-size problem that makes `deposit`'s proof the largest. So
next is F7 — state the closed rows over the part of the frame they read,
and stop carrying complete frame and state records past every call
boundary — then F2 phase B (numeric storage keys, variant indices) with F3
riding along, F1c and F4 last. Re-run the benchmark after each step; `UB=1` records the new numbers
and the diff is the evidence that the step worked.
