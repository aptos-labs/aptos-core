# A proof language for authored proofs

Status: proposal, stated against the native verifier
([`native-verification.md`](../designs/native-verification.md)). Nothing is
implemented; the milestones name the first step and the decided order
against [`native-function-values.md`](native-function-values.md) (see
"Milestones"). The design keeps the thesis of
[`proof-strategy.md`](../designs/proof-strategy.md) (cheap, bounded
automation; authored proofs on hard obligations; no SMT) and changes only how
an authored proof is written and lowered.

## Problem

An authored proof exists for speed. A hard leaf gets one because checking
the stated argument costs a fraction of the automatic attempt on it, which
spends its cap on preparation and search before it fails or succeeds; on the
map use cases, repeated normalization and general arithmetic or congruence
search cost much more than checking the stated map lemmas
([`proof-strategy.md`](../designs/proof-strategy.md), "Thesis"). A proof
language is worth having only if it keeps that: a proof written in
it must check for no more heartbeats than the hand-written script, and must
be cheaper to write and to keep working.

An authored proof is a Lean tactic script over the leaves `verex_obligations`
presents ([`source-verification.md`](../designs/source-verification.md),
"Proofs"). The corpus is 18 proof files, 1882 lines, 65 `verify` items and 11
theorems (`aptos-move/framework/**/*.proof.lean`,
`move-prover/tests/sources/functional/**/*.proof.lean`; the two paper
examples under `move-prover/doc/higher-order-paper-26` are counted apart, one
of them stated over runtime values and turned off,
[`roadmap.md`](roadmap.md)). Counted over its 1539 body lines (headers,
`open`, comments and item heads excluded; a line can fall in several
classes):

| Class | Lines | Share | What it is |
|---|---:|---:|---|
| Preparation | 374 | 24% | `verex_prepare …` (104 calls in six variants), `verex_paths`, `verex_intro_any`, `verex_normalize_context`, `verex_split_hypotheses`, `verex_subst_vars`, `verex_reduce_projections`, `verex_finish_prepare`, `verex_canonical_families`, `verex_type_bounds`, `verex_name_maps`, `verex_cases` (54), `verex_group` (9), `clear` of obsolete callee facts, `simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at *` |
| Dispatch by trial | 536 | 35% | `all_goals first \| … \| skip`, `try tac`: a goal finds its closer by trying every alternative in turn (`big_ordered_map`: about 210 alternative lines) |
| Solver vocabulary | 128 | 8% | the normal form's spelling in a proof: `vir_denote_norm`, `vir_denote_eval`, `variantName`, `variantPayload`, `HList.ofSkolem`, `NTy.ofSkolem`, `variantCarrier.*`, `IntegerValueFits`, `SpecInt.val_ofInt`, `Obligation_iff`; carrier spellings `SpecInt (.bits 64) false`, `{ values := #[⟨1, by decide⟩, …], bounded := by decide }`, witnesses `⟨3, by decide, by decide, rfl⟩`; specialized names `«spec_fold$lambda$0».at_T0.spec unit (Skolems.instantiate ⟨.cons (.int 64 false) .nil, by decide⟩ (Skolems.runtime unit))`, `count_is_len.lemma unit (…)`; positional payload names `it_Some_2`, `oit_Position`, `internal_find_result_0` |
| Mathematical argument | 581 | 38% | `have`/`exact`/`rw`/`rcases` lines; 113 of them apply a `SpecMap` lemma, 25 distinct lemmas, `keyAt_rank`, `hasKey_keyAt`, `rank_lt_size` most |

Ten of the 65 items contain no argument at all: 64 lines of preparation and
dispatch that prove nothing a decider could not, given the right context. The
12-step pipeline (`verex_paths; verex_intro_any; verex_normalize_context;
verex_split_hypotheses; verex_subst_vars; verex_reduce_projections;
verex_finish_prepare; …`) is copied seven times in
`big_ordered_map.proof.lean`, and the iterator block (`verex_cases it`, five
`clear …_ensures_k`, `verex_cases iter_next_result`, two normalizations,
`verex_type_bounds`) five times. Of the 162 `verex_*` tactics the solver
defines, 32 occur in proofs; the rest are internals that leak through the
same namespace.

The cost side is in [`performance.md`](../designs/performance.md), "Authored
proof interface": the directed part of an authored proof is a few percent of
its cost; the rest produces and normalizes its leaves, preparation repeated
at every obligation of a path and a callee's contract normalized at every
call (the call stage 20–28% of authored targets). The figures were measured
on encoded leaves and are to be measured again on native ones; the
proportions are what the design relies on.

Three facts about these numbers decide the design:

