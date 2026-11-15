# The proof language

The structured language authored proofs are written in: its model, its
surface, what each construct costs, and how it lowers onto the solver's
leaves. The user chapter [`../user/proof-language.md`](../user/proof-language.md)
is the reference for writing proofs, with worked examples; the open items of
the language are in [`../plan/proof-language.md`](../plan/proof-language.md).
It keeps the thesis of [`proof-strategy.md`](proof-strategy.md) — cheap,
bounded automation, authored proofs on hard obligations, no SMT — and
decides only how an authored proof is written and lowered.

The language is hosted in VerexSL: the items and steps are parsed as VerexSL
syntax (`VerexSL/Verify/Steps.lean`), a step's expressions are specification
expressions, and `verify f by` is the item a proof file states beside the
Move source ([`source-verification.md`](source-verification.md), "Proofs").

## Why a proof language

An authored proof exists for speed. A hard leaf gets one because checking a
stated argument costs a fraction of the automatic attempt on it, which
spends its cap on preparation and search before it fails or succeeds
([`proof-strategy.md`](proof-strategy.md), "Thesis"). A language over those
proofs is worth having only if it keeps that: a proof written in it checks
in no more heartbeats than the hand-written tactic script for the same
target, and is cheaper to write and to keep working.

The language is the **stability boundary** between proofs and the solver.
Three properties of hand-written scripts decide its design.

