# Open problems of the proof language

The language itself — its surface, lowering, cost contract, keys, points,
closers and built-in lemmas — is described in
[`../designs/proof-language.md`](../designs/proof-language.md). This file
holds what is missing: each item states what is not there, why it matters,
what is known, and the reproducer or measurement that exists in the tree.
The tracks these items belong to are in [`roadmap.md`](roadmap.md).

## A proof-file `assume`

**Missing.** `assume e` is parsed and rejected in a proof file: "an
assumption of an obligation has no place in the target's theorem"
(`VerexSL/Verify/Steps.lean`, the `stepAssume` arm of the step
interpreter). **Why it matters.** The Move Prover's `assume` and Verex's
in-body `spec assume` can state an assumption where a proof needs one; in a
proof file the only route is to weaken the Move specification, which changes
what callers get. **Known.** The target's theorem states its hypotheses
before the walk, so a fact over an obligation's prepared scope cannot be one
of them, and a node in the body would change the function's semantics. The
design is a trusted contract per function: the proof file's `assume`s, each
under `on ensures k`, `on aborts_if k` or `on aborts_with k` and without a
selector, read as clauses of that kind over the clause vocabulary (`old_x`,
`x'`, `result`), marked by key and step, with the hypothesis that the
function's runs satisfy them beside `AssumptionsHold` in the theorem, in its
callers' theorems and in the ledger
([`../status/assumptions.md`](../status/assumptions.md)); the obligation's
proof then receives its own clauses' assumptions as hidden premises, which
the `assume` step names. **Reproducer.** Any `assume` step in a proof file;
the error names this file.

## Function values in the language

**Missing.** `split` on a function value, the invocation keys and decisions
that address an invocation's obligations, and the built-in lemmas of a
function value's behaviour. **Why it matters.** A proof whose argument is
about which closure a value is, or about what an invocation owes, has no step
for it and falls back to `lean =>`, where it spells the carrier interface and
so depends on the solver's spelling — the one thing the language exists to
prevent. **Known.**

- `split e` accepts a Boolean, an enum value, an optional value and an
  integer range (`splitKind?`, `splitStep`, `VerexSL/Verify/Steps.lean`). A
  function value's cases are the link set's closure sites plus `foreign`,
  which is a `cases` over the generated family at the base frame
  ([`../designs/higher-order-functions.md`](../designs/higher-order-functions.md),
  "Function values as native data"); nothing selects that family from a
  split step.
- `verexKey` (`Denote/Solve.lean`) has no `requires … at invoke j` form and
  `Provenance` no invocation kind, so an invocation's precondition is not
  addressable by `on`; the `invoke k g` decision exists (`Decision.invoke`)
  and selects paths, but the obligation it selects is keyed as the enclosing
  clause.
- The requires equations per closure site and the behavioural readings of a
  known target exist (`Denote/FunctionValueLemmas.lean`,
  `Proofs/Behavior.lean`); they are not entries of the lemma tables, so a
  proof reaches them only through `lean =>`.

**Reproducer.** The two items of
`verex-e2e-tests/VerexE2ETests/SourceVerify/behavior_manual_facts.proof.lean`
and `use_concrete` of
`move-prover/tests/sources/functional/closures/inline/specialize_generic_caller.proof.lean`,
the function-value items still written under `lean =>`; the closure proofs
that are in the language (`stored_fun_values.proof.lean`, the paper's
`amm_example.proof.lean` and `calculator.proof.lean`) argue over enum state
and arithmetic, not over which closure a value is.

## The deciders' own theory of membership

**Missing.** One spelling of `e ∈ v` in both normal forms. **Why it
matters.** Membership is the one place where the step normal form and the
leaf normal form disagree by design, and the bridge between them is a
decider, not a rule: a fact stated positionally and a goal stated as the
atom are related by `statedProof?` rather than by the normal form, so a
proof that mixes the two depends on which side each step lands on.
**Known.** The leaf's normal form reads the atom by its positions
(`mem_iff_exists_int_index`), the step normal form keeps the atom with its
algebra (`mem_push`, `mem_append`, `mem_singleton`, `mem_map`,
`mem_swapIfInBounds`) and reads a search for an element as the atom
(`exists_mem_val_eq_iff`, `forall_mem_val_ne_iff`, `Solve/Normal.lean`). The
wanted shape is a hypothesis `t ∈ xs` taken apart into a positional witness
and a goal `t ∈ xs` closed by finding one, with the atom in both normal
forms. A rule `e ∈ v ↔ ∃ i, … v[i]?.getD default = e` was tried and
reverted: `DenotePerformance` was unchanged, but
`verex-e2e-tests/VerexE2ETests/Check/Specifications/SpecFunctions.lean`'s automatic
`push` target
stopped verifying. **Reproducer.** That target, with the rule restored.