1. **Preparation cost is a system problem the language cannot fix.** A
   fact is prepared once per goal that holds it because the normal form does
   not commute with substitution. Preparing a call's facts where the walker
   introduces them was tried three ways and paid on none of the authored
   targets ([`performance.md`](../designs/performance.md), "Tried and
   rejected"). The remedy is a rule set that commutes with substitution and
   a context prepared once per path ("Open levers", 1), with or without a
   language.
2. **Every cost lever is blocked by the scripts.** The levers are annotated
   with it: directed steps "not measured as a replacement; the scripts
   depend on `simp_all`'s results"; continuation folding at calls "broke
   authored scripts; rejected". The move to native leaves ported every
   authored proof. The scripts depend on the normal form because they
   address goals by position and rewrite them by Lean lemma names, so the
   normal form is their interface.
3. **Dispatch and the solver vocabulary are language problems, and dispatch
   is paid for.** A proof names a leaf by a traversal-numbered tag
   (`ensures_2_1`, `aborts_if_1_8`, `ensures_1.inl`, `invariant_4_step_1`,
   `after_add_shares_1`, `leaf_2`) or not at all (a callee's precondition
   is keyed by its clause and call since L0's first part, `requires 1 of
   iter_next at call 2`); the payload of a variant is
   `it_Some_2`; a generic callee's contract is `g.at_T0.spec` with an
   explicit `Skolems` instantiation; a concrete key's `DecidableEq`
   instance is spelled `NTy.decEq carriers τ` in the leaf and
   `instDecidableEqString` in a term the proof elaborates, so `cases` and
   `simp` miss the leaf's terms ([`roadmap.md`](roadmap.md), "Key-instance
   spelling"). Nothing in the surface says what an obligation is, so the
   author writes a cascade and lets each goal find its closer, and every
   alternative that fails before the one that closes is checked and paid:
   an `omega` that fails over a prepared map context, a `simp_all` that
   rewrites and then does not close, an `exact` that elaborates and
   mismatches. The same scripts prepare twice, once in a `preparing` block
   and again with `verex_prepare typed` inside the alternative, and name
   the maps of every goal whether or not a map lemma is applied to it.

The conclusion: a structured proof language is worth building as the
**stability boundary** between proofs and the solver, so that the cost work
can change the normal form without touching a proof; it removes dispatch and
the solver vocabulary from proofs by construction, and with dispatch the
checking of failed alternatives and the repeated preparation the scripts
carry; it does not change what preparation or a decider costs, and the
design must not promise that.

## Goals and non-goals

Goals, in order:

- **Efficient proofs.** A proof in the language is cheap to check and cheap
  to write and maintain. Cheap to check: the heartbeats of a target under
  its lowered proof, preparation included, are at or below those of the
  best hand-written script for the same target, and nothing in the lowering
  does work the script would not do; no trial dispatch, no preparation of a
  goal already prepared in its scope, no contract normalized again at an
  `apply`, no elaboration that searches. Cheap to write: the proof names the
  argument and nothing else, in the vocabulary the author already knows, and
  the message prints the frame it must fill. Cheap to maintain: no proof
  text mentions a normal form, so the solver changes without the corpus.
  Every construct below is justified against this goal; a construct that
  buys convenience at checking cost is left out or lowered so that it costs
  nothing by construction.
- A proof is written in the specification's vocabulary: the Move spec
  expressions of the module, its locals, `old`, `result`, its specification
  functions, its lemmas and the built-in lemmas. No carrier spelling, no
  specialized name, no Lean lemma name and no solver tactic appears in a
  proof unless it opts into the escape.
- Every goal a step sees is native, as the verifier enforces
  (`verex_check_native` on leaves, prepared leaves and their cases;
  `reportAuthoredRuntimeNames` on authored proofs). The language adds
  nothing the enforcement must exempt; a `lean =>` block is checked as any
  authored proof is. A function value is a value of the
  `Carriers.functionCarrier` interface of
  [`native-function-values.md`](native-function-values.md), the spelling
  the elaborator targets for every function-typed expression; one boundary
  still shows runtime values in a leaf, the requires table over
  `Array RuntimeValue`, which goes with that plan's step 3. The language
  states no construct over the runtime closure vocabulary.
- An obligation is addressed by what it is (the clause, its owner site, the
  call or loop that raises it), never by a traversal number, and a path to it
  by the source decisions on that path (a branch taken, a call or `assert!`
  that aborts), so that a proof can guide every path individually.
- Preparation is invisible. The proof states facts and names case splits; the
  system decides when and how to prepare, and may change it.
- Automation stays as it is: an obligation the proof addresses gets the
  cheap deciders with the proof's facts; one it does not address gets the
  cheap deciders alone; the full automatic leaf runs only where the proof
  asks for it, under its cap.
- The failure message of an automatic verification prints the proof it asks
  for in this language, with each open obligation stated in the same
  vocabulary, so that a person or an agent can write the proof from the
  Move file and the message alone.
- The Move Prover's proof blocks (`assert`, `assume`, `apply`, `split`,
  `calc`, `let`, `if`, `post`; [`lemmas.md`](../designs/lemmas.md)) are the
  precedent for every step that has one.

Non-goals:

- A general tactic language, or a replacement for Lean where an argument is
  genuinely about Lean terms: those stay Lean theorems and `lean =>` steps.
  No item of the corpus is such an argument once the lemma items, the
  built-in lemmas and the normal-form fixes below exist ("Coverage of the corpus").
- New deciders, larger budgets, or search inside the closers. The two
  closers that wrap a Lean decision procedure, `ring` and `congr`, run one
  module of `grind` each with splitting and instantiation off.
- Counterexamples, or proof search for the author.
- A change to the Move language. The Move proof block keeps its grammar;
  the obligation-addressed form lives in the proof file (see "Host").

## Surface

A proof file keeps its header and its items, and the item keeps its keyword:
`verify f by` followed by a block of the language. Lean tactics are reached
through the `lean =>` step only, so one grammar parses every item and the
language's step names (`apply`, `split`, `calc`, `let`, `if`) never compete
with Lean's tactics of the same name. A whole-target Lean proof is `verify f
by lean => …`. A proof file may also state and prove lemmas in the
specification vocabulary ("Lemma items").

```text
item      ::= 'verify' f 'by' block
            | 'lemma' L ('<' T (',' T)* '>')? '(' params ')'
                ('requires' e)* ('ensures' e)+ ('decreases' e)? 'by' block
block     ::= step*
step      ::= 'let' x ':=' e
            | 'assert' e ('by' closer)?
            | 'assert' e 'by' 'contradiction' '{' block '}'
            | 'assume' e
            | 'apply' φ                                -- lemma instance
            | 'split' e ('in' e '..' e)?               -- Boolean, enum/option, function value; integer range
            | 'let' Variant '{' x (',' x)* '}' ':=' e  -- destructure a known variant
            | 'calc' '(' e (op e)+ ')' ('by' closer)?
            | 'forall' binders ('where' e)? '{' block '}'
            | 'witness' e (',' e)*
            | 'if' e '{' block '}' ('else' '{' block '}')?
            | point selector? '{' block '}'
            | 'on' obligation (',' obligation)* selector? '{' block '}'
            | 'lean' '=>' tacticSeq
closer    ::= 'decide' | 'arith' | 'eval' | 'ring' | 'congr' | 'solve' | 'lean' '=>' tacticSeq
point     ::= 'at' ('entry' | 'exit' | 'loop' k ('entry' | 'step'))
            | 'after' site
site      ::= 'call' k ('of' g)? | 'invoke' k | 'abort' k | 'op' k
obligation::= 'ensures' k | 'aborts_if' k | 'aborts_with' k
            | 'requires' k? 'of' g ('at' 'call' j)?
            | 'requires' ('of' g)? 'at' 'invoke' j
            | 'invariant' k ('of' 'loop' j)? ('entry' | 'step')
            | 'assert' k | 'data' 'invariant' | 'module' 'invariant' | 'stored' 'invariant'
            | 'lemma' 'requires' | 'unroll' | 'ghost' 'domain'
            | '_'
selector  ::= 'when' decision (',' decision)*
decision  ::= 'if' k ('then' | 'else')
            | 'match' k Variant
            | 'invoke' k 'is' (g | 'foreign')
            | 'loop' k ('exits' | 'continues')
            | 'aborts' 'at' site
```

A site names a construct of the body: `call k` the `k`-th call of a named
function, `invoke k` the `k`-th invocation of a function value (`f(x)`,
`invoke(f, args)`; [`higher-order-functions.md`](../designs/higher-order-functions.md)),
`abort k` the `k`-th `abort` or `assert!`, `op k` the `k`-th operation the
frontend marks as aborting. `at` a site means the construct itself: `requires
k of g at call j` is the precondition the call owes, `aborts at call j` a path
aborting in it. `after` a site means its continuation, the paths on which it
did not abort; after `abort k` that is divergence, reached by no path, so the
facts placed there owe nothing.
Facts after a site name only the variables in scope there: the result of an
operation or of a nested call is anonymous unless the Move code binds it.

`e` and `φ` are VerexSL specification expressions (`VerexSL/Syntax.lean`:
`x + 0 == x`, `len(v)`, `v[i]`, `spec_contains_key(m, k)`, `old(m)`,
`result`, `x is Position`, `x.index`, `∀ (i : Int), … ==> …`), the Lean-side
projection of Move's spec expression syntax that lemmas and in-body `spec
assert` already use. The proof file accepts this dialect only, not Move's
literal spec syntax (`forall i: u64: …`): its parser and translator exist,
and one dialect keeps one meaning per expression. `φ` is the lemma formula of
[`lemmas.md`](../designs/lemmas.md): `L(a)`, `c ==> φ`, `∀ xs. ψ`.

Meaning of the steps, each at the point or obligation whose block holds it:

- `let x := e` binds `e` for the following steps (substituted, as the Prover
  substitutes).
- `assert e by c` owes `e`, proved by the closer `c` from the facts in scope
  (the obligation's or point's context, the facts stated before it), then
  gives `e` to the following steps and to the closing of the obligation.
  Without `by`, the closer is `decide`.
- `assert e by contradiction { block }` runs the block under `!e` and
  closes when `decide` derives `false` from its facts
  (`lower_bound_scan_start`'s `Classical.byContradiction`).
- `assume e` gives `e` without proof: an in-body assumption of the target's
  theorem, in the assumption ledger, with a warning, as the Prover's and as
  `spec assume` is today. It is allowed only inside an `on` block, where it
  holds for that obligation alone; a point's block or the item's top level
  cannot assume, because a fact placed at a point holds for every obligation
  after it.
- `apply φ` is the Prover's step: `apply L(a)` owes `L`'s premise at `a` (by
  `decide`, or `assert` the premise first) and gives its conclusion; under
  `c ==>` both are guarded; `apply ∀ xs. ψ` gives the implication under the
  binders. `L` is a `spec lemma` of the module or of a module it uses, a
  recursive specification function (its unfolding is a lemma of it), or one
  of the built-in lemmas (below). A generic `L` is instantiated from the
  types of `a`; the specialized name the solver states it under
  (`L.lemma unit (Skolems.instantiate …)`, `g.at_θ.spec`) never appears.
- `split e` continues the block once per case: `true`/`false` of a Boolean,
  one case per variant of an enum or option, the variant's fields in scope
  under their Move names (`it.index`, not `it_Some_2`); for a function
  value, one case per closure site of its type in the verified instance,
  the captures in scope under the target's parameter names, and one case
  `foreign`. The function-value form exists once closures are data (step
  2b of [`native-function-values.md`](native-function-values.md)); over
  the subtype carrier a function value has no finite case set. `e` is any
  specification expression of these types, not only a local: the lowering
  generalizes it and keeps `e == V(x, …)` as a fact (`next_key_scan`'s
  `generalize …; rcases` on a neighbour key). `split i in a..b` splits an
  integer the context bounds by literals into one case per value, `i`
  substituted in each (`vector_hofs_fold`'s `rcases (show i = 0 ∨ … ∨ i = 6
  by omega)`, `drain_loop`'s three sizes, `test_contains`'s four indices);
  the bounds are owed by `decide`, and the case count is capped as `pragma
  unroll` caps iterations. The steps after a `split` run in every case; `if
  e { … } else { … }` guards steps by a condition without splitting the
  obligation's proof.
- `let V { f, … } := e` destructures a value the context shows to be of
  variant `V`: it owes `e is V` by `decide`, brings the payload fields into
  scope under their Move names, and gives `e == V { f, … }`
  (`leaf_walk_sum` and `leaf_iter`, whose `variantCarrier.second_of_name
  "Inner" "Leaf"` extracts the leaf a callee's ensures describes). Unlike
  `split`, it creates no cases.
- `calc(e₁ op e₂ op …)` asserts each step by the closer and gives the chain's
  conclusion.
- `forall x : T where g { block }` introduces `x` under `g` for the block and
  gives `∀ x : T, g ==> (every fact the block asserted)` after it. This is
  Dafny's `forall` statement and generalizes the Prover's `forall xs apply`.
- `witness e, …` supplies the witnesses of an existential obligation, or of
  an `assert` of an existential, whose body is then owed. The witnesses are
  spec expressions; their carrier certificates (`⟨3, by decide, …⟩` today)
  are the elaborator's.
- `point { block }` places facts at a program point: `at entry`, `at exit`
  (the Prover's `post`), `at loop k entry|step`, or `after s`, the
  continuation of a site `s`. The
  block's facts are owed once there and hold for every obligation
  after the point, on every path through it.
- `on obligation { block }` proves the named obligations: its facts hold in
  the obligation's context, and the obligation closes by `decide` over them.
  `_` takes every obligation no earlier `on` names. An obligation named by no
  `on` gets `decide` alone.
- `when d, …` after an `on` key or a point selects paths: the block
  applies to the paths consistent with every listed decision. A path goes to
  the first `on` in text order whose key and selector match it, so a block
  for selected paths precedes a general block for the same clause, and the
  general block takes the remaining paths. A proof can thus guide each path
  separately, down to a single path, without naming a traversal number (see
  "Obligation addressing").
- `lean => tac` runs a Lean tactic block on every goal the block has at that
  step: after a `split`, on every case, as every step does. It may leave
  goals open, which `decide` then closes. It is the escape for what the
  steps cannot say. Inside it the goal has the solver's prepared form, and
  the block is checked for runtime names as every authored proof is. It is
  permitted in every proof file, the framework's included.

Closers:

| Closer | Today's tactic | What it decides |
|---|---|---|
| `decide` | `verex_block_decide` on the block's normal form: a goal stated by a hypothesis (up to the spelling of instances, an equation either way round, a disjunct), closed by reflexivity or by a context that contradicts itself (a proposition beside its negation, a Boolean stated both true and false), an instance of a quantified hypothesis (its premises by `omega`), or linear arithmetic; first, the conclusions of the implications and equivalences whose premises are stated or true in the block's normal form, where they share a local with the goal (`impliedFacts`). The order is a cost, not a cascade: each alternative is one bounded check, a goal reading a map's positions or values tries the instance before `omega` | the default; bounded; no search, no budget |
| `arith` | `verex_omega_unsplit` alone: `omega` over the facts it reads without case splits and a few connected facts with one split; never the budgeted residual `omega` over the whole context that `verex_linear` adds, which is search the proof did not ask for | linear arithmetic with the stated bounds |
| `eval` | `verex_map_simp`, `verex_typed_close`, `verex_goal_witnesses`, `decide` | closed maps, literal vectors, ground arithmetic |
| `ring` | `grobner` (the `ring` module of `grind` alone: no splits, no instantiation, bounded by `ringSteps` and `ringMaxDegree`) on the goal with the hypotheses hidden | a polynomial identity `e₁ == e₂` over the integers; an equation it depends on is substituted first (`let`, a `calc` step) |
| `congr` | `lia (splits := 0)`: congruence closure and the linear module of `grind`, nothing else (`grind (splits := 0) (ematch := 0) -ring -linarith` is not this: it keeps the `ac`, `order`, `mbtc`, extensionality and `funext` modules on) | equality through function applications and uninterpreted terms combined with linear arithmetic, `f(a * b) == f(2 * b)` from `a == 2`; no open `if`, no existential |
| `solve` | `verex_solve`: the automatic leaf under its cap (`solveLeaf`: call observations, the leaf pipeline, the prepared leaf and each case of its variant split as an attempt of its own, `verex_open_prepared`, `verex_prepared_case`) | an explicit request for automation, charged as automatic work |
| `lean => tac` | the tactic | anything |

A closer that fails reports at the step the goal it did not close, in spec
form (see "Messages"). No closer tries another closer.

`ring` replaces the scripts' rewriting chains (`square_of_sum`'s `rw
[Int.add_mul, Int.mul_add, …]`, `newton_quotient`'s expansion and
reassociation) and the `grind` calls whose work is expanding products
before a linear step (`triangular_step`, `market_bounds`,
`ratio_monotone`): the proof states the expanded identity by `ring`, and
`arith` closes over the products as atoms. `congr` replaces the remaining
`grind` calls. Without splits it decides no `if` whose condition is open,
even with the same `if` on both sides, and proves no existential, so the
proof names the case and the witness: `pool_u64::buy_in`'s third
`ensures`, equal to a specification function whose body is an `if`, is a
`split` on that condition and `congr` in each case, the split its `grind
-ring only` makes today; `capability`'s `spec_contains` obligations are
`witness` or `forall` steps and `congr`.

### What each construct costs

Checked against the hand-written script, construct by construct; the rule
is that each lowers to the tactics the best script writes, or fewer.

- `assert e by c`: one `have` whose proof runs once in the block's scope.
  At a point that is once per path through the point; in an `on`
  block once per addressed obligation; the script's `have` costs the same
  where the author placed it well, and the language places it at the point
  by default. The elaboration of `e` is the clause translator's, the one
  that elaborates the contracts: proportional to the term, no `simp`, no
  coercion or instance search (instances are spelled, point 2 of the
  lowering).
- `apply φ`: a delta unfolding of the lemma's premise and conclusion
  (`unfoldLemmaDefinitions`), one bounded `decide` for the premise, and the
  conclusion as a fact. A generic lemma's instance is a declaration stated
  once per instantiation, as a specialized contract is; its contract is
  never normalized at the application. The script's `have := L.lemma …
  (by omega)` costs the same.
- `split e`: `verex_cases` or one `Decidable.em`, as the script's; placed
  at a point it splits the path once where the script splits each
  obligation.
- `forall`, `calc`, `witness`, `let`, `if`: an `intro`, a chain of
  `assert`s, a `refine`, a substitution, a `by_cases`; nothing the script
  does not write. `split i in a..b`, `let V { … } := e` and `by
  contradiction`: an `rcases` over the literal cases, one `obtain` of the
  payload, a `by_contra`, as the scripts write them.
- `on key`: a syntactic match of the key against the leaf's recorded
  provenance; no tactic runs to find the obligation. The script's
  `verex_on` costs the same; the script's `first | … | …` inside it does
  not exist.
- `when`: a lookup of the leaf's recorded decisions before the paths are
  combined, so an addressed path is never joined into a disjunction and
  taken apart again (`combineLeaves`, then `verex_paths`).
- `decide`, `arith`, `eval`: one bounded attempt each, the script's own
  tactics. An obligation the proof does not address gets one `decide`, as
  the script's `| _ => verex_decide`.
- `ring`, `congr`: one bounded attempt each, one `grind` module under a
  configuration without splitting or instantiation. Each replaces a script's
  `grind` with its full search; a use above that `grind` is a defect of the
  restriction, not a price.
- `solve`: the automatic leaf under its cap, charged as such; the one step
  that can cost more than a script line, by request.
- `lean =>`: what it writes.
- Spec-form rendering: delaborators run when a message or the InfoView
  prints a goal, never in a successful check.

### Worked examples

`ordered_map::test_verify_drain_symbolic`, today (`ordered_map.proof.lean`,
25 lines; the loop removes the first entry and the invariants describe the
remaining map):

```lean
verify test_verify_drain_symbolic by
  verex_obligations
  verex_on
  | invariant_1_step, invariant_2_step, invariant_3_step =>
    verex_prepare typed
    have present : SpecMap.hasKey m (SpecMap.keyAt m 0) = true := SpecMap.hasKey_keyAt (by omega) (by omega)
    have length : SpecMap.size m' = SpecMap.size m - 1 := by
      rw [m'_eq, SpecMap.size_remove, if_pos present]
    first
    | omega
    | (have shift : SpecMap.keyAt m' i = SpecMap.keyAt m (i + 1) := by
         rw [m'_eq, SpecMap.keyAt_remove_ordered,
           SpecMap.rank_keyAt m_valid (by omega) (by omega), if_neg (by omega)]
       rw [shift]
       first
       | simpa only [Int.add_assoc, Int.add_comm, Int.add_left_comm] using invariant_2 (i + 1) (by omega) (by omega)
       | (rw [m'_eq, SpecMap.valueAt_remove m_valid _ _ (by
            intro equal
            have := (SpecMap.keyAt_eq_keyAt_iff m_valid (by omega) (by omega) (by omega) (by omega)).mp equal
            omega)]
          exact invariant_3 (i + 1) (by omega) (by omega)))
  | _ =>
    first
    | verex_decide
    | (verex_prepare typed; all_goals (first | assumption | omega))
```

In the language:

```text
verify test_verify_drain_symbolic by
  at loop 1 step {
    let k0 := spec_key_at(m, 0)
    assert spec_contains_key(m, k0) by apply lemma_contains_key_key_at(m, 0)
    assert len(m') == len(m) - 1 by apply lemma_len_remove(m, k0)
    forall i : Int where 0 <= i && i < len(m') {
      assert spec_key_at(m', i) == spec_key_at(m, i + 1)
        by apply lemma_key_at_remove(m, k0, i), lemma_rank_key_at(m, 0)
      assert spec_key_at(m, i + 1) != k0 by apply lemma_key_at_eq_iff(m, i + 1, 0)
      assert spec_get(m', spec_key_at(m', i)) == spec_get(m, spec_key_at(m, i + 1))
        by apply lemma_get_remove(m, k0, spec_key_at(m, i + 1))
    }
  }
```

The three invariant steps close by `decide`: each is an instance of its
invariant at `i + 1` rewritten by the stated equalities, which the cheap
decider already does (`verex_instance`, rewriting by the context). The facts
are placed at the loop step, so they are owed once and every clause of the
step shares them; nothing names a path, a prepared form, or a Lean lemma.
`apply L(a), M(b)` applies several lemmas in one step.

`ordered_map::test_verify_iter_walk_mut_symbolic` (47 lines today, with a
`preparing` block of `verex_prepare values`, `verex_cases it`,
`verex_name_maps` and two `have`s, then three `verex_on` alternatives of
`first` cascades over `rcases Decidable.em`):

```text
verify test_verify_iter_walk_mut_symbolic by
  at loop 1 step {
    split it
    let k := spec_key_at(m, count)
    assert spec_contains_key(m, k) by apply lemma_contains_key_key_at(m, count)
    assert len(m') == len(m) by apply lemma_len_set(m, k, v)
    forall i : Int where 0 <= i && i < len(m) {
      assert spec_key_at(m', i) == spec_key_at(m, i) by apply lemma_key_at_set_present(m, k, v, i)
    }
  }
  on invariant 6 of loop 1 step {
    forall i : Int where 0 <= i && i <= count {
      split i == count
      assert spec_get(m', spec_key_at(m', i)) == (if i == count then v else spec_get(m, spec_key_at(m, i)))
        by apply lemma_get_set(m, k, v, spec_key_at(m, i)), lemma_key_at_eq_iff(m, i, count)
    }
  }
```

`big_ordered_map::test_verify_iter_collect_symbolic` (63 lines today: the
iterator block of `verex_cases it` and five `clear`s, the 12-step pipeline,
and five `verex_on` alternatives of up to eight-way `first` cascades over
`iter_next_ensures_k` and `internal_new_begin_iter_ensures_k`). The only
argument in it is `invariant_3_step`, that the collected vector extends by
the key at the iterator's rank:

```text
verify test_verify_iter_collect_symbolic by
  at loop 1 step {
    split it
    assert len(out) == spec_rank(m, it.key) by arith
  }
  on invariant 3 of loop 1 step {
    forall i : Int where 0 <= i && i < len(out) + 1 {
      split i == len(out)
      assert out'[i] == spec_key_at(m, i)
        by apply lemma_key_at_rank(m, it.key)
    }
  }
```

Everything the other alternatives do (`iter_next_ensures_7`, `omega`,
`verex_assumption`, the rank bound) is the cheap decider's work over the
callee's facts in the prepared context.

`square_of_sum` (e2e `SourceVerify/proofs.move`, 13 lines today: a
rewriting chain for the expansion, `Int.mul_nonneg` for the sign):

```text
verify square_of_sum by
  at entry {
    assert (a + b) * (a + b) == a * a + 2 * a * b + b * b by ring
    apply lemma_mul_nonneg(a + b, a + b)
  }
```

The identity closes the `ensures`; the sign closes the abort obligations,
whose overflow check is two-sided.

### Lemma items

A `lemma` item is a Move `spec lemma` ([`lemmas.md`](../designs/lemmas.md))
written beside the source instead of in it: the same `LemmaDecl`, the same
verification (assume `requires`, run the block, owe `ensures`), the same
recursion rule, so an `apply` of a lemma of its own recursion group is
induction under `decreases`. `apply` reaches the file's lemmas as it
reaches the module's; a lemma item's obligations are its clauses, `on
ensures k`. Lemma items replace the Lean theorems of today's proof files,
whose statements are all in the specification vocabulary once the
arithmetic and vector lemmas below exist. `effective_bound`, from the paper's
`amm_example`:

```text
lemma effective_bound(a : Int, scale : Int)
  requires 0 <= a && 0 <= scale && scale <= 10000
  ensures 0 <= a * scale / 10000 && a * scale / 10000 <= a
by {
  apply lemma_mul_nonneg(a, scale)
  apply lemma_mul_mono(a, a, scale, 10000)
  apply lemma_div_nonneg(a * scale, 10000)
  apply lemma_div_le(a * scale, a, 10000)
}
```

Each `apply` owes its premise by `decide` from the facts before it, and the
`ensures` closes by `decide` over the four conclusions, the products and
quotients as atoms. The swap function then states `apply
effective_bound(amount_in, 9500)` at the point the fee is computed.

### Lemmas

The lemmas of the intrinsic maps are a fixed table over the map roles every
owner's `pragma intrinsic = map` binds (`map_spec_get`, `map_spec_set`,
`map_spec_del`, `map_spec_len`, `map_spec_has_key`, `map_spec_key_at`,
`map_spec_rank`; [`intrinsic-maps.md`](../designs/intrinsic-maps.md)), each
entry a `SpecMap` theorem (`mapLaws`, `VerexSL/Verify/Steps.lean`): the
role pairs `valueAt_update`, `hasKey_update`, `size_update`,
`keyAt_update_present`, `valueAt_remove`, `hasKey_remove`, `size_remove`,
`keyAt_remove_ordered`, `size_empty`, `hasKey_empty`, `hasKey_keyAt`,
`rank_keyAt`, `keyAt_rank`, `keyAt_update_before`, `keyAt_update_after` and
`rank_lt_size`, and the single roles `keyAt_eq_keyAt_iff`, `rank_nonneg`,
`rank_lt_rank` and `keyAt_lt_of_lt`. Each lemma is named `lemma_` and the
words of the owner's bindings (`lemma_get_set`, `lemma_len_remove`, `lemma_contains_key_key_at`,
`lemma_key_at_eq_iff` for `ordered_map`), so the names come from the roles,
never from a module name. The language reaches the lemmas by `apply` only.
The module's copy that lowers the step expressions declares each lemma as a
`spec lemma` with its signature alone, which types an application's
arguments: the map at the owner's type, keys and values at its type
parameters. At the leaf, `apply L(a)` instantiates the lemma's `SpecMap`
theorem at the translated arguments (`lawInstance`): the map as its
owner's typed map, a key or a value as the typed value the map holds, the
discipline as the owner gives it. Each parameter of a lemma names the binder
of the theorem it fills, and the table is checked against its theorems when
the verifier is built: every parameter fills a binder, and every other
binder is the discipline, the order, a type, an instance or a premise. The
theorem's premises are opaque goals, each normalized and proved by the
step's closer; its statement, normalized, is the fact. No alias theorem
and no generic frame is stated for an application.

Of the 25 distinct `SpecMap` lemmas the corpus's proofs apply, the table holds
eighteen. Three (`keyAt_satisfies`, `keyAt_satisfies_all`,
`hasKey_satisfies`) are applied only to type bounds of keys, which
preparation supplies ("Normal-form fixes"). Four (`nextKey_none`,
`nextKey_some`, `valid_slice`, `keyAt_of_keys`) state the next key, a map's
validity and its key list, which no role names; only `big_ordered_map`'s
proofs apply them, and they enter the table with that module's rewrite
(L4), with the roles or specification functions that state those notions.
A prelude of role-generic `spec lemma`s is added only if a Move proof block
must `apply` a map lemma; Move has no construct that quantifies a `spec
lemma` over a role's binding.

**Arithmetic lemmas.** A fixed table of integer lemmas over the specification's
`*`, `/`, `%`, `<<` and `>>`, with Move's semantics, reached by `apply` as
the map lemmas are. The corpus's Lean lemmas map onto these:

| Lemma | Statement | Replaces |
|---|---|---|
| `lemma_mul_nonneg(a, b)` | `0 <= a && 0 <= b ==> 0 <= a * b` | `Int.mul_nonneg` |
| `lemma_mul_mono(a, b, c, d)` | `0 <= a && a <= b && 0 <= c && c <= d ==> a * c <= b * d` | `Int.mul_le_mul`, `mul_le_mul_of_nonneg_left/right` |
| `lemma_mul_strict_mono(a, b, c)` | `a < b && 0 < c ==> a * c < b * c` | `mul_lt_mul_of_pos_left/right` |
| `lemma_div_nonneg(x, c)` | `0 <= x && 0 < c ==> 0 <= x / c` | `ediv_nonneg`, `tdiv_nonneg` |
| `lemma_div_le(x, q, c)` | `0 <= x && 0 < c && x <= q * c ==> x / c <= q` | `ediv_le_of_le_mul` |
| `lemma_div_lt(x, q, c)` | `0 <= x && 0 < c && x < q * c ==> x / c < q` | `ediv_lt_of_lt_mul` |
| `lemma_le_div(x, q, c)` | `0 <= x && 0 < c && q * c <= x ==> q <= x / c` | `le_ediv_of_mul_le` |
| `lemma_div_mono(x, y, c)` | `0 <= x && x <= y && 0 < c ==> x / c <= y / c` | `ediv_le_ediv` |
| `lemma_div_mul_le(x, c)` | `0 <= x && 0 < c ==> x / c * c <= x` | `ediv_mul_le` |
| `lemma_div_add_mul(x, b, c)` | `0 <= x && 0 <= b && 0 < c ==> (x + b * c) / c == x / c + b` | `add_mul_ediv_right` |
| `lemma_mod_small(x, c)` | `0 <= x && x < c ==> x % c == x` | `emod_eq_of_lt` |
| `lemma_shl_mul(x, k)` | `x << k == x * 2^k` | `shiftLeft_eq` |
| `lemma_shr_div(x, k)` | `0 <= x ==> x >> k == x / 2^k` | `shiftRight_eq_div_pow` |

The specification's `/` and `%` elaborate to `Int.tdiv` and `Int.tmod`
(truncation toward zero, as Move), `<<` and `>>` to the exact
`Int.shiftLeft` and `Int.shiftRight` ([`numeric-representation.md`](../designs/numeric-representation.md):
specification arithmetic is mathematical). Where truncation and Euclidean
division differ, a lemma requires nonnegative operands, and its Lean proof
carries `tdiv_eq_ediv_of_nonneg`, which no proof states again; every lemma
of the table has such a proof. The body's shift at a width `w` is `(x *
2^k) % 2^w` in an obligation, which the normal form already reduces to `x *
2^k` where the value fits (`shiftLeft_emod_of_fits`); otherwise `lemma_mod_small`
applies. The lemmas are a prelude of VerexSL `spec lemma`s proved once in
Lean, as the map table's alias theorems are. A lemma is stated in one
operand order; the `apply` instance and the obligation elaborate through
the same normal form, so a commuted product never meets its lemma's
spelling. `arith` already treats a product as an atom, so the corpus's
pattern, a product bounded by a lemma and the rest linear, needs nothing
more.

**Vector lemmas.** A table over Move's vector operations, as the map table
is over map roles: a length lemma and an element lemma for `update`,
`push_back`, `pop_back`, `swap`, `remove`, `swap_remove`, the slice
`v[a..b]` and `concat` (`lemma_vector_len_remove(v, i)`; `lemma_vector_index_remove(v, i, k)`: `k < i
==> v'[k] == v[k]` and `i <= k ==> v'[k] == v[k + 1]`), and extensionality,
`lemma_vector_ext(v, w)`: `len(v) == len(w) && (forall k: 0 <= k && k < len(v) ==>
v[k] == w[k]) ==> v == w`. `verify_vector`'s `swap_erase_last` becomes a
`forall` block with a `split k == i` over the element lemmas of `swap`,
`remove`, `update` and the slice, then `apply lemma_vector_ext`, the argument it
makes in Lean over `Array.ext`; `acl`'s `erase_unique` is two `split`s
(each index before or after the removed position) and, per case, one
instance of the uniqueness invariant over `lemma_vector_index_remove`. The decider's
`Array.getElem?_push` rewriting, which the `iter_collect` example above
relies on, becomes `lemma_vector_index_push_back`, applied by the proof or kept in
`decide` as a stated rule.

User `spec lemma`s and recursive specification functions are reached the
same way: a recursive specification function's definition is a lemma of it
(`apply spec_test_sum_upto.unfold(v, n)` for today's
`spec_test_sum_upto.spec.unfold`).

Lemmas of function values come after step 3 of
[`native-function-values.md`](native-function-values.md), when a closure
is a constructor of its type and the requires table is a definition per
constructor: the invocation of a closure site is its target's contract at
the woven arguments, `requires_of<f>(x)` of a site is the target's
`requires`, and equality of two function values is structural up to ghost
fields. Today's weave and composition lemmas
(`closureMeaning_closureOf_*`, `requiresOf_closureOf`) go with that step
and never enter the table. Among them is the lemma of a function value's
frame: the stored invariants of the resources outside the frame hold after
the call (`stored_fun_values`'s `stored_invariants_outside`, over
`MemoryInvariants`).

## Obligation addressing and provenance

An obligation key is what `on` names and what every message prints:

| Kind | Key | Today's tag (`leafName`, `Solve.lean`) |
|---|---|---|
| Postcondition, abort condition | the clause's kind and index in its `spec` block | `ensures_2`, `aborts_if_1`, with a traversal suffix per path |
| Callee precondition | `requires k of g at call j`: the clause of `g` and the `j`-th call of `g` in the body; without `k` where no clause of `g` states it (a data invariant of an argument, a map role's precondition) | `requires_1_of_g_at_call_2` (done) |
| Continuation after a call | an `after call j of g` point | `after_g` for every call of `g` |
| Precondition of an invocation through a function value | `requires at invoke j` (`requires_of<f>(x)` of the value, [`higher-order-functions.md`](../designs/higher-order-functions.md)); after step 3 of [`native-function-values.md`](native-function-values.md) it splits per closure site, `requires of g at invoke j`, and `foreign` keeps the uninterpreted key | `requires_of_g` under the target's name where the value is a known site, else no clause name |
| Continuation after an invocation | an `after invoke j` point | none |
| Continuation after an aborting operation | an `after op j` point | none |
| Loop invariant | `invariant k of loop j entry|step`: the clause and the loop's site index | `invariant_k_entry`, `invariant_k_step`, the loop not named |
| In-body assertion | `assert k`: the `k`-th `spec assert` of the body | `assert_k` |
| Data, module, stored invariant; lemma requirement; unroll bound; ghost domain | the kind, and the site where several | `data_invariant`, `module_invariant`, `stored_invariant`, `lemma_requires`, `unroll`, `ghost_domain` |
| A path to any of these | the key with a selector, `when` and the decisions on the path | the traversal suffix `_1_8`, `.inl`, `_1`; `leaf`, `leaf_2`, `search_2_2` where no clause names the goal |

Done: a callee's precondition is keyed by its clause and call. The
contract marks each `requires` clause, the walker strips the marker into
the leaf's clauses as it does a loop invariant's, and the generator numbers
every function's preconditions and the target's calls in source order
(`numbering`, `calling` of `verex_obligations`); `verex_on` takes keys
beside tags (`ObligationKey`, `Solve/Base.lean`), and a failed
precondition is reported at its clause and call. The template keeps tags
until L3 rewrites it.

Done as well: decisions and selectors. The compiled body carries the VIR
site of each conditional, throw, aborting operation (checked arithmetic, a
cast, a vector access, a resource access), loop, invocation and `match` arm;
the denotation marks a branch, an arm, an abort and an operation's or call's
failures definitionally (`branchAt`, `armAt`, `abortAt`, `opAt`, `callAt`),
and the markers reach the goals as reducible conclusions (`Taken`,
`Matched`, `AbortsAt`), so the normal form and the deciders see through
them. The walker takes them off where it steps a goal and records the
decisions on the path, a field of the leaf; `verex_on … when …` takes a
clause's paths apart where a selector names them. The generator numbers
the sites per kind in source order (`siting`); a conditional is an `if` only
where the source writes one, not an `assert!`, `&&`, `||` or a `while`
loop's heading, which is the loop's `exits`/`continues`; the compiler
leaves the others unmarked, so the short-circuit rules still merge them. An
invariant clause keys a leaf that owes it (`data invariant`, `module
invariant`, `keying`) where its step names no invariant itself. A selector
taking no path is reported once the target's proof ends, since a proof can
run an alternative once per case of an enum parameter.

A `match` lowered to variant tests keeps its provenance on the arena node
(`Expr.origin`, `ExprOrigin.matchArm`: the match's location, the arm's
variant, the untested last arm's variant), set by VerexSL's decision trees
and by the import pass for patterns through references; the site table
numbers it as one `match`, and its arms record `match k V`. The source
printer writes such a chain, and a match node it lowered itself, as
`match`, so a Move source's verification, which elaborates the printed text,
keeps the decisions. Two forms keep conditionals: a match of a move-only
enum by value, which VerexSL has no non-consuming `match` for, and an arm
testing more than one variant at a time (a nested pattern, a guard); their
paths take `if` decisions.

Open: a success path owes that the abort conditions do not hold
(`¬mustAbort`), built unmarked, so those leaves keep the key `leaf`
(`nested_loop_inv`). Marking the clauses of `mustAbort` keys them `aborts_if
k`, but every variant tried changed the solver: the markers reach the
shared-continuation owners, callers' hypotheses (clauses inside markers are
taken as normal, so a caller stops normalizing them), and, where `mayAbort`
stays unmarked, the syntactic identity of the two conditions the normal
form relies on. It needs a design that keeps the marker out of every
hypothesis and owner.

`Provenance` (`Solve/Base.lean`) gains the site index at `precondition`,
`continuation`, `loopEntry`, `loopIteration`, `assertion` and the memory
kinds, and `precondition` gains the clause; an invocation through a
function value gets its own kind, the invocation site and, where the
value's constructor is known, the target and clause, so that an invocation
is never recorded as a call of its target; the continuation of an aborting
operation gets its own kind and the operation's site. `leafName` renders the key;
`leafClauses` keys goals by it. The counting of calls (`call j of g`) is by
source order of the call expressions in the body, and `call j` without `of`
counts every call; invocations are counted the same way, apart from calls;
all are stable under solver changes. The invocation provenance is the one
piece of the solver both this plan (L0) and the function-value plan touch:
L0 records the site, and the function-value plan's step 3 records the
constructor's target under it.

Paths are addressed by their decisions, not by traversal numbers. The walker
records, for each leaf, the source decisions on its path: the `k`-th `if` of
the body taken `then` or `else`, the `k`-th `match` taking a variant, the
`k`-th loop exiting or continuing, the `k`-th invocation dispatching to a
closure site of `g` or to `foreign` (a decision only once closures are
data, step 2b of [`native-function-values.md`](native-function-values.md)),
and where the path aborts, its abort site: the `k`-th `abort` or
`assert!`, the `k`-th operation the frontend marks as aborting (checked
arithmetic, a cast, an index, a resource access), the `j`-th call (of
`g`), or the `j`-th invocation. Sites are counted per kind in source order of
the body, as calls are, so a selector changes only when the Move source
does. Without a selector, the paths owing one clause are one obligation,
proved under the disjunction of their contexts (`combineLeaves` builds it
today under `verex.combine`); a selector takes the paths it matches out of
that disjunction into its own block. `split` and `if` remain for case
analysis that is not a program path. A selector that matches no path is an
error that lists the decisions of the key's paths.

`pool_u64::buy_in` named its two abort paths by traversal number
(`case aborts_if_1_8`, `case aborts_if_1_10`); they are the overflow of
`self.total_coins += coins_amount` and the abort in `add_shares`. With
selectors:

```text
verify buy_in by
  on aborts_if 1 when aborts at op 3 {
    assert old(self).total_coins + coins_amount > MAX_U64 by arith
  }
  on aborts_if 1 when aborts at call 1 of add_shares {
    split spec_contains_key(old(self).shares, shareholder)
  }
```

Points use the same sites: `after call j of g` is the continuation of the
`j`-th call of `g`, `after op j` (`after invoke j`) the path on which the
`j`-th aborting operation (invocation) did not abort, `at loop j entry|step`
the loop's entry and the start of its iteration body, `at entry` the first
step of the body, `at exit` every return (the Prover's `post`). An abort has
no continuation, so `after abort j` names no point. A point is where the walk
is, not what a leaf owes: it never changes a leaf's provenance or key. The
walker runs a point's hooks where it produces the point's goals: at the loop
rule's application and the iteration's start; after a call or operation a
proof names, at a marker the walker places on the continuation's conclusion
(`AfterAt`, opaque, so that the normal form rewrites under it but never
distributes the path across it), met on every path that continues there,
however the normal form duplicates or decides the paths around it. A hook
at a point no path reaches is an error. Points are numbered as the
decisions are (`loop j` by the source order of the loops; an invariant's
key names its loop, `invariant 2 of loop 1 step`, where the function has
several).

## Semantics and lowering

The language lowers onto the solver's existing authored machinery; nothing
in the walker, the contracts, the budgets or the reporting of automatic
targets changes in the first milestones. The lowering is a translation to
the tactics a script writes, not a tactic of its own: it emits no `first`,
no `try` over alternatives, and no speculative decider, and it prepares each
goal at most once (the path's hook or the obligation's alternative, never
both; a goal prepared in an outer scope is marked, as `preparationOwners`
marks it, and an inner scope reads the mark).

1. **Items.** `verify f by block` owns the target's whole goal, no automatic
   attempt first, as today. The lowering is
   `verex_obligations preparing P` followed by routing, as `verex_proof
   preparing` does today (`Authored.lean`), where `P` is the preparation the
   block needs (below) and the routing is `verex_on` over the keys. A
   `lemma` item lowers to a `LemmaDecl` of the unit and is verified as a
   Move `spec lemma` is.
2. **Expressions.** A step's `e` is elaborated by the clause translator
   (`VerexSL.Contract.Clauses`) in a scope built from the goal's context: the
   source-named locals and parameters `verex_prepare` establishes (`m`,
   `count`, `it`, `old_m`, `m'`, callee results under the local they are
   bound to), `result`, the obligation's own binders, `let` bindings of
   the block, and under `split`, a variant's fields. The translation of
   `spec_contains_key(m, k)` is `SpecMap.hasKey m k = true` over the same
   `m` the goal holds (`verex_name_maps`); no transport. The elaborator
   spells instances as the normal form spells them in the leaf (the
   `DecidableEq` instance of a concrete key as `NTy.decEq carriers τ`), so
   that a stated fact rewrites the leaf's terms. This settles the
   key-instance spelling for authored proofs: the proof works on the leaf's
   own terms, and the elaborator does once what every proof would otherwise
   do by hand; the normal form keeps its spelling, and no automatic proof
   changes ([`roadmap.md`](roadmap.md), "Key-instance spelling", for the
   automatic side). A function-typed expression elaborates to the
   `Carriers.functionCarrier` interface of
   [`native-function-values.md`](native-function-values.md), step 2a:
   construction, invocation, equality and order through it, never
   `ClosureValue`, `NTy.encode_function`, `Weave` or `Which`.

   The expressions are lowered by the module's own lowering, in an
   unregistered copy of the module that declares one specification
   function per expression (`f__step_k`) whose parameters are the names in
   scope: the function's locals by source name, each also as `x'` and as
   `old_x` (what `old(x)` reads), among them the binders of its clauses'
   quantifiers; each callee's result as the leaf names it: in evaluation
   order the last call's `g_result`, the earlier ones' `old_g_result`,
   `old_g_result_1`, …, a tuple's components `…_k` (a name given twice is
   an error at the item); the binders of the enclosing `forall` steps. A
   `let` is substituted. A `witness` has the type of the existential binder
   it supplies, every other expression is a Boolean. The copy's syntax
   holds the proof file's items at positions shifted into a band of their
   own, and the map lemmas in a third: the lowering records each range in its
   source's file (`CompilationUnit.sources`, `sourceBand`), and a
   diagnostic of a step expression is reported at it. At the leaf, after
   its preparation, the binders of the obligation's clause, which the
   preparation introduces without names, take the clause's binder names.
   A name of the scope reads its scope entry (`ScopeEntry name index
   value`), an inert fact created where the walk binds the name: a
   parameter at the theorem's introduction, before its pattern takes it
   apart; a call's result as a premise under the binder the call names, so
   that a substitution eliminating the binder keeps the entry with its
   value; a loop's slot value; a clause's binder; the prophecy of a borrow
   of a whole local `x`, a `&mut` parameter's reborrow included, as `x'`,
   the value written back (the borrow's `Prophecy` mark, which the walk
   consumes where it introduces the value; `denotation.md`). A case split
   or a substitution rewrites the entry as it rewrites the leaf, so an
   enum's value is in scope in every case, a variant without fields
   included; a renaming of the local renames its entry (`old_g_result`),
   and the latest binding of a name, by index, is the one in scope: at an
   `ensures`, `x'` is the last value written back, the final one. `old_x`,
   what `old(x)` reads, is the earliest binding of `x`, its value at the
   function's entry, where no binding is named `old_x`. A mutable
   reference's entry holds its current value. Generic introductions bind
   no entry: an operation's lemma binds values under generic names. A
   value the leaf holds only in parts has no entry, and its name reads the
   parts under the names the leaf gives them: a structure's or a tuple's
   components (`x_f`, `x_k`). A frame
   local the walk never binds (a Move local assigned a call's result) is
   read by the result's name. `result` is not in scope yet; the leaf's names
   for a function's results come with the templates (L3). A call's result is named after
   the callee (`g_result`) and its clauses after the callee's (`g_ensures_1`);
   those of an earlier call of the same callee keep old names
   (`old_g_result`, `old_g_ensures_1`), so no fact is hidden.
3. **Facts.** `assert e by c` is `have h : ⟦e⟧ := by ⟦c⟧` on every goal of
   the block's scope, `h` named by the step's index (`fact_3`) and marked a
   fact of the block by a hidden `BlockFact` beside it, which a substitution
   or a case split carries along; it is reported at the step on failure.
   The fact is normalized once, as a term, and that normal form is proved
   and given (an implication's premises decide its conclusion's
   conditionals). An equation of a name and a value or another name is
   substituted over the context; one of a term and a name folds the term
   into the name over the context, once; any other fact is a rule of the
   block's normal form and rewrites no hypothesis. The rules rewrite a
   fact's higher side to its lower one, a term above a name above a value
   (two names toward the one declared first), and a name equated to a term
   holding it is no rule; the leaf's definitions of named values, a name
   equated to a term on either side, fold the same way. Every rule thus folds a term into a
   name, takes a name to an older one or to a value, or is a normal-form
   rule, so the normal form ends. A conditional reduces where its
   condition, or the condition's negation in canonical form (`b ≤ a` of `a
   < b`), is a hypothesis, a conjunction where each conjunct is or one is
   refuted: the facts decide the conditionals they state, by
   their spelling, and no arithmetic runs in the normal form; a condition
   the facts only imply is stated by the proof. `apply φ` is the lemma machinery of
   [`lemmas.md`](../designs/lemmas.md) (`verex_lemma_owed`,
   `verex_lemma_holds`; `LemmaApplication`, `LemmaInstance`) at an authored
   site: the premises as `assert`s by `decide`, the conclusions as facts. A
   generic lemma is instantiated by `ensureSpecialization` from the
   argument types (`native-verification.md`, SC1–SC5), as a call site is.
   `assume e` has no lowering yet and is reported as unsupported: the
   target's theorem states its hypotheses before the walk, so a fact over
   an obligation's prepared scope cannot be one of them, and a node in the
   body would change the function's semantics. The design for it is a
   trusted contract per function: the proof file's `assume`s, each under
   `on ensures k`, `on aborts_if k` or `on aborts_with k` and without a
   selector, read as clauses of that kind over the clause vocabulary
   (`old_x`, `x'`, `result`), marked by key and step, and the hypothesis
   that the function's runs satisfy them, beside `AssumptionsHold` in the
   theorem, in its callers' theorems and in the ledger
   ([`assumptions.md`](../status/assumptions.md)); the obligation's proof
   then receives its own clauses' assumptions as hidden premises, which the
   `assume` step names. `witness` is `refine ⟨⟦e⟧, ?_⟩` at the obligation's
   existential, a step of an `on` block naming one obligation: each
   witness lowers at the type of the clause's existential binder (of a
   callee's precondition, `requires k of g at call j`, at the call's type
   arguments), and is given at the binder's type: as it is, as the typed
   value a typed view holds it as, as an integer with its certificate, or
   as its native value; a bounded integer's range (`∃ fits : m, …`) and a
   certificate are owed by `decide` at the witness. `calc` is a chain of `assert`s: each link a fact `fact.k_j` by
   the closer, the conclusion `fact.k` by `decide` from them. `forall x
   where g { … }` is `have : ∀ x, g → F := by intro x hg; …` with `F` the
   conjunction of the facts the block asserts (its `assert`s, its `calc`s'
   conclusions, its `forall` and `if` steps' facts; an `apply` is the
   block's means, not its fact), closed by `decide` after the block's
   steps; a bounded binder's range is a premise beside `g`. `if g { A }
   else { B }` is the same without binders: it gives `g → F_A` and `!g →
   F_B`, the obligation's proof unsplit. `assert e by contradiction { … }`
   is `by_contra` with the block's steps after it and `decide` on
   `False`.
4. **Cases.** `split e` on a Boolean is `rcases Decidable.em ⟦e⟧`; on a
   variant value it is `verex_cases x` with the payload fields renamed to
   their Move field names, and the block continues on every case
   (`verex_each`); on a function value it is `cases` over the generated
   `FV_τ`, the captures renamed to the target's parameter names, available
   after step 2b of [`native-function-values.md`](native-function-values.md).
   A split expression that is not a local is generalized first, in the
   goal and the hypotheses that hold it (`generalize h : ⟦e⟧ = x at …`), `h`
   kept as the case's fact; it lowers as a specification function without
   a declared result type, whose body's type is the result. A case fact the
   normal form states as an existential or a conjunction (an optional model
   value a role's result reads, `∃ a, o = some a ∧ …`, by
   `Option.elim_eq_iff`) is taken apart into its witnesses and facts, which
   are normalized and settled under the witnesses' bounds. The equations of
   names and values a split leaves in a case (a call's result equal to the
   variant's payload) are settled as a fact's are. A case of a term that
   reads a map's model gets the facts the map's positions give, a key's
   neighbour included (`nextKey_none`, `nextKey_some`, which the solver
   instantiates at a leaf's neighbour facts as it does the position facts);
   a split of a local gets none, and a proof states what it needs of the
   positions by the built-in lemmas. `split i in a..b` is an `rcases` over `i = a ∨ … ∨
   i = b`, owed by `decide`, with `subst` per case. `let V { f, … } := e`
   owes `e is V` by `decide`, then generalizes and splits `e` as a split
   does, the cases but `V` refuted by the owed fact; a field `f` reads
   `(e).f` in the steps after it, which the case's fact reduces to the
   payload. `if` guards without splitting (item 3).
5. **Points.** `at p { … }` and `after s { … }` run the block's steps in the
   `preparing` hook at the goals whose provenance is the point: today's `verex_obligations preparing`
   runs at every completed path before its clauses split; the hook is
   generalized to run at the continuation after a site (no goal after `abort
   k`), at a loop's entry and step, and at exits, keyed by
   provenance (`splitStep` already distinguishes `.continuation`; an
   operation's continuation, the path on which its bind does not abort, is a
   new kind). Facts asserted there are owed once per path through the
   point and inherited by the obligations after it, including the clauses
   that split later. This is the mechanism the map proofs use today by hand
   (`have present …; have sizes …` in the `preparing` block of
   `iter_walk_mut`). An `assert e` of a point step is lowered as the Move
   proof block's in-body assertion is (`lemmas.md`, "Function proofs"): an
   obligation of its own, `assert k of p` (the `k`-th assert at the point
   `p`, `assert 1 of at entry`, `assert 2 of after call 1 of g`), which the
   walk queues as a leaf where the path reaches the point, routed like any
   obligation, and given to the paths after the point; a closer on it, `assert
   e by c`, is shorthand for `on assert k of p { assert e by c }`. The other
   steps of a point block (`let`, `apply`, `split`, guarded blocks) run in the
   hook, their facts given there.
6. **Obligations.** `on key { … }` is a `verex_on` alternative that first
   prepares the obligation (today's `verex_paths; verex_finish_prepare;
   verex_prepare typed`), runs the block, and closes by `decide` with the
   facts, in the block's normal form (the facts and the leaf's definitions
   of named values as rewrites), with no decider after it. `_` is the
   default alternative; an unnamed obligation gets `try verex_decide`, as
   today. A selector restricts the alternative to the
   leaves whose recorded decisions match it, before `combineLeaves` joins
   the rest; a point with a selector runs its hook on the matching
   paths only.
7. **Escape.** `lean => tac` runs `tac` on the goals of the block's scope.

Which of today's tactics become the language, which stay internal, which
leave the surface:

| Today | Under the language |
|---|---|
| `verex_obligations`, `verex_obligations preparing`, `verex_proof preparing` | the item and points |
| `verex_on`, `verex_group`, `verex_group_contexts` | `on`; grouping is implied by points and by `on` with several keys |
| `verex_paths`, `verex_intro_any`, `verex_finish_prepare`, `verex_defined_values` | internal to the lowering |
| `verex_prepare` and its variants (`only`, `values`, `typed`, `values splitting [..]`, `typed splitting [..]`, `cases [..]`), `verex_prepare_leaf`, `verex_typed_prepare`, `verex_normalize_context`, `verex_normalize`, `verex_canonical_families`, `verex_split_hypotheses`, `verex_subst_vars`, `verex_reduce_projections`, `verex_type_bounds`, `verex_name_maps`, `verex_unname`, `verex_clear_computations`, `verex_consume`, `verex_lookups_after_writes`, `verex_scalar_equalities` | internal; chosen by the lowering from what the block needs (`split x` selects the variant preparation of `x`; a map lemma selects the named maps) |
| `verex_cases`, `verex_split`, `verex_split_variants`, `verex_split_search` | `split` |
| `generalize …; rcases`, `rcases (show i = 0 ∨ … by omega)`, `variantCarrier.second_of_name`, `Classical.byContradiction` | `split` of an expression or a range, `let V { … }`, `by contradiction` |
| `grind`, `grind only [..]`, `simp_all; grind`, rewriting chains over `Int.add_mul`, `Int.mul_add` | `congr`, `ring` |
| `theorem`s and `@[vir_denote_norm]` rules in proof files | `lemma` items; the rules go to the normal form ("Normal-form fixes") |
| `verex_decide`, `verex_assumption`, `verex_leaf` | the `decide` closer |
| `verex_omega_unsplit`, `omega` | `arith` |
| `verex_linear`, `verex_arithmetic_only` | internal to the automatic leaf; reached through `solve` |
| `verex_map_simp`, `verex_typed_close`, `verex_goal_witnesses`, `decide` | `eval` |
| `verex_solve` | `solve` |
| `verex_lemma_owed`, `verex_lemma_holds`, `verex_lemma_decreasing` | the lowering of `apply` |
| the closure vocabulary of the paper examples' proofs (`NTy.encode_function`, `Weave.mask`, `Which.project?_eq_some_iff`, `Which.inject_here`, `packResults_single`, `closureMeaning`) | leaves the surface: a function value is written as the specification writes it (`function[f](captures…)`, `f(x)`, `requires_of<f>(x)`) and elaborated to the function carrier interface; the lemmas go with the function-value plan's steps 2b and 3 |
| the 130 solver internals that no proof uses (`verex_pipeline`, `verex_saturate`, `verex_open_prepared`, `verex_prepared_case`, …) | unchanged, unreachable except through `lean =>` |

The `verex_*` names stay defined, for the lowering and for `lean =>`; the
gate is that the corpus uses none directly.

## Who owns preparation

The system. A proof never says how a context is prepared; it says which
values to split and which facts to state. The lowering chooses the cheapest
preparation the obligation needs: `verex_prepare values` for every leaf,
and the map naming (`verex_name_maps`, which gives a map's size bounds and
typed reads) where the block or the obligation reads a map: a name the
block reads holds a map, a step applies a map lemma, or the clause of one of
the `on`'s keys has a map's type in it (under `on _`, the leaf's own
clause; a key naming no clause, by the obligation's statement). A proof
over maps whose clauses read none states a fact that reads one where it
needs the maps' bounds. A `split` names the values it splits. These are
today's best-practice sequences (`verex_proof preparing verex_prepare typed
splitting […]`; the by-hand form of the same steps measured 13% dearer on
`upsert` on encoded leaves), so the first lowering costs what the best
scripts cost today, less the preparation the scripts repeat.

A completed path whose obligations several `on` steps take is prepared once
before its clauses split (`verex_step_prepare`, in the walk's `preparing`
hook), its clauses' markers kept, since they name the leaves; each leaf then
adds only what its clause brings (its binders and premises, their bounds),
reads its clauses unmarked (`verex_unmark`) and reduces the projections a
substitution into them leaves. A path with one such obligation is left to
its leaf: preparing it twice, once for the path and once for the clause,
costs more than once (`aborts_if_new_from_2`: 10.7M against 13.8M).

Incremental preparation then becomes a change inside the lowering. Once the
rule set commutes with substitution and the walker can carry a path's
prepared context ([`performance.md`](../designs/performance.md), "Open
levers", 1), the point hooks prepare a fact when it is introduced and the
obligations inherit it; no proof text changes, because no proof text
mentions a normal form. The language does not depend on that change for
correctness and must not wait for it; it is what makes the change possible
without rewriting the corpus again.

The facts a proof states at a point are prepared once with their context and
shared by every obligation after the point, the saving `preparing` blocks
give today by hand, now the default placement. The design asks authors to
prefer points over `on` wherever a fact is not clause-specific, as
`performance.md` recommends ("Authored proof interface": facts at program
points instead of scripts over presented leaves).

## Normal-form fixes

Some escapes exist because a solver gap shows through the proof. They are
fixed in the normal form or in preparation, not by a construct, and the
proofs stop naming them. Each is a change of the normal form, its own step
gated on the automatic targets (N1 and N2 in "Milestones"):

- **Transports of concrete types.** The normal form states `NTy.toSkolem θ
  .address v = v` and `NTy.toSkolem θ (.param i) v = v` (`Types.lean`,
  `toSkolem_address`, `toSkolem_param`) but not the unapplied `NTy.toSkolem
  θ .address = id`, which `capability` needs where the transport is mapped
  over a vector in a generic search and registers from the proof file
  (`address_transport`, `parameter_transport`). The unapplied forms join
  the normal form. A proof file then registers no normal-form rule: the
  other registration, `for_loop_invariants`'s `triangular_div`, is the
  quotient spelling below.
- **Type bounds of values read from typed containers.** A key read by
  `spec_key_at` or returned as a neighbour key, and an integer payload field
  of a variant, lie in their Move type's range. The proofs derived this
  (`keyAt_satisfies_all`, `keyAt_satisfies`, `hasKey_satisfies`,
  `SpecInt.val_ofInt`, `IntegerValueFits_unsigned_succ`,
  `oit_Position.fits`). Preparation adds these bounds as it adds those of
  parameters (`verex_type_bounds`): from a map's `KeysSatisfy` and
  `ValuesSatisfy` facts, the bounds of a key at a position, a key stated
  present, a successor or predecessor (a key of the map by definition,
  `nextKey_satisfies`, `prevKey_satisfies`, with no presence premise) and a
  value at a key; after a variant split, the payloads' bounds. It then reads
  the round trip `SpecInt.ofInt` of a bounded value as the value, by the
  cut's lemma at the bounds, in the facts that mention one. A read of the
  key vector at a position is the key at the position (`getElem_keys_toNat`),
  so the vector `keys` returns has no bounds of its own.
- **The specification form of `AbortsNewFrom`.** The model states the
  abort of building a map from vectors over its key list (`Nodup`); the Move
  contract states duplicate indices. Over a vector of integer keys the
  authored preparation and the block's normal form read the first as the
  second (`abortsNewFrom_indexed`, in the set `vir_denote_spec`); the
  automatic normal form keeps the atom, as it keeps every role abort
  predicate, which its lemmas and deciders consume and its context rewriting
  would otherwise instantiate. A lemma's premise is matched in the normal form
  the context has (the normalization's discharger normalizes a premise it
  finds no hypothesis for), so a lemma states a role's atom whichever form
  the context carries.
- **Spellings that hide an equality.** Address equality as `NTy.eqb
  .address` (`acl`; the rule `NTy.eqb_address` is a `simp` lemma outside
  the normal form's set) and a vector read as `Option.getD` of a lookup the
  context resolves (`nested_loop_inv`'s `Option.getD_some`) are spelled
  `==` and `v[i]` in the normal form, where `decide` and `congr` see them.
- **One spelling of a quotient.** Preparation rewrites `Int.tdiv` to
  Euclidean division where a hypothesis states the dividend nonnegative
  (`normalizeContext`, `tdiv_eq_ediv_of_nonneg`), and leaves the other
  occurrences, so one quotient reaches the proof in two spellings: a
  product's sign is no hypothesis (`pool_u64::balance`'s `Int.mul_nonneg`
  then the rewrite by hand), nor is a fact the proof states after
  preparation (`triangular_div`). The normal form spells each quotient
  once per goal: it discharges the sign from the facts the leaf facts
  derive (`nonnegative?` already derives a product's sign) and from the
  proof's facts, which the point steps place before preparation (L2), and
  never leaves one quotient in both forms. The arithmetic lemmas are stated
  over the specification's `/`, so a proof never sees the difference.
- **Predicates of intrinsic operations.** `SpecMap.AbortsNewFrom keys
  values` is `keys.length ≠ values.length ∨ ¬keys.Nodup`, which
  `aborts_if_new_from_2` unfolds by hand and maps to the specification's
  indexed form through `duplicate_indices_map`. The normal form states it
  in the specification's form, two indices with equal keys or unequal
  lengths, so that `decide` (its witnesses, a quantified fact's instance)
  and `congr` close what `omega` and `grind only` close today.

## Budgets and automation

Unchanged from [`proof-strategy.md`](../designs/proof-strategy.md), P3:

- The item owns the target's goal under the target budget
  (`verex.verifyHeartbeats`); no automatic attempt precedes it.
- A closer is bounded as its tactic is today: `decide` by the cheap
  deciders' own bounds, `arith` by `verex_omega_unsplit`'s fact selection
  (no residual allowance), `eval` by evaluation. None retries another.
- `solve` is the only step that spends the automatic leaf cap
  (`verex.leafHeartbeats`), and it is charged as automatic work; inside it
  the prepared leaf and each case of its variant split are attempts of
  their own within that cap, as in automatic verification.
- An obligation no `on` names gets `decide` alone; the template never
  generates `solve`.
- Facts at points are proved under the target budget as authored preparation is
  today (an unowned scope; [`performance.md`](../designs/performance.md),
  "Budget accounting").
- The cost gate of every milestone is per item: the heartbeats of a target
  under its lowered proof, read from the native target total
  (`verex.stageLog`), at or below the target's heartbeats under the script
  it replaces. An item above its script is a defect of the lowering, found
  by the stage log's preparation and closer lines, never accepted as the
  price of the language.

## Messages and templates

Every message names an obligation by its key and states goals and facts in
spec form. Spec form is the clause translator run backwards: delaborators
(`app_unexpander`s) for the native vocabulary print `SpecMap.hasKey m k =
true` as `spec_contains_key(m, k)`, `SpecMap.keyAt m i` as `spec_key_at(m,
i)`, `x.val` as `x`, a specialized contract `g.at_θ.spec` under the callee's
name, the obligation markers not at all. They serve the InfoView as well as
the driver's messages, so the goal under the cursor in `foo.proof.lean`
reads as the proof is written. The delaborators (`VerexSL/Verify/SpecForm.lean`)
print where the option `verex.specForm` is set, the role functions taken
from `verex.specRoles`, which the verifier states from the module's maps;
the proof language's messages set both, so a successful check and the
messages of scripts print as before.

A failed automatic verification prints the proof it asks for in the
language, replacing today's template (`proofRequest`,
`VerexSL/Verify/Contracts.lean`: `verex_obligations`, `verex_on` with
`| leaf => verex_prepare; sorry` per open leaf and `| _ => verex_decide`):

```text
ordered_map.move:412:9: error: verex verification failed: `test_verify_drain_symbolic`
does not establish `invariant 2 of loop 1 step`; provide a proof in
`ordered_map.proof.lean`:
  verify test_verify_drain_symbolic by
    on invariant 2 of loop 1 step {
      -- owes: forall i : Int where 0 <= i && i < len(m'):
      --         spec_key_at(m', i) == spec_key_at(old_m, i + 1)
      -- from:  m' == spec_remove(m, spec_key_at(m, 0)); 0 < len(m);
      --        invariant 2: forall i … ; invariant 1: len(m) == len(old_m) - count
      assert … by decide
    }
```

The `owes` and `from` lines are the prepared obligation: its goal and the
facts the cheap decider would read, in spec form, the clause facts named by
their clause. Where an obligation fails on some of its paths and not others,
the template prints an `on` block with a selector for each failing path. A
failing step reports the same way: the step, the fact it owed, the closer,
and the facts the closer had. A `split` that leaves a case unproved names
the case (`it is End`).

## Suitability for generated proofs

An agent writing proofs today reads a Lean goal in the solver's normal form,
chooses among 162 tactics, and learns by failure which `verex_prepare`
variant produces the shape a lemma matches. The language limits what it must
know to the Move file and the message:

- the vocabulary is the specification's, which the agent has already read;
- the obligation key tells it which clause at which site, so it can read the
  Move code there;
- the facts it needs are the lemmas by their names
  (a finite list the message can offer on request);
- each step fails individually with the fact it owed, so repair is local;
- there is one way to say each thing, so the proofs it has seen are the
  proofs it should write;
- a proof it writes is cheap to check by construction: every closer is one
  bounded attempt and the lowering emits no cascade, so a generated proof
  cannot spend heartbeats on alternatives the way a generated script does,
  and only `solve` can spend the automatic cap.

`lean =>` is permitted to generated proofs as to any other. A generated proof
that needs it signals that the language or the lemma set is missing something,
to be fixed there.

## Host

Two hosts exist for authored steps: the Move proof block (`spec f { … }
proof { … }`, lowered through the exchange into in-body specification
statements, [`lemmas.md`](../designs/lemmas.md)) and the proof file. The
language is one step grammar with two placements; the hosts differ in which
placements they can carry.

- The Move proof block carries point-placed steps only: the function's
  entry (its statements), `post` (exit), and `if`-guarded steps; `assert`,
  `assume`, `apply`, `split`, `calc`, `let`. It cannot carry `on`, closers or
  `lean =>` without changing the Move grammar and putting Lean in Move
  sources, and it is parsed in Rust, so every step would be implemented
  twice.
- The proof file carries both placements, closers, `lean =>`, and the
  theorems a `lean =>` step needs. Its steps are parsed by VerexSL, which
  already parses the lemma proof steps (`assert`, `assume`, `apply`, `split`
  in `spec lemma … proof`, and `spec assert`/`spec apply`/`spec split` in
  bodies), so the step grammar is shared with the Move block's lowering.

Recommendation: start in the proof file, as proposed; keep the Move proof
block as it is, and make its lowering and the proof file's `at entry`/`at
exit`/`if` steps one implementation, so a fact placed in either host means
the same: an `assert` at a point is an obligation in both hosts, proved by
the same means, and given after the point. The Move block's `assume` keeps its own lowering as an in-body
`spec assume`; it is Move's statement, and the proof file's point blocks
cannot assume. A user who wants a fact to travel with the source writes it
in the Move block; a user who needs an obligation-level argument, a closer or
Lean writes the proof file. Extending the Move block with `after call` and `at
loop` placements is a later, separate decision; `on` and `lean =>` stay out
of Move.

Precedents weighed: Verus (`assert(e) by (arith)`, `assert forall |i| … by
{…}`) gives the closer-on-assert and the quantified block; Dafny (`forall …
ensures … { }`, `calc`, `assert … by {}`) the `forall` statement and
`calc`; Isar (`have … by …`, `obtain`, `case`) the named facts and
`witness`; F* (`assert e by tac`) the escape; the Move Prover the step names
and the point placement. None addresses obligations, because none presents
them; `on` is the one construct without precedent, and it exists because
Verex presents obligations rather than one VC.

## Coverage of the corpus

Without the lemma items, the arithmetic and vector lemmas, `ring`, `congr`,
the structural steps beyond the `split` of a local, and the normal-form
fixes, about 30 of the roughly 75 items of the corpus, the paper examples
and the e2e proof fixtures need `lean =>`: most map-module items fit, about
a third of the others. What those items do in Lean, and what replaces it:

| Group | Items | Today's tactics | Replaced by |
|---|---|---|---|
| Nonlinear and division arithmetic | `math_fixed64`, `nonlinear_arithm`, `for_loop_invariants`, `pool_u64::balance`, `closures/amm_example`, the paper's `amm_example`, e2e `square_of_sum` | `Int.mul_le_mul`, `mul_le_mul_of_nonneg_left`, `mul_lt_mul_of_pos_left/right`, `mul_nonneg`, `ediv_nonneg`, `ediv_le_of_le_mul`, `ediv_lt_of_lt_mul`, `le_ediv_of_mul_le`, `ediv_mul_le`, `ediv_le_ediv`, `add_mul_ediv_right`, `emod_eq_of_lt`, `shiftLeft_eq`, `tdiv_eq_ediv_of_nonneg`; `add_mul`/`mul_add` rewriting; `grind` over products | arithmetic lemmas, `ring`, one spelling of a quotient |
| Lean theorems in the proof file | `sqrt_root_range`, `newton_quotient`, `triangular_nonneg/step/bounds`, `effective_bound`, `market_bounds`, `ratio_monotone`, `fee_monotone`, `erase_unique`, `swap_erase_last`; the rules `address_transport`, `parameter_transport`, `triangular_div` | `theorem … := by …`, applied by name; `@[vir_denote_norm]` | lemma items; the rules go to the normal form |
| Equality and congruence | `capability`, `pool_u64::buy_in`, part of `acl`, `nested_loop_inv`, `aborts_if_new_from_2` | `simp_all; grind` (`capability`), `grind -ring only` (`buy_in`), `grind only [NTy.eqb_address]`, `grind only [Option.getD_some]`, `grind only` | `congr`; the normal-form fixes for the spellings |
| Case analysis beyond the `split` of a local | `next_key_scan`, `leaf_walk_sum`, `leaf_iter`, `vector_hofs_fold`, `drain_loop`, `simple_vector_client::test_contains`, `lower_bound_scan_start` | `generalize …; rcases`, `variantCarrier.second_of_name`, `rcases (show i = 0 ∨ … by omega)`, `Classical.byContradiction` | `split` of an expression or a range, `let V { … }`, `by contradiction` |
| Sequences | `verify_vector`, `acl::remove`, `iter_collect` | `SpecVector.ext`, `eraseBounded_values`, `Array.getElem?_eraseIdx*`, `Array.getElem?_push` in the decider's simp set | vector lemmas |
| Solver gaps showing through | `capability`, `acl`, `nested_loop_inv`, `keys_for_loop`, `lower_bound_scan_start`, `next_key_scan`, `leaf_walk_sum`, `aborts_if_new_from_2`, `for_loop_invariants`, `pool_u64::balance` | `@[vir_denote_norm]` rules in proof files, `SpecInt.val_ofInt`, `IntegerValueFits_unsigned_succ`, `.fits`, `keyAt_satisfies(_all)`, `hasKey_satisfies`, `NTy.eqb_address`, `Option.getD_some`, `SpecMap.AbortsNewFrom`, `tdiv_eq_ediv_of_nonneg` | normal-form fixes |
| Function values and memory | `calculator`, `stored_fun_values`, the paper's `amm_example`, `specialize_generic_caller` | `NTy.encode_function`, `Weave`, `Which`, `MemoryInvariants`, `Skolems.instantiate`, `lemmaRequires` | the function-value lemmas and the frame lemma ([`native-function-values.md`](native-function-values.md), steps 2b and 3), generic `apply` (L1) |

`ground_enum_123` is not in the table: its script applies rank lemmas
(`rank_nonneg`, `rank_lt_size`, `rank_lt_rank`, `keyAt_rank`) and closes by
`omega`, which `apply` and `decide` cover.

With all of these and the function-value steps, no corpus item is left
that needs `lean =>`. Each item's rewrite confirms or refutes that; a
residual escape is a gap recorded in its milestone's gate. `capability` is
the least certain: whether its obligations, after the splits and witnesses,
are within `congr` is known only from its goals.

## Migration

- When the language lands (L1), every existing item's tactic script moves
  under `lean =>` (`verify f by lean => …`), a mechanical rewrite of the
  corpus that changes no proof. The driver, the editor header, `pragma verify
  = manual` and the templates are unchanged until L3.
- Each proof file is rewritten item by item, the map modules first. A
  rewritten item must contain no `verex_*` name, no solver vocabulary and no
  copied preparation sequence; an item that cannot be rewritten without
  `lean =>` is a gap in the language or the lemma set, recorded in the
  milestone's gate.
- The two turned-off proofs ([`roadmap.md`](roadmap.md): the paper example
  `amm`, stated over runtime values, and `pool_u64::deduct_shares` under
  `pragma verex_verify = false`) are written in the language rather than
  ported as scripts. The proofs over closures (`amm`, `calculator`,
  `stored_fun_values`) are written once, after closures are data (steps 2b
  and 3 of [`native-function-values.md`](native-function-values.md)); a
  script over `FV_τ` or over the carrier interface is not written for
  them in between.
- The Prover's `.lean_exp` baselines and the registry change only in the
  hint text (as the last two template changes did).
- `verex_proof`, `verex_obligations preparing`, `verex_group`,
  `verex_group_contexts` and the `verex_prepare` variants stay until the
  corpus uses none, then become internal.

## Milestones

Cost gates are per item against the script each item replaces, and per
module against the module's native total, both measured at the milestone's
start; the figures of [`performance.md`](../designs/performance.md) predate
native leaves and are to be re-measured
([`measurement-debts.md`](../status/measurement-debts.md)).

| | Scope | Gate |
|---|---|---|
| L0 | Obligation keys: site indices and the clause in `Provenance`, `leafName` rendering, `verex_on` accepting a key (done for calls); the sites of `if`, `match`, `abort`, aborting operations, loops and invocations in the denoted program; path decisions recorded per leaf at the split that makes them and selectors in `verex_on`; messages name keys; path suffixes replaced by selectors | the corpus verifies with its tags renamed, at unchanged heartbeats (recording a decision is a field on the leaf, no walk); `requires 1 of iter_next at call 2` routes one call's precondition and not the other's (fixture in `VerexSL.Tests.SharedPreparation` or a new `Specifications/Addressing`); `pool_u64::buy_in`'s two abort paths routed by selectors |
| L1 | The item and the steps `let`, `assert`, `assume`, `apply`, `split` (Booleans and variants), `calc`, `forall`, `witness`, `if`, `on`, `when`, `lean =>`; the closers; expression elaboration in obligation scope at the leaf's instance spelling, function-typed expressions against the carrier interface of the function-value plan's step 2a; generic lemma instantiation; delaborators for spec form; the map lemma table extended to the 22 role lemmas the corpus applies; the corpus moved under `lean =>` | the 9 `ordered_map` items rewritten with no `verex_*`, no solver vocabulary, no `lean =>`; every item at or below its script's heartbeats; the module's native total at or below its reference |
| N1 | Normal form, before L2: type bounds of values read from typed containers; the specification form of `AbortsNewFrom` | every automatic target unchanged in outcome, at or below its heartbeats; `keys_for_loop`, `lower_bound_scan_start`, `next_key_scan`, `leaf_walk_sum`, `aborts_if_new_from_2` need no `satisfies` lemma, `val_ofInt`, `.fits` or unfolding |
| L2 | Points: the `preparing` hook keyed by provenance (call and operation continuations, loop entry and step, exit); one lowering for Move proof blocks and proof-file point steps; the structural steps (`split` of an expression or a range, `let V { … }`, `by contradiction`) and `congr` | an `after op` fact shared by the paths after the operation (fixture); the 32 `big_ordered_map` items rewritten, `next_key_scan`, `lower_bound_scan_start`, `leaf_walk_sum`, `leaf_iter`, `drain_loop`, `keys_for_loop` and `aborts_if_new_from_2` among them; the five copies of the iterator block and the seven pipelines gone; every item at or below its script's heartbeats, the items that prepared twice below; the module's native total at or below its reference |
| L3 | Templates and messages: `on` blocks with `owes`/`from` in spec form; step failures in spec form; InfoView shows spec form | an agent given the Move file and the message, and nothing else, writes accepted proofs for the Prover registry's authored targets (`prover-test-problems.md`); each accepted proof at or below the script it replaces, and a target whose automatic verification succeeds unchanged in heartbeats (delaborators and templates run on failure only) |
| N2 | Normal form, after L2, before L4: the unapplied transports, the spellings that hide an equality, one spelling of a quotient | every automatic target unchanged in outcome, at or below its heartbeats; no proof file registers a normal-form rule |
| L4 | Lemma items, the arithmetic and vector lemmas, `ring`; `split` on function values, the `invoke` points, keys and decisions, the function-value lemmas and the frame lemma; corpus: the remaining 16 files and the two turned-off proofs rewritten | zero `verex_*` names and zero solver vocabulary across the corpus, and no `lean =>` except gaps recorded in this gate; each `ring` and `congr` at or below the rewriting or `grind` it replaces; the benchmark's 31 problems unchanged in outcome, every authored item at or below its script, the module totals at or below their references; the closure items (`calculator`, `amm`, `stored_fun_values`) written once, against the function-value plan's steps 2b and 3 |
| L5 | Invisible incremental preparation: the point hooks prepare where facts are introduced, once the rule set commutes with substitution | no proof file changes; `upsert` preparation toward the bound of `performance.md`, "Open levers", 1 |

`assume` in a proof file is not in L1 (item 3 of "Semantics and lowering"
has its design). L1's gate holds for eight of the nine `ordered_map` items,
each at or below the script it replaced (elaborator heartbeats, language
against script): `drain_symbolic` 27.29M against 28.50M,
`iter_walk_mut_symbolic` 78.35M against 81.02M,
`iter_remove_shift_at_position` 19.76M against 20.08M,
`iter_borrow_mut_at_position` 12.55M against 13.40M,
`iter_borrow_mut_symbolic` 25.41M against 26.74M,
`iter_add_middle_symbolic` 21.402M against 21.405M,
`iter_add_append_symbolic` 25.99M against 27.11M, `ground_enum_123` 9.02M
against 9.23M; together 219.8M against 227.5M.
`aborts_if_new_from_2` owed `AbortsNewFrom`, which had no specification
form before N1, and is `on _ { }` since. Its map lemma table holds the
eighteen lemmas of the corpus that the roles state ("Lemmas").

N1's gate holds for its five items, which name none of the lemmas
(elaborator heartbeats, against the script before N1): `keys_for_loop`
27.94M against 28.30M, `lower_bound_scan_start` 29.70M against 29.82M,
`leaf_walk_sum` 392.24M against 387.53M, `next_key_scan` 109.98M against
103.55M, and `aborts_if_new_from_2` as `on _ { }` 10.11M against 12.80M
(`ordered_map`'s 6.24M against 7.02M). The two above their scripts carry
the bounds as facts through every later step; `next_key_scan` adds them
once more where its own steps expose the successors. The automatic targets
keep their outcomes (the Prover's lean gate, the framework modules), and
`DenotePerformance` measured against the commit before N1 differs by at
most 1.1%, a difference whose sign follows the targets' order in the file.

L2 so far, twenty-three `big_ordered_map` items as step proofs (elaborator
heartbeats, language against script): `next_key_scan` 67.16M against
111.87M, `upsert` 198.80M against 241.73M, `insert_shift` 12.20M against
15.26M, `iter_valid_after_insert` 31.80M against 49.66M,
`aborts_if_new_from_1` 8.78M against 8.89M, `pop_back_drain` 33.86M against
35.73M, `drain_loop` 71.84M against 84.57M (three L4 exceptions),
`next_key` 71.83M against 78.66M, `prev_key` 67.36M against 75.71M,
`remove_shift` 13.09M against 14.32M, `drain_symbolic` 34.96M against
36.82M, `front_remove_drain` 41.29M against 41.77M, `aborts_if_new_from_2`
10.73M; above their scripts, to be fixed as cost work, `aborts_if_remove`
9.73M against 9.48M and `keys_sorted` 56.74M against 56.15M (one L4
exception); `iter_remove` 42.80M against 51.50M and `iter_remove_shift`
44.79M against 49.72M since a generic callee's results are read in the
caller's spelling; `iter_prev`, `iter_next`, `iter_rank_loop`,
`iter_collect`, `iter_across_upsert` and `leaf_iter` verify, several above
their scripts and some with `lean =>` steps, kept as the benchmark of the
tuning after the language is complete. The others wait on: a premise the block's normal form
decides from a literal map (`iter_across_upsert`); the walk's cost before
any block (`back_key_rank`, `ground_enum_123`, `iter_prev_loop`,
`keys_for_loop` and the three walks); the iterator items' validity
disjunctions; L4 features (`leaf_iter`, `leaf_walk_sum`).

Ordering against [`native-function-values.md`](native-function-values.md),
decided: that plan's step 2a (the function carrier interface) comes first,
because the elaborator of L1 targets the interface and the enforcement goal
holds exactly from then on; L0–L2 then run interleaved with its steps 2b
and 3, since the map modules hold no function value and the two plans
share only the invocation provenance; the function-value constructs of the
language (`split` on a function value, `invoke` keys and decisions, the
function-value lemmas, the closure items of the corpus) come after 2b and 3,
in L4. L2 needs L0's provenance; L5 waits on the cost work and is listed to
fix the interface it must respect. N1 precedes L2, whose gate holds the
items it serves. L4's lemma items, built-in lemmas and `ring` serve no function value
and may start after L2, alongside steps 2b and 3, with N2 before L4. A
change of the normal form is its own step, gated on the automatic targets,
never part of an L step.