- **Preparation cost is a system problem the language does not fix.** A fact
  is prepared once per goal that holds it, because the normal form does not
  commute with substitution. The remedy is a rule set that commutes and a
  context prepared once per path ([`performance.md`](performance.md), "Open
  levers"), with or without a language. The language neither adds nor
  removes that cost, and claims nothing about it.
- **A script addresses goals by position and rewrites them by Lean lemma
  names, so the solver's normal form is its interface.** Every cost lever
  that changes the normal form therefore breaks scripts. Under the language
  no proof text mentions a normal form, a prepared shape or a solver tactic,
  so the normal form can change without touching the corpus.
- **Dispatch and the solver vocabulary are language problems, and dispatch
  is paid for.** Where nothing in the surface says what an obligation is, an
  author writes a cascade of alternatives and lets each goal find its
  closer; every alternative that fails before the one that closes is checked
  and charged. The language removes the cascade by construction: an
  obligation is named by what it is, a path by the source decisions on it,
  and each closer is one bounded attempt.

## Goals and non-goals

Goals, in order:

- **Efficient proofs.** A proof is cheap to check: no trial dispatch, no
  preparation of a goal already prepared in its scope, no contract
  normalized again at an `apply`, no elaboration that searches. Cheap to
  write: the proof names the argument and nothing else, in the vocabulary
  the author already knows, and a failure message prints the frame to fill.
  Cheap to maintain: no proof text mentions a normal form. A construct that
  buys convenience at checking cost is left out, or lowered so that it costs
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
  authored proof is. A function-typed expression elaborates to the function
  carrier's interface
  ([`higher-order-functions.md`](higher-order-functions.md), "Function
  values as native data"), never to the runtime closure vocabulary.
- An obligation is addressed by what it is — the clause, its owner site, the
  call or loop that raises it — never by a traversal number, and a path to
  it by the source decisions on that path, so that a proof can guide every
  path individually.
- Preparation is invisible. The proof states facts and names case splits;
  the system decides when and how to prepare, and may change it.
- Automation is unchanged: an obligation the proof addresses gets the cheap
  deciders with the proof's facts; one it does not address gets the cheap
  deciders alone; the automatic leaf runs only where the proof asks for it,
  under its cap.
- A failed automatic verification prints the proof it asks for in this
  language, with each open obligation stated in the same vocabulary, so that
  a person or an agent can write the proof from the Move file and the
  message alone.
- The Move Prover's proof blocks (`assert`, `assume`, `apply`, `split`,
  `calc`, `let`, `if`, `post`; [`lemmas.md`](lemmas.md)) are the precedent
  for every step that has one.

Non-goals:

- A general tactic language, or a replacement for Lean where an argument is
  genuinely about Lean terms: those stay Lean theorems and `lean =>` steps.
- New deciders, larger budgets, or search inside the closers. The two
  closers that wrap a Lean decision procedure, `ring` and `congr`, run one
  module of `grind` each with splitting and instantiation off.
- Counterexamples, or proof search for the author.
- A change to the Move language. The Move proof block keeps its grammar;
  the obligation-addressed form lives in the proof file (see "Host").

## Surface

A proof file keeps its header and its items, and an item keeps its keyword:
`verify f by` followed by a block of the language. Lean tactics are reached
through the `lean =>` step only, so one grammar parses every item and the
language's step names (`apply`, `split`, `calc`, `let`, `if`) never compete
with Lean's tactics of the same name. A whole-target Lean proof is
`verify f by lean => …`. A proof file may also state and prove lemmas in the
specification vocabulary ("Lemma items").

```text
item      ::= 'verify' f 'by' block
            | 'lemma' L ('<' T (',' T)* '>')? '(' params ')'
                ('requires' e)* ('ensures' e)+ ('decreases' e)? 'by' block
block     ::= step*
step      ::= 'let' x ':=' e
            | 'let' x ':' T 'where' e
            | 'let' Variant '{' x (',' x)* '}' ':=' e  -- destructure a known variant
            | 'assert' e ('by' closer)?
            | 'assume' e
            | 'apply' φ (',' φ)*                       -- lemma instances
            | 'compute'
            | 'finish' 'by' closer
            | 'split' e ('in' e '..' e)?               -- Boolean, enum/option; integer range
            | 'calc' '(' e (op e)+ ')' ('by' closer)?
            | 'forall' binders ('where' e)? '{' block '}'
            | 'witness' e (',' e)*
            | 'if' e '{' block '}' ('else' '{' block '}')?
            | point selector? '{' block '}'
            | 'on' obligation (',' obligation)* selector? '{' block '}'
            | 'on' '_' selector? '{' block '}'
            | 'lean' '=>' tacticSeq
closer    ::= 'decide' | 'arith' | 'eval' | 'ring' | 'congr' | 'solve'
            | 'apply' φ (',' φ)* | 'witness' e (',' e)*
            | 'contradiction' '{' block '}' | 'lean' '=>' tacticSeq
point     ::= 'at' ('entry' | 'exit' | 'loop' k ('entry' | 'step'))
            | 'after' site
site      ::= 'call' k ('of' g)? | 'abort' k | 'op' k | 'invoke' k
obligation::= 'ensures' k | 'aborts_if' k | 'aborts_with' k
            | 'requires' k? 'of' g ('at' 'call' j)?
            | 'invariant' k ('of' 'loop' j)? ('entry' | 'step')
            | 'assert' k ('of' point)? | 'data' 'invariant' k? ('of' S)?
            | 'module' 'invariant' | 'stored' 'invariant' k?
            | 'lemma' 'requires' | 'ghost' 'domain' | 'after' site
            | '_'
selector  ::= 'when' decision (',' decision)*
decision  ::= 'if' k ('then' | 'else')
            | 'match' k Variant
            | 'invoke' k g
            | 'loop' k ('exits' | 'continues')
            | 'aborts' 'at' site
            | 'returns'
```

The step syntax is declared in `VerexSL/Verify/Steps.lean` (`stepOn`,
`stepAt`, `stepAfter`, `stepLet`, `stepAssert`, …), the keys, sites, points
and decisions in `Denote/Solve.lean` (`verexKey`, `verexSite`, `verexPoint`,
`verexDecision`).

A site names a construct of the body: `call k` the `k`-th call of a named
function, `invoke k` the `k`-th invocation of a function value (`f(x)`,
`invoke(f, args)`;
[`higher-order-functions.md`](higher-order-functions.md)), `abort k` the
`k`-th `abort` or `assert!`, `op k` the `k`-th operation the frontend marks
as aborting. `at` a site means the construct itself: `requires k of g at
call j` is the precondition the call owes, `aborts at call j` a path
aborting in it. `after` a site means its continuation, the paths on which it
did not abort; an abort has no continuation, so `after abort k` names no
point and is reported as an error. Facts after a site name only the
variables in scope there: the result of an operation or of a nested call is
anonymous unless the Move code binds it.

`e` and `φ` are VerexSL specification expressions (`VerexSL/Syntax.lean`:
`x + 0 == x`, `len(v)`, `v[i]`, `spec_contains_key(m, k)`, `old(m)`,
`result`, `x is Position`, `x.index`, `∀ (i : Int), … ==> …`), the Lean-side
projection of Move's spec expression syntax that lemmas and in-body `spec
assert` use. The proof file accepts this dialect only, not Move's literal
spec syntax (`forall i: u64: …`): one dialect keeps one meaning per
expression. `φ` is the lemma formula of [`lemmas.md`](lemmas.md): `L(a)`,
`c ==> φ`, `∀ xs. ψ`.

### Steps

Each step holds at the point or obligation whose block holds it.

- `let x := e` binds `e` for the following steps (substituted, as the Prover
  substitutes).
- `let x : T where φ` (Move's `choose`) owes `exists x : T: φ` by `decide`
  and gives `φ` with the witness `x` in scope. Where `φ` defines `x` by an
  equation with an integer term, `x` is the scope name of that term
  ("Normal-form fixes", "One spelling of a defined value").
- `let V { f, … } := e` destructures a value the context shows to be of
  variant `V`: it owes `e is V` by `decide`, brings the listed payload
  fields into scope under their Move names, and gives `e == V { f, … }`.
  Unlike `split`, it creates no cases.
- `assert e by c` owes `e`, proved by the closer `c` from the facts in scope
  (the obligation's or point's context and the facts stated before it), then
  gives `e` to the following steps and to the closing of the obligation.
  Without `by`, the closer is `decide`.
- `assert e by contradiction { block }` runs the block under `!e` and closes
  when `decide` derives `false` from its facts.
- `assert (∃ x, φ) by witness w, …` owes the existential at the witnesses,
  each at its own type and coerced to the binder's, its body by `decide`,
  then gives the fact.
- `apply φ, …` is the Prover's step: `apply L(a)` owes `L`'s premise at `a`
  (by `decide`, or `assert` the premise first) and gives its conclusion;
  under `c ==>` both are guarded; `apply ∀ xs. ψ` gives the implication
  under the binders. `L` is a `spec lemma` of the module or of a module it
  uses, a recursive specification function (its unfolding is a lemma of it),
  or one of the built-in lemmas (below). A generic `L` is instantiated from
  the types of `a`; the specialized name the solver states it under never
  appears in the proof.
- `compute` computes vector lengths and reads after updates where
  established bounds identify the affected position, and leaves unresolved
  reads unchanged.
- `finish by c` closes every remaining obligation of the block with a core
  closer (`decide`, `arith`, `eval`, `solve`, `congr`, `ring`) and reports
  an error if one remains open. It emits no fact, and is not a step of a
  program-point block, which asserts instead.
- `split e` continues the block once per case: `true`/`false` of a Boolean,
  one case per variant of an enum or option, the variant's fields in scope
  under their Move names (`it.index`, not a positional payload name). `e` is
  any specification expression of these types, not only a local: the
  lowering generalizes it and keeps `e == V(x, …)` as a fact. `split i in
  a..b` splits an integer the context bounds by literals into one case per
  value, `i` substituted in each; the bounds are owed by `decide`, and the
  case count is capped as `pragma unroll` caps iterations. The steps after a
  `split` run in every case.
- `if e { … } else { … }` guards steps by a condition without splitting the
  obligation's proof.
- `calc(e₁ op e₂ op …)` asserts each link by the closer and gives the
  chain's conclusion.
- `forall x : T where g { block }` introduces `x` under `g` for the block
  and gives `∀ x : T, g ==> (every fact the block asserted)` after it. This
  is Dafny's `forall` statement, and generalizes the Prover's `forall xs
  apply`.
- `witness e, …` supplies the witnesses of an existential obligation, a step
  of the `on` block that names it. The witnesses are spec expressions; their
  carrier certificates are the elaborator's.
- `point { block }` places facts at a program point: `at entry`, `at exit`
  (the Prover's `post`), `at loop k entry|step`, or `after s`, the
  continuation of a site `s`. The block's facts are owed once there and hold
  for every obligation after the point, on every path through it.
- `on obligation, … { block }` proves the named obligations: its facts hold
  in the obligation's context, and the obligation closes by `decide` over
  them. `on _ { … }` takes every obligation no earlier `on` names. An
  obligation named by no `on` gets `decide` alone.
- `when d, …` after an `on` key or a point selects paths: the block applies
  to the paths consistent with every listed decision. A path goes to the
  first `on` in text order whose key and selector match it, so a block for
  selected paths precedes a general block for the same clause, and the
  general block takes the remaining paths. A proof can thus guide each path
  separately, down to a single path, without naming a traversal number.
- `lean => tac` runs a Lean tactic block on every goal the block has at that
  step: after a `split`, on every case, as every step does. It may leave
  goals open, which `decide` then closes. It is the escape for what the
  steps cannot say. Inside it the goal has the solver's prepared form, and
  the block is checked for runtime names as every authored proof is. It is
  permitted in every proof file, the framework's included.
- `assume e` is parsed and rejected: an assumption of one obligation has no
  place in the target's theorem, whose hypotheses are stated before the
  walk. The in-body assumption is Move's `spec assume` in the proof block
  ([`lemmas.md`](lemmas.md), "Trust"). The design of a proof-file `assume`
  is an open item ([`../plan/proof-language.md`](../plan/proof-language.md)).

### Closers

| Closer | The tactic it is | What it decides |
|---|---|---|
| `decide` | `verex_block_decide` on the block's normal form | the default; bounded; no search, no budget |
| `arith` | `verex_omega_unsplit` alone | linear arithmetic with the stated bounds, no case splits and no residual allowance over the whole context |
| `eval` | `verex_map_simp`, `verex_typed_close`, `verex_goal_witnesses`, `decide` | closed maps, literal vectors, ground arithmetic |
| `ring` | `grobner`, the `ring` module of `grind` alone, with the facts in scope hidden | a polynomial identity `e₁ == e₂` over the integers; an equation it depends on is substituted first (a `let`, a `calc` link) |
| `congr` | bounded rewriting by local equalities and shallow congruence, then congruence closure with linear arithmetic over the facts in scope and no case split (`closerCongr`) | equality through function applications and uninterpreted terms combined with linear arithmetic, `f(a * b) == f(2 * b)` from `a == 2`; no open `if`, no existential |
| `solve` | `verex_solve` (`solveLeaf`) | the automatic leaf under its cap: an explicit request for automation, charged as automatic work |
| `apply φ, …` | the lowering of `apply`, then `decide` | the named lemmas at the stated arguments |
| `witness e, …` | the obligation's or assert's existential at the witnesses | an existential fact |
| `contradiction { … }` | `by_contra`, the block, then `decide` on `False` | a fact whose negation contradicts the facts in scope |
| `lean => tac` | the tactic | anything |

`decide` is one bounded check per alternative, not a cascade: a goal stated
by a hypothesis (up to the spelling of instances, an equation either way
round, a disjunct), a context that contradicts itself, an instance of a
quantified hypothesis (its premises by `omega`), reflexivity, or linear
arithmetic. Before those, it reads the conclusions of the implications and
equivalences whose premises are stated, and the remaining disjunct of a
disjunction whose other disjunct is refuted (`impliedFacts`). An instance's
binders are determined by the goal and the stated facts: a premise that is a
read or an equation (`xs[i]? = some x`, `m k = some v`, `f a = b`) is
matched against a stated hypothesis of that shape, which fixes the binders
it holds, and the remaining premises go by arithmetic; an integer binder in
a position is solved from a literal offset, and the instantiated positions
are normalized before the rest is matched. A literal bound is matched up to
its form: `a < k` is stated by `a ≤ k − 1` and conversely — matching, not
rewriting. Hypotheses and goals are read up to negation (`¬∃ x, φ` as
`∀ x, ¬φ`, `¬(A ∨ B)` as its parts, `¬(A ∧ B)` as `A → ¬B`, `¬¬A` as `A`),
recursively through premises; the reading is the instance decider's and
`statedProof?`'s, and no normal-form rule rewrites a negation
(`Solve/Decide.lean`, `VerexSL/Verify/Steps.lean`). A failing closer reports
at its step the goal it did not close, in spec form. No closer tries
another.

`ring` takes the rewriting chains a script writes for an expansion and the
`grind` calls whose work is expanding products before a linear step: the
proof states the expanded identity by `ring`, and `arith` closes over the
products as atoms. `congr` takes the remaining congruence arguments. Without
splits `congr` decides no `if` whose condition is open, even with the same
`if` on both sides, and proves no existential, so the proof names the case
and the witness.

### Lemma items

A `lemma` item is a Move `spec lemma` ([`lemmas.md`](lemmas.md)) written
beside the source instead of in it: the same `LemmaDecl`, the same
verification (assume `requires`, run the block, owe `ensures`), the same
recursion rule, so an `apply` of a lemma of its own recursion group is
induction under `decreases`. `apply` reaches the file's lemmas as it reaches
the module's. An item is lowered as a `spec lemma` of the module's copy that
lowers the step expressions, and its theorem is proved by its block
(`verex_lemma_steps`) before the `verify` items that apply it. The item's
premise is taken apart, each parameter entered under its name, the bounds of
the values in scope stated, the conclusion read in the normal form under the
`requires`, then the block's steps, then `decide` for the conclusion; a
conjunction is given one fact per part.

### Lemmas

The lemmas of the intrinsic maps are a fixed table over the map roles every
owner's `pragma intrinsic = map` binds (`map_spec_get`, `map_spec_set`,
`map_spec_del`, `map_spec_len`, `map_spec_has_key`, `map_spec_key_at`,
`map_spec_rank`; [`intrinsic-maps.md`](intrinsic-maps.md)), each entry a
`SpecMap` theorem (`mapLemmas` of `VerexSL/Verify/Steps.lean`, with
`MapLemma` naming the theorem, the roles it reads and its parameters). Each
lemma is named `lemma_` and the words of the owner's bindings
(`lemmaName`/`roleWord`: `lemma_get_set`, `lemma_len_remove`,
`lemma_contains_key_key_at`), so the names come from the roles, never from a
module name. The user chapter tabulates the table's conclusions.

The language reaches the lemmas by `apply` only. The module's copy that
lowers the step expressions declares each lemma as a `spec lemma` with its
signature alone, which types an application's arguments: the map at the
owner's type, keys and values at its type parameters. At the leaf,
`apply L(a)` instantiates the lemma's theorem at the translated arguments
(`lemmaInstance`): the map as its owner's typed map, a key or a value as the
typed value the map holds, the discipline as the owner gives it. Each
parameter of a lemma names the binder of the theorem it fills
(`LemmaParameter`, `LemmaParameterKind`), and the table is checked against
its theorems when the verifier is built: every parameter fills a binder, and
every other binder is the discipline, the order, a type, an instance or a
premise. The theorem's premises are opaque goals, each normalized and proved
by the step's closer; its statement, normalized, is the fact. No alias
theorem and no generic frame is stated for an application. A module reaches
the lemmas of its own maps first, then those of the maps of the modules it
uses.

The coverage of a map's keys by a vector is a map lemma of the `has_key` and
`len` roles (`lemma_contains_key_len_search_absurd(m, keys, k)` for an owner
binding `spec_contains_key` and `spec_len`; `Denote/MapCoverage.lean`): a
distinct vector of the map's size whose elements are keys of it holds every
key, so a failed search for a key is absurd.

**Arithmetic and vector lemmas.** A second fixed table (`builtinLemmas`,
`BuiltinLemma`, over the theorems of `Denote/StepLemmas.lean`) holds the
integer lemmas over the specification's `*`, `/`, `%`, `<<` and `>>` with
Move's semantics, and, per vector operation (`update`, `push_back`,
`pop_back`, `swap`, `swap_remove`, `remove`, `concat`, the slice, the search
`index_of` makes), a length lemma and an element lemma, plus extensionality.
The specification's `/` and `%` elaborate to `Int.tdiv` and `Int.tmod`
(truncation toward zero, as Move), `<<` and `>>` to the exact
`Int.shiftLeft` and `Int.shiftRight`
([`numeric-representation.md`](numeric-representation.md)). Where truncation
and Euclidean division differ, a lemma requires nonnegative operands and its
Lean proof carries the bridge, which no proof states again. A lemma is
stated in one operand order; the `apply` instance and the obligation
elaborate through the same normal form, so a commuted product never meets
its lemma's spelling. `arith` treats a product as an atom, so a product
bounded by a lemma with the rest linear needs nothing more.

Lemmas of function values are the link set's requires equations and the
behavioral readings of a known target
([`higher-order-functions.md`](higher-order-functions.md)), reached through
the same `apply`; the composition lemmas of the runtime closure vocabulary
are not in the table, and `Denote/FunctionValueLemmas.lean` holds the
built-in lemmas that read a function value.

### What each construct costs

Each construct lowers to the tactics the best script writes, or fewer.

- `assert e by c`: one `have` whose proof runs once in the block's scope. At
  a point that is once per path through the point; in an `on` block once per
  addressed obligation. The elaboration of `e` is the clause translator's,
  the one that elaborates the contracts: proportional to the term, no
  `simp`, no coercion or instance search, since instances are spelled as the
  leaf spells them.
- `apply φ`: a delta unfolding of the lemma's premise and conclusion
  (`unfoldLemmaDefinitions`), one bounded `decide` per premise, and the
  conclusion as a fact. A generic lemma's instance is a declaration stated
  once per instantiation, as a specialized contract is; its contract is
  never normalized at the application.
- `split e`: `verex_cases` or one `Decidable.em`, as a script's; placed at a
  point it splits the path once where a script splits each obligation.
- `forall`, `calc`, `witness`, `let`, `if`: an `intro`, a chain of
  `assert`s, a `refine`, a substitution, a `by_cases`. `split i in a..b`,
  `let V { … } := e` and `by contradiction`: an `rcases` over the literal
  cases, one `obtain` of the payload, a `by_contra`.
- `on key`: a syntactic match of the key against the leaf's recorded
  provenance; no tactic runs to find the obligation.
- `when`: a lookup of the leaf's recorded decisions before the paths are
  combined, so an addressed path is never joined into a disjunction and
  taken apart again (`combineLeaves`, then `verex_paths`).
- `decide`, `arith`, `eval`: one bounded attempt each. An obligation the
  proof does not address gets one `decide`.
- `ring`, `congr`: one bounded attempt each, one `grind` module under a
  configuration without splitting or instantiation. A use above that is a
  defect of the restriction, not a price.
- `solve`: the automatic leaf under its cap, charged as such; the one step
  that can cost more than a script line, by request.
- `lean =>`: what it writes.
- Spec-form rendering: delaborators run when a message or the InfoView
  prints a goal, never in a successful check.

The per-step cost of a checked item is in the stage log (`verex.stageLog`),
which records each step's heartbeats and its remaining goals.

## Worked examples

The user chapter holds the examples of the language as it is written. The
two below are the ones the lowering turns on.

Facts placed at a program point are owed once and shared by every clause of
the point, which is what a `preparing` block does by hand:

```text
verify test_verify_drain_symbolic by
  at loop 1 step {
    let k0 := spec_key_at(m, 0)
    assert spec_contains_key(m, k0) by apply lemma_contains_key_key_at(m, 0)
    assert len(m') == len(m) - 1 by apply lemma_len_remove(m, k0)
    forall i : Int where 0 <= i && i < len(m') {
      assert spec_key_at(m', i) == spec_key_at(m, i + 1)
        by apply lemma_key_at_remove(m, k0, i), lemma_rank_key_at(m, 0)
    }
  }
```

The invariant steps close by `decide`: each is an instance of its invariant
at `i + 1` rewritten by the stated equalities, which the cheap decider does.
Nothing names a path, a prepared form or a Lean lemma.

A selector routes the paths of one clause separately, where a script named
them by traversal number:

```text
verify buy_in by
  on aborts_if 1 when aborts at op 3 {
    assert old(self).total_coins + coins_amount > MAX_U64 by arith
  }
  on aborts_if 1 when aborts at call 1 of add_shares {
    split spec_contains_key(old(self).shares, shareholder)
  }
```

## Obligation addressing and provenance

An obligation key is what `on` names and what every message prints
(`ObligationKey`, `Solve/Base.lean`): a kind, an index among the clauses of
that kind, and the parts that locate it — a loop and a phase, a callee and
the number of the call, the point of a point `assert`, the struct a data
invariant is of, and the other clauses a leaf owing several at once owes.
`ObligationKey.text` renders it as a proof and a message spell it
(`ensures 1`, `invariant 2 step`, `invariant 3 of loop 2 entry`, `requires 1
of iter_next at call 2`, `after call 1 of add_shares`, `assert 1 of at
entry`), `ObligationKey.tag` as a goal's tag, and `ObligationKey.selects`
decides which leaves an `on` pattern takes: the same kind, and every part
the pattern states.

| Kind | Key |
|---|---|
| Postcondition, abort condition | the clause's kind and index in its `spec` block: `ensures k`, `aborts_if k`, `aborts_with k` |
| Callee precondition | `requires k of g at call j`: the clause of `g` and the `j`-th call of `g` in the body; without `k` where no clause of `g` states it (a data invariant of an argument, a map role's precondition) |
| Continuation after a call | `after call j of g`, a point |
| Continuation after an aborting operation or an invocation | `after op j`, `after invoke j`, points |
| Loop invariant | `invariant k of loop j entry\|step`: the clause and the loop's site index |
| In-body assertion | `assert k`: the `k`-th `spec assert` of the body; `assert k of p` the `k`-th `assert` of a proof's point block |
| Data, module, stored invariant; lemma requirement; ghost domain | the kind, and the site where several; `data invariant k` and `stored invariant k` the struct's own numbering of the clause; `data invariant k of S` routes a function constructing two struct types by the struct, for routing only |
| A path to any of these | the key with a selector, `when` and the decisions on the path |

The contract marks each `requires` clause, the walker strips the marker into
the leaf's clauses as it does a loop invariant's, and the generator numbers
every function's preconditions and the target's calls in source order
(`numbering`, `calling` of `verex_obligations`); a failed precondition is
reported at its clause and call.

**Decisions.** The compiled body carries the VIR site of each conditional,
throw, aborting operation (checked arithmetic, a cast, a vector access, a
resource access), loop, invocation and `match` arm; the denotation marks a
branch, an arm, an abort and an operation's or call's failures definitionally
(`branchAt`, `armAt`, `abortAt`, `opAt`, `callAt`), and the markers reach
the goals as reducible conclusions (`Taken`, `Matched`, `AbortsAt`), so the
normal form and the deciders see through them. The walker takes them off
where it steps a goal and records the decisions on the path, a field of the
leaf (`Decision`, with `Decision.text` the selector's spelling); `on … when
…` takes a clause's paths apart where a selector names them. The generator
numbers the sites per kind in source order (`siting`); a conditional is an
`if` only where the source writes one, not an `assert!`, `&&`, `||` or a
`while` loop's heading, which is the loop's `exits`/`continues`, so the
short-circuit rules still merge the others. An invariant clause keys a leaf
that owes it (`keying`) where its step names no invariant itself. A selector
that matches no path is an error that lists the decisions of the key's
paths, reported once the target's proof ends, since an alternative can run
once per case of an enum parameter.

A `match` lowered to variant tests keeps its provenance on the arena node
(`Expr.origin`, `ExprOrigin.matchArm`: the match's location, the arm's
variant, the untested last arm's variant), set by VerexSL's decision trees
and by the import pass for patterns through references; the site table
numbers it as one `match`, and its arms record `match k V`. The source
printer writes such a chain, and a match node it lowered itself, as `match`,
so a Move source's verification, which elaborates the printed text, keeps
the decisions. Two forms keep conditionals: a match of a move-only enum by
value, which VerexSL has no non-consuming `match` for, and an arm testing
more than one variant at a time; their paths take `if` decisions.

**Returning paths.** A returning path owes that no abort condition holds:
one obligation, `¬mustAbort`. The walker records the abort clause table's
ranges on that leaf (its `also` clauses, provenance `returns`), so the leaf
is keyed `aborts_if k` for every clause of the table, never by a traversal
name; the goal, the hypotheses and the continuation owners see the contract
term unchanged. An authored proof's theorem marks the obligation (`NoAbort`,
`verex_mark_returns`, at the proof's goal alone), and the walk takes the
marker off where it owes the obligation; an automatic goal has no marker and
its leaf is decided whole. Where an `on aborts_if k …` names one of its
clauses, routing rewrites that goal by `not_or` once, hands the clause's
conjunct to the alternative, and keeps the remaining conjuncts as one goal
keyed by the remaining clauses; a leaf none of whose clauses an `on` names is
never taken apart. `combineLeaves` groups leaves by their key set, so
returning paths combine as other clauses' paths do, and `verex_paths` and
the selectors take them apart first. The decision `returns` is recorded on
every completed path, so a block for one side selects it (`when aborts at
…`, `when returns`).

**Provenance.** `Provenance` (`Solve/Base.lean`) is what a leaf records of
its origin: a callee's `precondition` and the `continuation` after a call,
both with the callee and the number of the call; `loopEntry` and
`loopIteration` with the loop; `assertion`; the certification kinds
(`construction`, `mutationEnd`, `copyOut`, `handOver`, `exitMutationEnd`);
`memoryWritten`, `callReturned`, `storedWrite`, `ghostWrite`,
`lemmaRequirement`, `unrollBound`; `pointAssert` with the point and the
index of the `assert`; and `returns`. The counting of calls (`call j of g`)
is by source order of the call expressions in the body, and `call j` without
`of` counts every call; invocations are counted the same way, apart from
calls. All are stable under solver changes: a selector changes only when the
Move source does.

**Points.** `after call j of g` is the continuation of the `j`-th call of
`g`, `after op j` and `after invoke j` the paths on which the `j`-th
aborting operation or invocation did not abort, `at loop j entry|step` the
loop's entry and the start of its iteration body, `at entry` the first step
of the body, `at exit` every return. A point is where the walk is, not what
a leaf owes: it never changes a leaf's provenance or key. The walker runs a
point's hooks where it produces the point's goals: at the loop rule's
application and the iteration's start, and, after a call or operation a
proof names, at a marker the walker places on the continuation's conclusion
(`AfterAt`, opaque, so that the normal form rewrites under it but never
distributes the path across it), met on every path that continues there,
however the normal form duplicates or decides the paths around it. A hook at
a point no path reaches is an error. Points are numbered as the decisions
are.

## Semantics and lowering

The language lowers onto the solver's authored machinery; nothing in the
walker, the contracts, the budgets or the reporting of automatic targets
depends on it. The lowering is a translation to the tactics a script writes,
not a tactic of its own: it emits no `first`, no `try` over alternatives and
no speculative decider, and it prepares each goal at most once — the path's
hook or the obligation's alternative, never both; a goal prepared in an
outer scope is marked (`preparationOwners`) and an inner scope reads the
mark.

1. **Items.** `verify f by block` owns the target's whole goal, with no
   automatic attempt first. `lowerProofBlock` (`VerexSL/Verify/Steps.lean`)
   emits three tactics: `verex_step_routing` naming the keys the block's
   `on` steps state (with `all` where the block has an `on _`);
   `verex_obligations` with one point hook per `at`/`after` step and, where
   the block has obligation blocks, a `preparing` clause holding
   `verex_step_prepare` and the steps of `at exit`; and `verex_step_route`
   over the block. A block that is one `lean =>` step lowers to that script
   alone. `verex_step_route` turns each routed step into one
   `verex_step_obligation` alternative, carrying the step's keys and
   selector, and adds an implicit alternative per `assert` of a point block;
   it then groups the leaves (`groupLeaves`) and routes them
   (`routeLeaves`). A `lemma` item lowers to a `LemmaDecl` of the unit and
   is verified as a Move `spec lemma` is.
2. **Expressions.** A step's `e` is elaborated by the clause translator
   (`VerexSL/Contract/Clauses.lean`) in a scope built from the goal's
   context: the source-named locals and parameters preparation establishes,
   `result`, the obligation's own binders, the block's `let` bindings, and,
   under `split`, a variant's fields. The translation of
   `spec_contains_key(m, k)` is the map predicate over the same `m` the goal
   holds; no transport. The elaborator spells instances as the normal form
   spells them in the leaf, so that a stated fact rewrites the leaf's terms:
   the proof works on the leaf's own terms, and the normal form keeps its
   spelling. A function-typed expression elaborates to the function
   carrier's interface
   ([`higher-order-functions.md`](higher-order-functions.md)) — construction,
   invocation, equality and order through it.

   The expressions are lowered by the module's own lowering, in an
   unregistered copy of the module that declares one specification function
   per expression whose parameters are the names in scope
   (`StepExpression`, `LoweredStep`): the function's locals by source name,
   each also as `x'` and as `old_x` (what `old(x)` reads), among them the
   binders of its clauses' quantifiers; each callee's result as the leaf
   names it — in evaluation order the last call's `g_result`, the earlier
   ones' `old_g_result`, `old_g_result_1`, …, a tuple's components `…_k` (a
   name given twice is an error at the item); and the binders of the
   enclosing `forall` steps. A `let` is substituted. A `witness` has the
   type of the existential binder it supplies, a split expression the type
   its lowering infers, every other expression is a Boolean. The copy's
   syntax holds the proof file's items at positions shifted into a band of
   their own, and the map lemmas in a third: the lowering records each range
   in its source's file (`CompilationUnit.sources`, `sourceBand`), and a
   diagnostic of a step expression is reported at it. At the leaf, after its
   preparation, the binders of the obligation's clause, which preparation
   introduces without names, take the clause's binder names.

   A name of the scope reads its scope entry (`ScopeEntry name index
   value`), an inert fact created where the walk binds the name: a parameter
   at the theorem's introduction, before its pattern takes it apart; a
   call's result as a premise under the binder the call names, so that a
   substitution eliminating the binder keeps the entry with its value; a
   loop's slot value; a clause's binder; and the prophecy of a borrow of a
   whole local `x`, a `&mut` parameter's reborrow included, as `x'`, the
   value written back. A case split or a substitution rewrites the entry as
   it rewrites the leaf, so an enum's value is in scope in every case, a
   variant without fields included; a renaming of the local renames its
   entry, and the latest binding of a name, by index, is the one in scope:
   at an `ensures`, `x'` is the final value written back. `old_x` is the
   earliest binding of `x`, its value at entry, where no binding is named
   `old_x`. A mutable reference's entry holds its current value. Generic
   introductions bind no entry. A value the leaf holds only in parts has no
   entry, and its name reads the parts under the names the leaf gives them
   (`x_f`, `x_k`); a frame local the walk never binds is read by the
   result's name. A source `let` is a binding site: the denotation marks it
   (`letAt`, `letRowAt`), and in a step proof the normal form states what
   its body owes under the binding (`LetIn`, `LetRowIn`), which the walk
   meets and turns into the entries of the locals of the target's own frame
   the scope names; elsewhere the mark is erased. The exit is a binding site
   of the returned value: where the postcondition meets it (`ResultAt`), the
   walk binds `result`, and with several results `result.k`.
3. **Facts.** `assert e by c` is `have h : ⟦e⟧ := by ⟦c⟧` on every goal of
   the block's scope, `h` named by the step's index and marked a fact of the
   block by a hidden `BlockFact` beside it, which a substitution or a case
   split carries along; it is reported at the step on failure. The fact is
   normalized once, as a term, and that normal form is proved and given. An
   equation of a name and a value or another name is substituted over the
   context; one of a term and a name folds the term into the name over the
   context, once; any other fact is a rule of the block's normal form and
   rewrites no hypothesis. The rules rewrite a fact's higher side to its
   lower one — a term above a name above a value, two names toward the one
   declared first — and a name equated to a term holding it is no rule; the
   leaf's definitions of named values fold the same way. Every rule thus
   folds a term into a name, takes a name to an older one or to a value, or
   is a normal-form rule, so the normal form ends. A conditional reduces
   where its condition, or the condition's negation in canonical form, is a
   hypothesis, and a conjunction where each conjunct is or one is refuted:
   the facts decide the conditionals they state, by their spelling, and no
   arithmetic runs in the normal form; a condition the facts only imply is
   stated by the proof. A fact asserted under `if` or `forall` is an
   implication and never a rule; to rewrite by it, `split` or state the
   equation at the top of the block. A leaf equation of a term and a built
   value (`m k = some v`, `xs[k]? = some v`, `o = none`) is a rule rewriting
   the term to the value. A fact whose normal form is an existential is
   taken apart as `split`'s case facts are, and a leaf's existential
   hypotheses are not opened at preparation.

   `apply φ` is the lemma machinery of [`lemmas.md`](lemmas.md)
   (`verex_lemma_owed`, `verex_lemma_holds`; `LemmaApplication`,
   `LemmaInstance`) at an authored site: the premises as `assert`s by
   `decide`, the conclusions as facts. A generic lemma is instantiated by
   `ensureSpecialization` from the argument types
   ([`native-verification.md`](native-verification.md), SC1–SC5), as a call
   site is. `witness` is `refine ⟨⟦e⟧, ?_⟩` at the obligation's existential,
   a step of an `on` block; it applies to every obligation its `on` names,
   each of which must state the existential, and a key naming an obligation
   without one is an error at the witness. A witness lowers at the binder's
   type where the key names one clause at one instantiation; where the key
   names a clause the leaf instantiates (`data invariant k`, `stored
   invariant`), the witness lowers at its own type and the leaf coerces it to
   the binder's — as it is, as the typed view's value, as an integer with its
   owed range, as its native value — and a witness no coercion reaches is an
   error. A bounded integer's range and a certificate are owed by `decide`
   at the witness. `calc` is a chain of `assert`s: each link a fact by the
   closer, the conclusion by `decide` from them. `forall x where g { … }` is
   `have : ∀ x, g → F := by intro x hg; …` with `F` the conjunction of the
   facts the block asserts (its `assert`s, its `calc`s' conclusions, its
   `forall` and `if` steps' facts; an `apply` is the block's means, not its
   fact), closed by `decide` after the block's steps; a bounded binder's
   range is a premise beside `g`. `if g { A } else { B }` is the same
   without binders: it gives `g → F_A` and `!g → F_B`, the obligation's
   proof unsplit. `assert e by contradiction { … }` is `by_contra` with the
   block's steps after it and `decide` on `False`.
4. **Cases.** `split e` on a Boolean is `rcases Decidable.em ⟦e⟧`; on a
   variant local it is `verex_cases x` with the payload fields renamed to
   their Move field names, and the block continues on every case; on an
   optional value it splits the model's `none`/`some`. A split expression
   that is not a local is generalized first, in the goal and the hypotheses
   that hold it, the generalization's equation kept as the case's fact. A
   case fact the normal form states as an existential or a conjunction (an
   optional model value a role's result reads) is taken apart into its
   witnesses and facts, which are normalized and settled under the
   witnesses' bounds. The equations of names and values a split leaves in a
   case are settled as a fact's are. A case of a term that reads a map's
   model gets the facts the map's positions give, a key's neighbour
   included; a split of a local gets none, and a proof states what it needs
   of the positions by the built-in lemmas. `split i in a..b` is an `rcases`
   over `i = a ∨ … ∨ i = b`, owed by `decide`, with `subst` per case.
   `let V { f, … } := e` owes `e is V` by `decide`, then generalizes and
   splits `e` as a split does, the cases but `V` refuted by the owed fact; a
   field `f` reads `(e).f` in the steps after it, which the case's fact
   reduces to the payload. `if` guards without splitting.
5. **Points.** `at p { … }` and `after s { … }` run the block's steps
   (`verex_step_point`) where the walk reaches the point: `at entry`,
   `at loop k entry|step` and `after s` as hooks of `verex_obligations`, no
   goal after an abort, and `at exit` in its `preparing` clause, at every
   completed path before its clauses split. The hooks are keyed by
   provenance (`splitStep`, `ProgramPointKey`, `pointOfSyntax`). Facts
   asserted there are owed once per path through the point and inherited by
   the obligations after it, including the clauses that split later. An
   `assert e` of a point step is lowered as the Move proof block's in-body
   assertion is ([`lemmas.md`](lemmas.md), "Function proofs"): an obligation
   of its own, `assert k of p` (the `k`-th assert at the point `p`), which
   the walk queues as a leaf where the path reaches the point, routed like
   any obligation and given to the paths after the point; `assert e by c` at
   a point is shorthand for `on assert k of p { assert e by c }`. The other
   steps of a point block (`let`, `apply`, `split`, guarded blocks) run in
   the hook, their facts given there. A point binds the locals the body has
   bound there — the slots of the environment the walk carries, a
   reference's current value — each under a binder of its name with its
   equation; a slot holding a value already in scope is read by that value's
   name, and a local bound in code the walk has reduced to values holds no
   slot, so a point after a call is where a body's `let` comes into scope. A
   reference-typed name's prophecy is `r'`.
6. **Obligations.** `on key { … }` is one routed alternative
   (`verex_step_obligation`) that first prepares the obligation, runs the
   block, and closes by `decide` with the
   facts, in the block's normal form (the facts and the leaf's definitions
   of named values as rewrites), with no decider after it. `on _` is the
   default alternative; an unnamed obligation gets one `decide`. A selector
   restricts the alternative to the leaves whose recorded decisions match
   it, before `combineLeaves` joins the rest; a point with a selector runs
   its hook on the matching paths only. A block can end with nested `on`
   blocks: its initial steps run over the shared path, then the children
   receive their clauses and clause-local binders, in source order,
   overlapping selections included; their keys must be within the parent's
   selection, and path selectors belong on the parent.
7. **Escape.** `lean => tac` runs `tac` on the goals of the block's scope.

The `verex_*` tactics stay defined, for the lowering and for `lean =>`: the
preparation tactics (`verex_prepare` and its variants, `verex_paths`,
`verex_intro_any`, `verex_finish_prepare`, `verex_normalize_context`,
`verex_subst_vars`, `verex_type_bounds`, `verex_name_maps`, …), the case
tactics (`verex_cases`, `verex_split`), the deciders (`verex_decide`,
`verex_assumption`, `verex_leaf`, `verex_linear`), and the solver's
internals. The discipline is that a proof in the language names none of them
directly.

## Who owns preparation

The system. A proof never says how a context is prepared; it says which
values to split and which facts to state. The lowering chooses the cheapest
preparation the obligation needs: the value preparation for every leaf, and
the map naming (`verex_name_maps`, which gives a map's size bounds and typed
reads) where the block or the obligation reads a map — a name the block
reads holds a map, a step applies a map lemma, or the clause of one of the
`on`'s keys has a map's type in it (under `on _`, the leaf's own clause; a
key naming no clause, by the obligation's statement). A proof over maps
whose clauses read none states a fact that reads one where it needs the
maps' bounds. A `split` names the values it splits.

The value preparation destructures an integer local the context bounds above
and below by one literal and substitutes the literal (`pinnedInteger?`,
`substitutePinned`, `VerexIR/Proofs/Denote/Solve/Base.lean`), so a computed
position reads at the value; that local then has no source name in the leaf.
A local the bounds fix to a term is not prepared that way: reading it as the
term is an alternative of the leaf decision (`verex_pin_terms`, tried by
`verex_leaf_attempt` in `Solve/Decide.lean` in front of `verex_leaf_decided`),
which keeps the local and its equation and is undone when the decision does
not close the leaf with it. The first is an open problem in
[`../plan/proof-language.md`](../plan/proof-language.md), "A literal-pinned
local cannot be named by an authored proof".

A completed path whose obligations several `on` steps take is prepared once
before its clauses split (`verex_step_prepare`, in the walk's `preparing`
hook), its clauses' markers kept, since they name the leaves; each leaf then
adds only what its clause brings (its binders and premises, their bounds),
reads its clauses unmarked and reduces the projections a substitution into
them leaves. A path with one such obligation is left to its leaf: preparing
it twice, once for the path and once for the clause, costs more than once.

The facts a proof states at a point are prepared once with their context and
shared by every obligation after the point. The design asks authors to
prefer points over `on` wherever a fact is not clause-specific, as
[`performance.md`](performance.md) recommends ("Authored proof interface").
Incremental preparation is a change inside the lowering: once the rule set
commutes with substitution and the walker can carry a path's prepared
context ([`performance.md`](performance.md), "Open levers"), the point hooks
prepare a fact when it is introduced and the obligations inherit it, with no
change to any proof text, because no proof text mentions a normal form.

## Normal-form fixes

Some escapes exist because a solver gap shows through the proof. They are
fixed in the normal form or in preparation, not by a construct, so that
proofs stop naming them. Each is a rule of the normal form, gated on the
automatic targets.

- **Transports of concrete types.** A transport of a concrete type or of a
  type parameter is the identity (`NTy.toSkolem_address`, `NTy.toSkolem_param`
  and the rest of the `vir_transport` set, with the family identities
  `FamilyTransport.toSkolemIdentity` builds and the container identities
  `mapTransportIdentity`, `optionTransportIdentity`,
  `paramFreeValueTransport`), including where a transport is mapped over a
  container in a generic search. No proof file registers a normal-form rule.
- **Type bounds of values read from typed containers.** A key read by a
  position or returned as a neighbour key, and an integer payload field of a
  variant, lie in their Move type's range. Preparation adds these bounds as
  it adds those of parameters (`verex_type_bounds`): from a map's
  `KeysSatisfy` and `ValuesSatisfy` facts, the bounds of a key at a
  position, a key stated present, a successor or predecessor
  (`nextKey_satisfies`, `prevKey_satisfies`, with no presence premise) and a
  value at a key; after a variant split, the payloads' bounds. It then reads
  the round trip of a bounded value as the value, by the cut's lemma at the
  bounds. A read of the key vector at a position is the key at the position
  (`getElem_keys_toNat`), so the vector of keys has no bounds of its own.
- **Validity of maps equated entry by entry.** Preparation gives the
  validity of a map equated entry by entry to a valid map, by the solver's
  lemma that validity reads only the key sequence and equal entries have
  equal keys. No role or spec function names a key sequence or validity.
- **The specification form of an intrinsic operation's predicate.** The
  model states the abort of building a map from vectors over its key list;
  the Move contract states duplicate indices. The authored preparation and
  the block's normal form read the first as the second
  (`abortsNewFrom_indexed`, in the set `vir_denote_spec`); the automatic
  normal form keeps the atom, as it keeps every role abort predicate, which
  its lemmas and deciders consume and its context rewriting would otherwise
  instantiate. A lemma's premise is matched in the normal form the context
  has, so a lemma states a role's atom whichever form the context carries.
- **A row's reads.** `(e.1, e.2)` is `e`, and `(e.1, ())` is `e` where the
  spelling of `e`'s type shows a row of one component at reducible
  transparency, nothing more.
- **A concatenation's size** (`Array.size_append`, `size_push`), so a body's
  bound is linear in the sizes.
- **A nest of concatenations is one concatenation**, under one premise: the
  sum of the leaves' lengths at most `MAX_U64`, lengths as integers in the
  order of the leaves, matched in the normal form's canonical comparison, so
  a proof states that one bound (`Solve/Normal.lean`).
- **One spelling of a defined value.** `∃ c : τ, c.val = v ∧ p c` is read as
  the range of `v` and `p` at the cut value wherever it occurs, clause
  binders included (`existsDefinedValue`); where `p` reads `c` only by its
  value, at the value. A `let x : T where φ` whose `φ` defines `x` by an
  equation with an integer term lowers to the same form, `x` the scope name
  of the cut value and its range among the step's facts: no witness local,
  no substitution.
- **A clause's binder keeps its name.** A conjunction under a quantifier
  splits with the binder's name kept (`forallAndKeepingName`), so a clause's
  binders stay in a proof's scope and a hypothesis's name is the source's.
- **A literal position.** `Int.toNat` of a literal is evaluated in the
  normal form and in a rule's rewrite (`literalToNat`), so a read a case
  leaves at a literal is the read a step states.
- **A clause's premise over the path's hypotheses.** A path prepared before
  its clauses split holds conditionals a clause's premise decides; the leaf
  reads those hypotheses in the block's normal form, as `decide` reads the
  target.
- **Spellings that hide an equality.** Address equality and a vector read
  spelled as a defaulted lookup the context resolves are spelled `==` and
  `v[i]` in the normal form (`NTy.eqb_address`, `Option.getD_some` in
  `Solve/Normal.lean`'s sets), where `decide` and `congr` see them.
- **One spelling of a quotient.** A truncating quotient or remainder of a
  nonnegative dividend is the Euclidean one, `/` and `%` of unbounded
  integers, as in the Move Prover (`tdivOfNonnegative`,
  `tmodOfNonnegative`, in `vir_denote_norm`, with the sign discharged by
  `nonnegative?`, which derives a product's and a sum's sign); `Int.tdiv`
  and `Int.tmod` remain only where the dividend's sign is open. The solver's
  quotient facts are stated over `/` and `%` for non-literal divisors.
  Spelling quotients as `tdiv` instead fails: `omega` reads `/` and `%` by
  literals and `tdiv` as an atom. At a divisor that is not a literal the
  facts are the bounds and the cancellations; the division algorithm is left
  out, because adding it stops the instance decider closing leaves unrelated
  to the division, and a proof states it by the division lemma. A `lemma`
  item's proof reads its conclusion in the normal form under its `requires`,
  so its steps see `/`.
- **One spelling of membership.** The leaf's normal form reads `e ∈ v` by
  its positions (`mem_iff_exists_int_index`), so the deciders see the
  positional spelling they decide; the step normal form keeps `e ∈ v` an
  atom with its algebra (`mem_push`, `mem_append`, `mem_singleton`,
  `mem_map`, `mem_swapIfInBounds`) and reads a search for an element and its
  negation as the atom (`exists_mem_val_eq_iff`, `forall_mem_val_ne_iff`);
  the block's decider bridges the two (`statedProof?` proves an atom or its
  negation from its positional statement, also at a position a split left,
  and the converse). The bridge is not the design, and the deciders' own
  theory of membership is open
  ([`../plan/proof-language.md`](../plan/proof-language.md)).

## Budgets

The budgets and the leaf cap are [`proof-strategy.md`](proof-strategy.md)'s;
the language adds no allowance of its own. What is the language's:

- The item owns the target's goal under the target budget
  (`verex.verifyHeartbeats`); no automatic attempt precedes it, and facts
  placed at points are proved under that budget, as authored preparation is
  ([`proof-strategy.md`](proof-strategy.md), "One leaf cap").
- `solve` is the only step that spends the automatic leaf cap
  (`verex.leafHeartbeats`), charged as automatic work; every other closer is
  bounded as its own tactic is and retries nothing. An obligation no `on`
  names gets `decide` alone, and the failure request never asks for `solve`.
- The cost contract is per item: the heartbeats of a target under its
  lowered proof, read from the stage log (`verex.stageLog`), are at or below
  the target's heartbeats under the tactic script it replaces. An item above
  that is a defect of the lowering, found by the stage log's preparation and
  closer lines, never accepted as the price of the language.

## Messages and templates

Every message names an obligation by its key and states goals and facts in
spec form. Spec form is the clause translator run backwards: delaborators
(`VerexSL/Verify/SpecForm.lean`) print the native vocabulary as the
specification writes it — a map read as its role function, a certified
integer as its value, a specialized contract under the callee's name, the
obligation markers not at all. They serve the InfoView as well as the
driver's messages, so the goal under the cursor in a proof file reads as the
proof is written. The delaborators print where `verex.specForm` is set, the
role functions taken from `verex.specRoles`, which the verifier states from
the module's maps (`withSpecForm`); a successful check prints nothing.

A failed automatic verification names the function, says whether the
bounded attempt failed or ran out of its budget (and, over budget, how many
leaves it decided and at which call and stage), and asks for a proof in the
module's proof file, with one alternative per leaf it left undecided
(`verificationFailureMessage`, `VerexSL/Verify/Diagnostics.lean`, and
`proofRequest`, `VerexSL/Verify/Contracts.lean`). A leaf is named by its
obligation key where a clause keys it (`leafKey`, `ObligationKey.tag`), so
the request already names clauses, calls, loops and phases rather than
traversal numbers. The skeleton it prints is a tactic script
(`verex_obligations`, `verex_on`, one `verex_prepare; sorry` per leaf):
printing the request as blocks of the language, with each obligation's goal
and available facts in spec form, is open
([`../plan/proof-language.md`](../plan/proof-language.md)).

A failing step reports at the step, in spec form: the fact it owed, and the
goal the closer did not close (`finish`, `assert` and a lemma item's
conclusion each report their own). A `split` that leaves a case unproved
names the case.

## Suitability for generated proofs

An agent writing proofs over tactic scripts reads a Lean goal in the
solver's normal form, chooses among the solver's whole tactic namespace, and
learns by failure which preparation produces the shape a lemma matches. The
language limits what it must know to the Move file and the message:

- the vocabulary is the specification's, which the agent has already read;
- the obligation key tells it which clause at which site, so it can read the
  Move code there;
- the facts it needs are the lemmas by their names, a finite list;
- each step fails individually with the fact it owed, so repair is local;
- there is one way to say each thing, so the proofs it has seen are the
  proofs it should write;
- a proof it writes is cheap to check by construction: every closer is one
  bounded attempt and the lowering emits no cascade, so a generated proof
  cannot spend heartbeats on alternatives, and only `solve` can spend the
  automatic cap.

`lean =>` is permitted to generated proofs as to any other. A generated
proof that needs it signals that the language or the lemma set is missing
something, to be fixed there.

## Host

Two hosts carry authored steps: the Move proof block (`spec f { … } proof
{ … }`, lowered through the exchange into in-body specification statements,
[`lemmas.md`](lemmas.md)) and the proof file. The language is one step
grammar with two placements; the hosts differ in which placements they
carry.

- The Move proof block carries point-placed steps only: the function's entry
  (its statements), `post` (exit), and `if`-guarded steps; `assert`,
  `assume`, `apply`, `split`, `calc`, `let`. It cannot carry `on`, closers
  or `lean =>` without putting Lean in Move sources, and it is parsed in
  Rust, so every step would be implemented twice.
- The proof file carries both placements, the closers, `lean =>`, and the
  theorems a `lean =>` step needs. Its steps are parsed by VerexSL, which
  also parses the lemma proof steps and the in-body `spec assert`/`spec
  apply`/`spec split`, so one step grammar serves both hosts.

A fact placed in either host means the same: an `assert` at a point is an
obligation in both, proved by the same means, and given after the point. The
Move block's `assume` keeps its own lowering as an in-body `spec assume`; it
is Move's statement, and a proof file's point block cannot assume. A user
who wants a fact to travel with the source writes it in the Move block; a
user who needs an obligation-level argument, a closer or Lean writes the
proof file. `on` and `lean =>` stay out of Move.

Precedents weighed: Verus (`assert(e) by (arith)`, `assert forall |i| … by
{…}`) gives the closer-on-assert and the quantified block; Dafny (`forall …
ensures … { }`, `calc`, `assert … by {}`) the `forall` statement and `calc`;
Isar (`have … by …`, `obtain`, `case`) the named facts and `witness`; F*
(`assert e by tac`) the escape; the Move Prover the step names and the point
placement. None addresses obligations, because none presents them; `on` is
the one construct without precedent, and it exists because Verex presents
obligations rather than one verification condition.
