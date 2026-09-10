# Unified integer model — design plan

## Motivation

Signed and unsigned Move integers are currently two parallel worlds: `Ty.uint w`
/ `Ty.sint w`, `add w` / `addS w`, `UInt W` / `SInt W`, duplicated semantics,
duplicated `sem_preserves` cases, duplicated Checked specs and `wp_*` lemmas.
That doubling is not incidental — it is what forced the `Oper.semSigned`
factoring: 11 extra `Oper.sem` arms pushed `fun_cases`/`split` matcher-principle
generation past the **hard 200000-heartbeat limit** (a fresh-context budget no
`set_option` can raise). The workaround (route signed ops through the catch-all
into a second matcher) is a symptom, not a fix.

The observation behind this plan: **a Move integer is just an `Int` confined to a
range `[lo, hi)`**. Unsigned `Un = [0, 2^bits)`; signed `In = [-2^(bits-1),
2^(bits-1))`. Every operation is "compute the mathematical result over `Int`,
then check or wrap it against `lo`/`hi`." Signedness contributes exactly two
numbers. If the model is parameterized by those numbers, there is **one** oper
family, **one** matcher (so `fun_cases` is fine and `semSigned` disappears),
**one** proof per operation, and half the surface everywhere.

## The abstraction

```lean
structure NumType where
  width  : IntWidth
  signed : Bool
```

Derived, all total and cheap:

| name | definition | unsigned | signed |
|---|---|---|---|
| `size` | `2 ^ width.bits` | `2^n` | `2^n` |
| `lo`   | `if signed then -(2^(bits-1)) else 0` | `0` | `-2^(n-1)` |
| `hi`   | `lo + size` | `2^n` | `2^(n-1)` |
| `toBits v` | `v % size` (emod, lands in `[0,size)`) | `v` (identity) | two's-complement pattern |
| `fromBits u` | `if u < hi then u else u - size` | `u` (identity) | inverse of `toBits` |
| `wrap n` | `lo + ((n - lo) % size)` | `n % size` | `((n+half)%size) - half` |

Two facts carry the whole theory (both one-line `omega` given `hi = lo + size`
and `lo ≤ 0 < hi`):

- `wrap_mem  : lo ≤ wrap n < hi`
- `wrap_of_mem : lo ≤ n → n < hi → wrap n = n`

and for bitwise, `toBits`/`fromBits` round-trip inside `[0,size)`
(`fromBits_mem`, `toBits_mem`). Crucially `toBits`/`fromBits`/`wrap` are the
**identity for unsigned**, so unsigned proofs collapse to today's shape via
`@[simp]` specialization lemmas (`toBits_unsigned : ¬nt.signed → nt.toBits v =
v`, etc.).

## Every operation unifies

`checked nt r := if nt.lo ≤ r ∧ r < nt.hi then ok r else abort`.

| op | unified definition | why it specializes correctly |
|---|---|---|
| add/sub/mul | `checked nt (a ⊕ b)` | only the range differs |
| div | `b≠0 ∧ checked nt (a.tdiv b)` | `tdiv` = floor for nonneg ⇒ = today's unsigned `/`; signed `minInt/-1` overflow caught by `checked` |
| mod | `b≠0 ∧ ok (a.tmod b)` | `tmod` = `emod` for nonneg; always in range for both |
| bitAnd/Or/Xor | `ok (fromBits (op (toBits a) (toBits b)))` | `toBits`/`fromBits` = id for unsigned ⇒ today's `toNat`-bitops |
| shl | `k<bits ∧ ok (fromBits ((toBits a <<< k) % size))` | id for unsigned ⇒ `(a<<<k)%size` |
| shr | `k<bits ∧ ok (a.fdiv (2^k))` | `fdiv` = logical `>>>` for nonneg; arithmetic (sign-extending) for signed — **one** definition serves both |
| cast nt' | `checked nt' a` | **bonus**: unsigned target has `lo'=0`, so a *negative* signed source aborts — the signed→unsigned cast I had to defer falls out for free and correct |
| lt/le/eq | on the mathematical value | already shared today |

So there is no operation that genuinely needs a sign split; the split is always
"consult `nt.lo`/`nt.hi`."

## Layer-by-layer changes

**`MoveModel/IR/Value.lean`** — add `NumType` + derived + the five range lemmas
(generalize the existing signed `toBits`/`ofBits`/`wrapSigned` by replacing
`halfSize` with `hi` and adding `lo`). `TypeTagToken`: replace `uint w`/`sint w`
with `int nt`.

