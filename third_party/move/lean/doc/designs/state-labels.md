# State labels

A Move specification names intermediate states of a function's execution
with *state labels* and reads its two-state predicates at them:

```move
spec create_then_read {
    ensures ..S |~ publish<Resource>(signer::address_of(account), Resource { value: 42 });
    ensures S.. |~ result == result_of<read_resource>(addr);
    aborts_if S |~ aborts_of<read_resource>(addr);
}
spec inc_twice {
    ensures exists S in *: (..S |~ counter_increased(c)) && (S.. |~ counter_increased(c));
}
```

This design carries labels end to end: the exchange, VIR, VerexSL,
contracts, and the solver. It follows the sketch in
[`higher-order-functions.md`](higher-order-functions.md) ("Behavioral
predicates and state labels"): a contract is a proposition over states, so
a label is a state variable. Open work:
[`roadmap.md`](../plan/roadmap.md).

## Semantics, as the Move Prover defines them

A *memory range* `{pre, post}` is attached to every two-state operation: a
behavioral predicate (`aborts_of`, `ensures_of`, `result_of`, …), a
specification-function call, and the state-change predicates `publish<R>`,
`remove<R>`, `update<R>`. `pre` is the state `old(…)` reads, `post` the
state current reads; a missing label is the clause's default (the
function's pre-state, respectively its post-state). The surface forms are
`S |~ e` (both at `S`, for a one-state reading), `..S |~ e` (pre default,
post `S`), `S.. |~ e` (pre `S`, post default), and `S..T |~ e`.

A label is one of:

- **Defined** by a two-state operation whose post-state it is: a
  state-change predicate (`..S |~ publish<R>(a, v)` states that `S` is the
  pre-state with `R` published at `a` holding `v`; `remove<R>(a)` removes
  it; `update<R>(a, v)` replaces it) or an invocation's `ensures_of` or
  `result_of` (`..S |~ ensures_of<f>(x, r)`, `let b = ..S |~
  result_of<f>(x)`), whose post-state is the state after the invocation.
  The Prover records the defining operation per label (`LabelInfo`), and
  every other clause reading `S` reads that state. A label without a
  defining operation that is not quantified is an error of the Prover's.
- **Quantified** over the *state domain* `*`, the set of all memories:
  `exists S in *: …` and `forall S in *: …`. The Prover's own reading
  (`state_domain_quant.move`): "universally quantifying over all possible
  memory states", not over program points. A quantified variable of type
  `StateDomain` is not a local; it is resolved only in `|~` positions, and
  one that is not used as a label is rejected.

A one-state predicate at `S` (`S |~ aborts_of<f>(x)`) reads the globals of
`S` as the start of the invocation. `S |~ f(x)` of a two-state
specification function reads both `old` and current at `S`, which makes
`old(c).value < c.value` false there; the tests use this.

## Representation

**Exchange (XAST).** Labels are numbers (`MemoryLabel`) in ranges, the
state-change predicates (`spec_publish {pre, post}` and kin, with the
resource as a type argument), and one-state reads (`global<R, L>`,
`exists<R, L>`). The model keeps each label's source name
(`GlobalEnv::memory_label_names`); the Boogie backend ties a `StateDomain`
quantifier variable to its labels by that name. The exchange exports the
same: the dump adds `"label": n` to a quantifier binder of type
`state_domain`, resolved by its name through `get_memory_label_names`, and
a per-module table `labels: [{id, name}]` so a defined (free) label is
spelled by its source name too.

**VIR.** `MemoryRange` and the labeled operations (`behavior`,
`functionCall`, `lemma`, `publish`, `remove`, `update`, `global` and
`exists` with an optional label). A `QuantifierBinder` over the
`stateDomain` domain carries the label it binds (`label : Option Nat`,
present exactly for a state-domain binder). Validation (`Capability`)
checks that every label a function's specification reads is bound by a
state-domain binder or defined by a two-state operation of the same
specification (a definition may be restated across clauses, and a
quantified label may be defined in its body, as the Prover's tests do),
and that a state-domain binder binds a label. Labels are names (`NameId`s)
in VIR, function-local as the model keeps them; a labeled `exists` is
`SpecOperation.exists`.

**VerexSL.** Labels are names:

```
spec create_then_read where
  ensures ..S |~ publish<Resource>(address_of(account), new Resource { value := 42 })
  ensures S.. |~ result == result_of<read_resource>(addr)
  aborts_if S |~ aborts_of<read_resource>(addr)
spec inc_twice where
  ensures ∃ (S : StateDomain), (..S |~ counter_increased(c)) && (S.. |~ counter_increased(c))
```

A label name resolves to the label of the enclosing `StateDomain` binder of
that name, else to the specification's defined label of that name
(declared by its defining predicate, in any clause order); the lowering
numbers them. `|~` applies to a behavioral call, a specification-function
call, a state-change predicate, or a parenthesized expression of those (the
Prover's grammar allows `S |~ e` for any `e`; the lowering pushes the range
to the two-state operations in `e` and rejects one that has none). The
numeric `@n` forms remain for in-body assertion anchors, a different thing
with the same spelling; the printer spells a label by its name and never by
number.

## Denotation and contracts

A contract is a proposition over the pre- and post-states (`Memory unit`),
and every two-state operation takes the states it reads (`EnsuresOf unit …
S T …`, specification functions over `old`). A label is a `Memory unit`:

- a quantified label is the quantifier's bound variable: `∃ S : Memory unit`,
  `∀ S : Memory unit`;
- a defined label is `let S := publish pre a v` (`Memory.publish`,
  `Memory.remove`, `Memory.update` on the typed memory of `R`,
  [`static-memory.md`](static-memory.md)), and its defining clause is the
  proposition that this is what the function did between the two states it
  ranges over: `S = publish pre a v` for `..S |~ publish<R>(a, v)`; with a
  bound label on both sides the predicate relates two variables. A free
  label a state-change predicate defines is a memory expression, resolved
  across clauses in any order; naming it assumes neither resource presence
  nor the defining predicate. Chained definitions preserve other slots and
  resource types, and functional `update_field` values are carried.
  Lowering `old(e)` switches to the range's pre-state before lowering `e`,
  so an old-only read in a definition of `..S` introduces no dependency on
  `S`.
- an invocation label is `Proofs.StateOf`, the post-state chosen from the
  same successful invocation as `ResultOf`; the projection defaults to the
  pre-state when no successful run exists and adds no success assumption.
  `stateOf_eq_of_ensuresOf` proves that a successful execution determines
  this state (runtime determinism up to loan renaming, injective slot
  encodings, agreement on unnamed resources), and the call rules carry the
  state equation into callers.

A range `{pre, post}` then selects the states each operation reads; the
default states are the contract's. A binder over the state domain also
binds one copy of every `&mut` parameter, as the Prover keeps per label
(`value_state_vars`); a range installs a label's memory and copies as the
old or current state and locals. The contract carries defined labels into
an opaque caller: no program point of the callee is needed when its
contract replaces the call. Specification types see through references
(`seeThroughReferences`): a specification function over a `&mut Counter`
reads `c.value` as an integer, labeled or not.

## Verification

In a theorem, a label in a hypothesis (a callee's contract) is instantiated
where the solver uses the clause: a universally quantified label at the
state the solver reasons about, an existential one by introduction. A label
in the goal (the target's own `ensures`) needs a witness: for a defined
label the witness is its definition; for `∃ S in *` the candidates are the
states the proof knows, the pre-state, the post-state, and the states
recorded after each call. This is sound, and complete for the Prover's
tests, which assert existential splits at call boundaries.

A program point is a hypothesis of the leaf (`ProgramPoint value state`, an
implementation detail the deciders skip): where the solver unfolds a
let-bound continuation at a call boundary it asserts the `(value, state)`
the continuation is applied to, and the context's rewrites carry the point
to the leaf, which a ledger beside the goal would not survive (a
substitution renames the values). Points are kept only for a target whose
contract binds a label (`Contract.contractBindsStateLabel`). `verex_witness`
tries the points first for a label's existential: the point's state for a
`Memory` binder, its locals for the copies, the nested existentials of one
label jointly, the body normalized and decided conjunct by conjunct.

Where no call separates the states (`inc_both_twice` increments two
parameters in place) a copy is a plain existential over values, as the
Prover reads it: an integer binder's first candidates are the bounds its
body states (`t < k` gives `t + 1`, `k < t` gives `t - 1`, `k = t` gives
`t`), the conjuncts not reading the binder are decided once before any
candidate, and a nested existential is witnessed in turn. A universal label
is instantiated where its clause is used. The `*_mismatch` tests, which the
Prover's Boogie translator rejects over its copies, are read uniformly here
(an intended difference).

A label's existential whose body states `requires_of`, `aborts_of` or
`ensures_of` of a literal closure is witnessed at the first point where
every conjunct, instantiated there, is derived through the closure's
target (`dispatchBehavior`: the table of declared preconditions, the
permitted aborts of its verified contract), its derived facts out of their
clause markers, and closed by the cheap deciders (`behaviorWitness`).

## Reads of absent resources

`Memory` has `Option` slots and removal writes `none`, whose specification
read is the default value, whereas Boogie's `$ResourceRemove` clears the
domain bit but retains the contents. A clause reading `old(Resource[addr])`
under the pre-label of a publication after that label's removal of
`Resource` (`unmodified_memory_at_label::swap`) therefore reads the default
here. A specification that needs the removed value binds it before
entering the labeled expression, as the calculator benchmark does for
`S1.. |~ result_of<old(State[addr]).Continuation.0>(x)`.

## Not in scope

- Labels in in-body `spec { … }` blocks (the Prover does not allow them
  either).
- The anchors `@n` of in-body assertions: they stay as they are; whether
  they and labels should share a notion is a question for later.