## A literal-pinned local cannot be named by an authored proof

**Missing.** A local the preparation pins to a literal is eliminated with an
inaccessible name, so an authored proof that takes the leaf over cannot
refer to it. **Why it matters.** The language's contract is that a proof
names source locals; this is the one preparation step that breaks it.
**Known.** `pinnedInteger?` (`Solve/Base.lean`) runs inside the preparation
every leaf receives (`substVars`, `verex_subst_vars`), and `substituteValue`
(called from `substitutePinned`) runs `cases` on the local with no given
names, so the integer's value gets a macro-scoped name and the local is
substituted away. The contrast is deliberate and documented at
`substitutePinned`: the literal case eliminates the local, the term case
rewrites and keeps it — term pinning is an alternative of the leaf decision
(`pinnedTerm?`, the `verex_pin_terms` stage, used by `verex_leaf_attempt` in
`Solve/Decide.lean`) and is undone where it does not close the leaf, so a
local that term bounds fix stays nameable while one that literal bounds fix
does not. Both cases assert their equation under the user name `pinned`; the
literal case substitutes it away with the local, while the term case keeps
it, so two term pins in one context leave two equations named `pinned`, the
second shadowing the first. **Reproducer.** An `assert` in an `on` block naming a local
whose bounds fix it to a literal; the step's elaboration reports the name as
unknown.

## The escapes the corpus still needs

**Missing.** Steps for the arguments the corpus still makes in Lean. **Why
it matters.** A `lean =>` step addresses goals in the solver's normal form,
so every item that uses one is an item a change of that normal form can
break: the escapes measure how far the stability boundary reaches.
**Known.** Nineteen `lean =>` steps remain, in ten proof files. By item,
with the goal each one stands at, read off the proof text:

| Item | Where the escape stands |
|---|---|
| `pool_u64::add_shares` | `ensures 1`, `ensures 2` and both data invariants of `Pool`; each step bridges the stored entries list and the map model (`SpecMap.ofEntries`, `eq_writeBack_of_ofEntries`, `SpecMap.update .sequence`, `size_update_sequence`) and reads a certified integer by its value |
| `ordered_map::test_verify_iter_sum_symbolic` | `invariant 3 step`, after `split`s and the recursive function's `unfold`: the step rewrites the stated fact by the invariant and aligns a named inhabitant wrapper (`NTy.inhabitant`) |
| `verify_vector::verify_swap_remove`, `verify_model_swap_remove` | whole items: vector extensionality (`SpecVector.ext`) over an erase and a swap, with the reads and sizes of the underlying array |
| `nested_loop_inv::assert_no_duplicate` | `aborts_if_1` and `invariant_6_step`: an existential over two positions, instantiated from the inner invariant in either order |
| `simple_vector_client::test_contains`, `test_index_of` | whole items: the witnesses of a membership existential at literal positions, and a ground evaluation the leaf's normal form does not finish |
| `vector_hofs_fold::count_even_concrete` | a whole item: the inlined lambda's specification function evaluated at each index of a literal vector |
| `closures/inline/specialize_generic_caller::use_concrete` | a whole item: an inlined generic callee's proved lemma instantiated at the caller's type argument (`count_is_len.lemma`, `Skolems.instantiate`) |
| `behavior_manual_facts::caller`, `wrong_final_value` | whole items: a cascade closing by `simp_all` over the normal form; the second is a negative fixture |
| `proof_lemma_quotient`'s lemma `scaled_down` | a lemma item whose whole block is Lean, over a quotient bound the built-in table also states |
| `proofs_errors::product` | a whole item of the error fixture, deliberately a script |

Three of these have a gap stated above: the membership witnesses, the
function-value facts of `behavior_manual_facts`, and the generic
instantiation of `use_concrete`. The others are unclassified: what the
language would need for each is read from its goal, not from the table.
**Reproducer.** Each file, with its `lean =>` step removed.

Beside the escapes, the items whose cost sits in the walk before any block
runs cannot be improved by a proof of any shape: the cost is the walk's and
preparation's, which
[`../designs/performance.md`](../designs/performance.md) ("Open levers")
owns. **Measurement.** Per-item elaborator heartbeats from
`scripts/verex-bench.py` against the stage log's walk and preparation lines.

## `on ensures k` inside a lemma item