**`ValueTyping.lean`** — `Ty` loses `uint`/`sint`, gains `int (nt : NumType)`;
one `IsValid.intv : nt.lo ≤ i → i < nt.hi → IsValid (.int nt) (.int i)`; one
`isValid_int_iff`. Keep `abbrev Ty.uint w := .int ⟨w, false⟩` and
`Ty.sint w := .int ⟨w, true⟩` so most call sites are untouched.

**`Syntax.lean` (Oper)** — one width-carrying family: `add nt | sub nt | mul nt
| div nt | mod nt | bitAnd nt | bitOr nt | bitXor nt | shl nt | shr nt | cast nt
| lt | le | eq | …`. **Delete** all 11 `*S` constructors. (Note: unsigned
`sub`/`div`/`mod` now carry `nt` — the one deliberate loss of the "width-free
unsigned" property, in exchange for uniformity. The wire format can still hide
it; see below.)

**`Semantics.lean`** — one `Oper.sem` arm per op (≈11 arithmetic arms, same as
today's unsigned-only matcher). The matcher is back to a size where `fun_cases`
works, so **`Oper.semSigned`, the `@[simp]` catch-all routing, and every
`, Oper.semSigned` patch are deleted.** `sem_deref_irrel` reverts to its
original form untouched.

**`CodeTyping.lean`** — one `WfOp` rule and one `sem_preserves` case per op
(the 14 signed cases and the doubled unsigned cases collapse to ~11 generic
cases, each `intv` + `checked`/`wrap`/`fromBits`/`tmod`/`fdiv` mem-lemma).

**`Interp/Exec.lean`, `Mono`, `RefElim`, `Prover/Sim.lean`** — one case per op;
the signed additions and the `semSigned_operandsSafe`/`split at hs` patches in
RefElim disappear.

**`Move/Basic.lean` (source)** — one `MoveInt (S W : Type) [Sign S] [Width W]`
subtype `{ val : Int // (numTypeOf S W).lo ≤ val ∧ val < .hi }`, `toInt : Int`.
`abbrev UInt W := MoveInt Unsigned W`, `SInt W := MoveInt Signed W`, `U8 :=
UInt W8`, `I8 := SInt W8`. `toNat` stays for unsigned as `toInt.toNat` +
nonneg lemma. Operations become one generic set; `less/lessEq/equal` and the
trust-base axioms are shared (already are). The doubled instances/`*_eq_ofInt`
lemmas collapse.

**`Compiler/LIR.lean` + `Normalize.lean`** — one recognition path keyed on the
integer type's `(width, signed)`; one lowering to `int nt` ops. The parallel
`uintBinary?`/`sintBinary?`, `loadUInt`/`loadSInt`, `cast`/`castS` collapse.

**`Frontend/XIR/Json.lean` + `Decode.lean` + Rust `exchange`** — decision point,
see below.

**`Semantics/Checked.lean` + `Verify/{WP,Tactics,Syntax}.lean`** — one Checked
spec family (`addSpec` over `MoveInt`), one `wp_*` lemma each, one entry in the
fn→spec and total maps. The `*SSpec` family and `sInRange` fold into the generic
`checked`/`inRange nt`. Abort conditions read `¬ inRange nt (expr)` uniformly;
`I*.halfSize`/`U*.size` become `nt.lo`/`nt.hi` accessors.

## Status (implemented)

All stages are implemented.  `MoveModel`, `Move`, the default target, the Rust
crates, and **all 91 test targets** build.

`Oper.semSigned` and its `@[simp]` catch-all routing are **deleted**;
`sem_deref_irrel` is back to its original `fun_cases` proof, which is the
concrete evidence that the matcher is small again.

Option **B** was taken for the wire format (see below): the unified model needs
the operand type on every integer operation, including `sub`/`div`/`mod` and
the bitwise ones, so the exchange format carries it uniformly and bumps to
`FORMAT_VERSION = 10`.  Three version stamps must agree: the Rust constant, the
Lean `decodeMProgram` check, and the legacy envelope built by `decodeMModule`.

## Performance

Measured as `lake env lean <file>` minus the fixed ~1070 ms `import Move` cost
(identical in both trees), against a worktree at the pre-refactor commit.

The unification is **not** free, and the reason is worth recording.  The first
working version was *5× slower*, and the cause was not the model: while making
the test corpus pass I had grown the `grind [...]` lists in `Verify/Syntax.lean`
by ~35 lemmas.  `grind` went from 13.8 ms to 69.5 ms on one file while simp's
try-counts barely moved (373 → 385), i.e. the cost was per-attempt E-matching,
not simp-set size.  Pruning those lists back to near-baseline recovered almost
all of it.  **A missing fact should be supplied by a `move_norm` simp lemma or
by `uint_bounds`, never by growing the grind list.**  (`uint_bounds` now expands
product-typed locals into components, since generated contracts package
parameters as one tuple — that alone replaced six per-width grind lemmas.)

What remains is **~10–15% slower**.  Wall clock is only trustworthy on the two
CLI-free files (`CrossInv` 1.02×, `GlobalInv` 1.09×), because every file with a
compiler lowering shells out to `aptos move exchange` and the two trees
necessarily use different binaries — the exchange call alone measured 7 ms with
the pre-existing binary against 139 ms with a freshly built debug one, which
swamps the signal.  The profiler counters exclude that I/O, and over eight files
give `tactic execution` 2400 ms → 2773 ms (1.16×) and `type checking` 819 ms →
916 ms (1.12×), with `simp` at parity.

That residual is the honest cost of the abstraction: a `MoveInt S W` value
carries two class parameters and proofs mentioning `numTypeOf S W`, where
`UInt W` carried one and mentioned `(widthOf W).size` — slightly larger proof
terms everywhere, which the kernel then has to check.  Closing it would mean
collapsing `Sign S` and `Width W` into a single twelve-way tag class, trading
the `UInt W` / `SInt W` factoring for one type parameter.


### Result

Measured with `scripts/bench-proofs.sh` — heartbeats, the deterministic metric;
wall clock is unusable here because the suite spends most of it in the `aptos`
CLI, and the two trees necessarily run different CLI binaries.

Heartbeats measure elaboration work, so **lower is faster**; the rows run
oldest first, and the last one is where the tree stands today.

| state | common 115 fns | vs baseline |
|---|---|---|
| **before** — pre-unification baseline | **117.0M** | 1.000× |
| unified, before this work | 138.6M | 1.184× |
| after the per-view interface | 130.6M | 1.116× |
| after measured corrections | 127.3M | 1.087× |
| after the attribute-priority fix | 118.7M | 1.014× |
| after the never-applied-lemma audit | 107.9M | 0.922× |
| **now** — after the two-spellings fixes | **107.5M** | **0.918×** |

So the unification is not merely paid for — it is **8.2% cheaper than before
signed integers existed**, while covering strictly more (twelve integer types
instead of six).  The whole-suite figure, including the six new signed proofs,
is 143.3M → 111.4M, a 22% reduction.

Three recoveries dominate.  `OrderedMap.lowerBoundLoop` 19.5M → 10.9M, the
proof the diagnosis predicted because it had the deepest bridging chain.  The
attribute-priority bug described above, which was never about the unification
at all.  And the lemma audit (`scripts/simp-audit.sh`, see
`performance-analysis.md`), which found that 88% of all simp *tries* across the
corpus fail, and that three `forall_imp_*` congruence helpers alone accounted
for 4,953 tries with zero successes.

Several proofs are now far below their pre-unification cost: `Integers.halved`
1.86M → 0.37M (0.20×), `Invariants.span` 0.63×, `VectorOperations.nested`
0.78×, `Vectors.insertMiddle` 0.82×, `CrossInv.shift` 0.86×,
`ResourceComposition.shift` 0.87× — the per-view rules close in the first
`simp only` phase where the old ones fell through to the `simp_all`/`grind`
cascade.  What remains above baseline is diffuse and nothing is worse than
1.14×; the two largest absolute residues, `OrderedMap.lowerBoundLoop` (+213K,
1.02×) and `Quicksort.partitionLoop` (+177K, 1.04×), are hand-written loop
proofs paying for larger `MoveInt S W` proof terms in the kernel rather than
for any rewrite that fails to fire.

An earlier version of this section claimed `Integers.masked` needed
`Nat.and_le_left` in its grind list, and that the cost was irreducible.  That
was wrong on both counts: once the attribute-priority bug was fixed, the
per-view `toNat_ofNat_land` rule fired, the extra grind lemma became dead, and
the file went 1.99M → 247K.

### Implemented: generic core, per-view interface

Stages 1–3 of the plan below are done.  What the implementation confirmed:

**The regression was concentrated exactly where the bridging was deepest.**
Per-function heartbeats (`scripts/bench-proofs.sh`, the deterministic metric)
put `OrderedMap.lowerBoundLoop` at 10.7M → 19.5M — **+8.8M, a third of the
whole regression in one proof**, and it is precisely the one whose
`(high - low) / 2` had to travel the longest `Int`↔`Nat` chain.  Proofs with
little arithmetic (`OrderedMap.add`, `.borrow`, `Vectors.removeMiddle`) sat at
1.00–1.01×.  That is the signature of an interface problem, not a model one.

**A pathology this repo had already recorded, repeated.**  `performance-analysis.md`
notes that a `no_index (OfNat.ofNat n)` key is a discrimination-tree *wildcard*
— one such lemma was retried 483,740 times on a single file.  The bridging work
had introduced four more of them as `@[simp]`.  All four are now demoted; the
one place that genuinely needs a numeral bridge (`SourceVerification`, which
opens the relational spec on purpose) passes it explicitly to its `simpa`.  A
global wildcard to serve one local need is never the right trade.

**Removing the spec definitions from `move_spec` was tried, and reverted.**
The reasoning looked sound — they hand a proof the neutral `Int` form around
every per-view rule, the same shape as the `bind` entry in
`performance-analysis.md`.  Measurement disagreed: `Tests/Language/Vectors`
went 7.99M → 10.31M heartbeats with them removed, because proofs that
legitimately work at the relational level then have to re-derive what the
definition gave them directly.  They are back.  The `bind` analogy does not
carry: unfolding `bind` *multiplies* work (three fields, each holding the
continuation), while unfolding a checked spec merely exposes a two-branch
relation.  Not every "expose rules, not definitions" is the same fix.

The leak the removal was meant to plug is closed at its real source instead:
the unsigned *notation* lemmas (`add_eq_ofNat` and friends) now outrank the
generic `ofInt` ones, so a user's `x + 1` normalizes into the unsigned view
before anything else sees it.

**Priority is load-bearing, and only because the view is a head constant.**
The per-view lemmas are keyed on `MoveInt Unsigned W` / `MoveInt Signed W`, so
`simp high` picks the view in one discrimination step and the generic rules
fall through to the signed view.  This is the concrete reason the two-tag
design (`Sign S`, `Width W`) must stay: a fused twelve-way tag would turn
"unsigned" from a matchable head into a class fact, costing an instantiation
per width per lemma.

### Performance plan: generic core, per-view interface

The analysis above pins the regression on one architectural slip: the
*definitions* were unified (correct) and the *proof interface* was unified
along with them (the mistake).  The generic wp/`_eq_pure` lemmas state every
obligation in the neutral `Int` form — `inRange (numTypeOf S W) (a.toInt +
b.toInt)`, values as `MoveInt.ofInt (…)` — and every unsigned proof then pays
a second simp pass through ~30 bridge lemmas to translate back into the `Nat`
form it actually reasons in.  The old model had no such pass because its specs
were *born* in `toNat` form.  The bridges also bloat `move_norm` (68 → 119
entries, ~12 keyed on the same `MoveInt.ofInt _` head — poor discrimination),
and `uint_bounds` grew from one to three hypotheses per unsigned local to make
the bridging possible.

The fix is a principle, not a patch: **definitions, semantics, and typing stay
generic; every user-visible obligation is stated in the view's native domain**
(`Nat` for unsigned, `Int` for signed).  The neutral `Int` form must never
leak into a user goal; bridging happens once, inside interface-lemma
statements.

1. **Per-view wp lemmas.**  For each operation, `wp_addSpec_unsigned` keyed on
   `Spec State (MoveInt Unsigned W)`, stated exactly as the pre-unification
   lemma (`lhs.toNat + rhs.toNat < (widthOf W).size`, value
   `UInt.ofNat (…)`), proved once from the generic lemma plus the bridges;
   `@[simp high, wp_norm]`.  A signed twin keyed on `MoveInt Signed W` in
   `toInt` form.  Generic lemmas demoted to plain theorems.  `Unsigned` /
   `Signed` are constants, so the discrimination tree dispatches the view in
   O(1) — this is why the two-tag design must stay (a fused 12-value tag would
   turn "unsigned" into a class fact and cost 6 instantiations per lemma).
2. **Per-view `_eq_pure`** for the remaining operations (mod/shl/shr/cast;
   add/sub/div exist), all producing `ofNat`-of-`toNat` values.
3. **Prune.**  Drop the `ofInt_*`/`inRange_*`/`natCast_*` bridge zoo from
   `move_norm` (they become internal to 1–2); target ≈70 entries again.
   `uint_bounds` back to one hypothesis per unsigned local; grind lists
   byte-identical to the pre-unification baseline.
4. **Hygiene + guardrails.**  Interface statements use resolved bounds
   (`(widthOf W).size`, `(widthOf W).halfSize`), never `numTypeOf`/`.lo`/
   `.hi`/`inRange` (irreducible, internal).  A small checked-in bench script
   (CLI-free files + profiler counters vs recorded numbers) so grind-list-
   style regressions are caught at review time.

Expected: stages 1–2 remove the second rewrite pass (the bulk of the +16%
tactic time) and shorten kernel replay; residual ≈2–4% from the intrinsically
larger `MoveInt S W` types is the permanent price of the unification.  Beyond
parity, the real levers are independent of this refactor: the `spec_norm`
discharger cascade re-runs simp on every failure path (where absolute simp
time lives), `move_norm` could be split into phase-specific sets, and
generated proofs could carry explicit `simp only` lists computed at generation
time.

## The wire format: two options

The internal unification does **not** force a wire-format change.

- **Option A (keep the wire compact / back-compatible).** The Lean encoder maps
  `sub ⟨w,false⟩ → "sub"` (bare) and `sub ⟨w,true⟩ → {"sub":"iN"}`; the decoder
  inverts. The Rust `exchange` enums stay as the serialization DTOs they already
  are (`Sub` unit, `Add(IntType)`, `IntType::{U*,I*}`). Only the Lean model
  unifies. No format bump beyond the v9 already added. Least churn; the model is
  clean while the wire stays terse.

- **Option B (unify the wire too).** XIR carries one `int` type with a signed
  flag and one nt-annotated oper family; Rust `Type`/`Oper`/`IntType` mirror it;
  format bumps to v10. Cleanest conceptually, but every operation now carries a
  width on the wire (unsigned `sub` included) and the Rust consumer
  (`move-compiler-v2/xir.rs`) must map back. More churn, touches the on-chain
  producer path.

Recommendation: **A**. The wire format is a serialization concern; the value of
this refactor is in the model and proofs, which A delivers fully.

## Proof strategy

Everything reduces to: after a `checked`/`wrap`, the result is in `[lo,hi)`, so
`IsValid (.int nt)` holds by `intv`. The toolkit is `wrap_mem`, `wrap_of_mem`,
`fromBits_mem`, `toBits_mem`, `tmod_mem`, `fdiv_mem` — all already written for
the signed case; generalizing them (`halfSize → hi`, add `lo`) is mechanical and
each stays a one-line `omega`/`emod` proof. Unsigned obligations discharge by
the identity-specialization simp lemmas, recovering today's proofs. No proof
case-splits on sign; it reads the bounds off `nt`.

## Migration strategy (keep every intermediate build green)

1. Land `NumType` + derived + generalized range lemmas in `Value.lean` (additive).
2. Switch `Ty`/`TypeTagToken`/`IsValid` to `int nt`, adding `uint`/`sint`
   **abbrevs** so downstream still elaborates; fix the `IsValid` iff sites.
3. Collapse `Oper` to the single family; delete `*S`; fix `Oper.sem` to one arm
   per op and **delete `semSigned`** and its `@[simp]`/`simp only` patches; revert
   `sem_deref_irrel`. Rebuild → fix `sem_preserves`, Exec, Mono, RefElim, Sim to
   one generic case each.
4. Collapse `Basic.lean` to `MoveInt`, `UInt`/`SInt` as abbrevs; the compiler and
   verify layers follow the renamed ops.
5. Encoder/decoder: Option A mapping. Rust unchanged.
6. Delete the now-dead signed-specific lemmas, run the full suite + `Tests.Move.Signed`.

Because `UInt`/`SInt`/`U8`/`I8`/`Ty.uint`/`Ty.sint` survive as abbrevs, existing
tests and the framework surface keep compiling throughout.

## Benefits

- **Deletes the `Oper.semSigned` workaround** and reverts `sem_deref_irrel` —
  the matcher shrinks below the `fun_cases` budget by construction.
- Halves the operation, semantics, typing, Checked-spec, and `wp` surface; one
  place to state each rule.
- **Signed→unsigned cast becomes correct for free** (the deferred case).
- Future numeric variants (e.g. a different range, saturating ops) are a new
  `NumType` field, not a new parallel world.
- Matches the project's core principle: general, not special-cased.

## Costs / risks / open questions

- One-time refactor of currently-verified, stable unsigned code (the largest
  risk). Mitigated by the abbrev-preserving, staged migration and the existing
  full test suite as a regression net.
- Field rename `UInt.nonneg → isGe` (provide `nonneg` as a lemma for unsigned).
- Decide **A vs B** for the wire format (recommend A).
- Decide whether `IntWidth` stays (referenced by `NumType.width`) or folds fully
  into `NumType` — keep `IntWidth` (widths are still a real axis).
- `div`/`mod` switch from `ediv`/`emod` to `tdiv`/`tmod`; prove equal on nonneg
  so unsigned semantics/baselines are unchanged (they are, for valid operands).
- Effort: comparable to the signed work just done, but net **removes** code; the
  proof generalizations are mostly already written for the signed case.