**Missing.** A lemma item's block runs over all its clauses at once; there
is no way to address one `ensures` of a lemma. **Why it matters.** A lemma
with several conclusions needs one argument per conclusion, and they share a
block whose facts must serve all of them. **Known.** `verexKey`
(`Denote/Solve.lean`) has a `lemma requires` key and none for a lemma's
`ensures`, so there is no key an `on` could name;
`verex_lemma_steps` (`VerexSL/Verify/Steps.lean`) enters the parameters,
states the bounds, normalizes the conclusion under the premises and runs the
block, then closes by `decide`; the routing that `verify` items have
(`verex_on` over keys) is not applied to a lemma's conclusion. **Reproducer.**
A lemma item with two `ensures` clauses needing different arguments, in
`verex-e2e-tests/VerexE2ETests/SourceVerify/proof_lemmas.proof.lean`.

## `pool_u64::deduct_shares`' removal path

**Missing.** The removal path's obligations: the vector's distinctness after
the removal, the length and the key membership of the map named after the
removal (`old_value`), and the `ensures` read by membership. **Why it
matters.** It is one of the two targets turned off by `pragma verex_verify =
false`, and the one whose proof the language was expected to carry.
**Known.** The non-removal paths and the removal's length bound verify, and
the not-found search closes by the coverage lemma. Two spellings block the
rest: the map's size lemmas are computed by the normal form before the term
folds into the map's name, so a lemma about the removal gives nothing about
`old_value`; and membership's spelling (`v[i]? = some e`) differs from the
element read, which is the membership item above. **Reproducer.**
`pool_u64`'s spec block with the pragma removed.

## Gaps authored proofs of the framework reported

Each of these forces `lean =>` or a detour in a proof written against the
framework. They are reported, not designed. The targets named are the
reproducers.

- `result` is unavailable for the value written through a returned `&mut`
  (`big_vector::borrow_mut`).
- A `lemma` item's map parameter has no validity fact.
- The index a vector search returns has no name; the vector search operation
  does not translate in a step (`big_vector::index_of`).
- A read in bounds does not bridge a positional read of the model to the
  element read of the container, even with `solve`.
- `contains` resolves to a module's own function of that name, and `val in b`
  does not parse in a step.
- A step reading global storage at `at entry` reports "internal: a clause
  reads storage without a frame or a unit"; inside `if` the facts are
  silently dropped (`staking_config::reward_rate`, where repeated facts cost
  most of its cost).
- A selector naming a site that does not exist (`when if 99 then`) raises no
  error.
- Points cannot reach inside inlined callees, and an inlined callee's
  results have no name. The VIR of an inlined body does not record the
  callee its locals come from, so this needs that provenance from the
  frontend first; until then a step names only the target's own locals.
- A `let` inside an `if` branch is not in scope for that branch's `assert`s.
- A premise a case makes trivially true (`q == n - 1 ==> l == last` becoming
  `True → l == last`) is not read by `decide`.
- `decide` does not unpack stored invariants.
- `split` on a value whose certificates mention it fails: the
  generalization abstracts some occurrences only
  (`randomness::next_32_bytes`, `split seed`).
- An `assert` at `at entry` of a fact reading a generic specification
  function's cut hits the maximum recursion depth — deep, not infinite.
  Suspect: a carrier reduced through a transport.
- A stated equation cannot act as a rewrite rule on later facts (it is a
  fact, and only the equations the block's normal form accepts as rules
  rewrite).
- Obligations with no clause to key them have no key, so a proof can address
  them only by the tag the leaf carries.

## Constructor-specific proof branches

**Missing.** A proof-step `match` with a separate body per constructor of an
enum. **Why it matters.** `split e` runs the following steps in every
surviving case and `let V { … } := e` destructures one known variant;
neither gives a separate authored body per constructor, so a proof whose
cases need different arguments states them under guards or repeats the
split. **Known.** The wanted shape:

```text
match it {
  End => { /* steps for the end case */ }
  Node { key, value } => { /* steps using key and value */ }
}
/* shared steps for any remaining obligations */
```

It must check exhaustiveness, bind payload names and constructor facts
within their branch only, run the following shared steps on each remaining
branch obligation, and keep facts from leaking between branches; the
lowering should reuse the preparation before the split and prepare only the
new branch facts, so that the bounded-automation contract holds. The syntax
and the treatment of impossible constructors are open. This is proof case
analysis, distinct from the `when match k V` selector, which selects
verification paths through a program `match`.

## Invisible incremental preparation

**Missing.** The point hooks prepare a fact where it is introduced, and the
obligations after the point inherit it. **Why it matters.** It is the one
change the language was built to make possible: the saving is in preparation
repeated at every obligation of a path, and no proof text changes when it
lands, because no proof text mentions a normal form. **Known.** It waits on
a rule set that commutes with substitution and a walker that can carry a
path's prepared context ([`../designs/performance.md`](../designs/performance.md),
"Open levers"). **Measurement.** `upsert`'s preparation share against the
bound that lever states.
