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

This design carries labels end to end: the exchange, LIR, LeanerLang,
contracts, and the closer. It completes the state-label part of H4d in
[`higher-order-functions.md`](higher-order-functions.md) ("Behavioral
predicates and state labels"), whose sketch it follows: a contract is a
proposition over states, so a label is a state variable. Every test of
`move-prover/tests/sources/functional/state_labels/` (29) fails today, 18 of
them in the LeanerLang parser (registry entries V1, V20, and one message of
V2 in [`prover-test-problems.md`](prover-test-problems.md)).

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
  state-change predicate — `..S |~ publish<R>(a, v)` states that `S` is the
  pre-state with `R` published at `a` holding `v`; `remove<R>(a)` removes
  it; `update<R>(a, v)` replaces it — or an invocation's `ensures_of` or
  `result_of` (`..S |~ ensures_of<f>(x, r)`, `let b = ..S |~ result_of<f>(x)`),
  whose post-state is the state after the invocation. The Prover records the defining operation per label
  (`LabelInfo`), and every other clause reading `S` reads that state. A label
  without a defining operation that is not quantified is an error of the
  Prover's.
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
spelled by its source name too. XAST version 9.

**LIR.** `MemoryRange` and the labeled operations exist (`behavior`,
`functionCall`, `lemma`, `publish`, `remove`, `update`, and `global` with
an optional label; `exists` gains the same label, which the exchange
exports already). A `QuantifierBinder` over the `stateDomain` domain gains the
label it binds (`label : Option Nat`, present exactly for a state-domain
binder); validation (`Capability`) checks that every label a function's
specification reads is bound by a state-domain binder or defined by a
two-state operation of the same specification (a definition may be
restated across clauses, and a quantified label may be defined in its
body, as the Prover's tests do), and that a state-domain binder binds a
label. Label numbers stay
function-local, as the model keeps them.

**LeanerLang.** The numeric forms `@n |~`, `@n.. |~`, `..@n |~`,
`@n..@m |~` already exist for behavioral calls. Labels become names:

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
numeric `@n` forms remain for anchors (`@n` are in-body assertion anchors,
a different thing with the same spelling today); the printer spells a
label by its name and never by number.

## Denotation and contracts

A contract is a proposition over the pre- and post-states (`Memory unit`),
and every two-state operation already takes the states it reads
(`EnsuresOf unit … S T …`, specification functions over `old`). A label is
a `Memory unit`:

- a quantified label is the quantifier's bound variable: `∃ S : Memory unit`,
  `∀ S : Memory unit`;
- a defined label is `let S := publish pre a v` (`Memory.publish`,
  `Memory.remove`, `Memory.update` on the typed memory of `R`,
  [`static-memory.md`](static-memory.md)), and its defining clause is the
  proposition that this is what the function did between the two states it
  ranges over: `S = publish pre a v` for `..S |~ publish<R>(a, v)`; with a
  bound label on both sides the predicate relates two variables.

A range `{pre, post}` then selects the states each operation reads; the
default states are the contract's. The only change to the behavioral
predicates is that the states are no longer always the contract's
(`Contract.lean`, "a behavioral predicate with state labels is not carried
yet"); `spec.global` and `exists` take a label the same way.

## Verification

In a theorem, a label in a hypothesis (a callee's contract) is instantiated
where the closer uses the clause: a universally quantified label at the
state the closer reasons about, an existential one by introduction. A label
in the goal (the target's own `ensures`) needs a witness: for a defined
label the witness is its definition; for `∃ S in *` the candidates are the
states the proof knows — the pre-state, the post-state, and the states the
call rule records after each call (the anchors `Contract.lean` already
binds for in-body assertions). The closer tries them in program order and
takes the first the rest of the leaf closes with; a `∀ S in *` goal is a
universally quantified leaf, decided as other quantified leaves are. This
is sound, and complete for the Prover's tests, which assert existential
splits at call boundaries.

## Milestones

- **S1 — round trip.** Done 2026-10-05. Exchange label names (XAST 9), LIR binder labels and
  validation, LeanerLang names for labels and the labeled forms for
  specification-function calls, state-change predicates, and one-state
  reads, printer and lowering. Gate: every `state_labels` test re-elaborates;
  the 18 parser rejections become verification outcomes; the e2e corpus
  unchanged (no module of it uses labels), plus the round-trip fixture
  `MoveToLeanerLang/state_labels.move`. Labels are names (`NameId`s) in
  LIR; a labeled `exists` is `SpecOperation.exists`. A Check fixture waits
  for S2: a Check baseline never records an unsupported proof.
- **S2 — contracts.** Labels bound in `Contract.lean`: quantified labels
  as `Memory unit` binders, defined labels by their defining predicates,
  ranges selecting the states of behavioral predicates, specification
  functions, `global`, and `exists`. Gate: the fixture's contracts state
  the labels; `aborts_if_at_state_label` no longer rejected.
  - S2a, done 2026-10-05: quantified labels over memory. A binder over the
    state domain is a `Memory unit` variable; a range makes its pre-state
    label the state `old` reads and its post-state label the current state,
    for a behavioral predicate's states, a specification function's
    arguments (translated once more under the range), and labeled `global`
    and `exists` reads. `spec_fun_state_domain_behavior` fails as the
    Prover does (intended). The value of a `&mut` parameter at a labeled
    state is not a memory's, so a read of one under a range is refused
    ("not carried yet"): the Prover carries per-label copies of the `&mut`
    parameters, which is S3's "values at labels"; `spec_fun_old_param_labeled`
    and the `*_mismatch` tests (rejected by the Prover's Boogie translator
    over exactly those copies) wait for it.
  - S2b, open: defined labels — `publish`, `remove`, `update` as memory
    functions of the pre-state; a label an `ensures_of` or `result_of`
    defines as the invocation's post-state.
- **S3 — the closer.** Witnesses for existential labels from the recorded
  states, instantiation of universal ones. Gate: `spec_fun_old_param_labeled`,
  `aliasing`, `two_state_labels`, `intermediate_states` verify as the Prover
  does (their deliberate failures excepted).
  - S3a, done 2026-10-05: values at labels. A binder over the state domain
    also binds one copy of every `&mut` parameter, as the Prover keeps per
    label (`value_state_vars`); a range installs a label's memory and copies
    as the old or current state and locals. The contracts of
    `spec_fun_old_param_labeled` now fail exactly the Prover's negatives
    and, pending S3b, its positives too: `∃ S …` over a memory and the
    copies needs a witness.
  - S3b, done 2026-10-05: witnesses. A program point is a hypothesis of the
    leaf (`ProgramPoint value state`, an implementation detail the deciders
    skip): where the closer unfolds a let-bound continuation at a call
    boundary it asserts the `(value, state)` the continuation is applied
    to, and the context's rewrites carry the point to the leaf, which a
    ledger beside the goal would not survive (a substitution renames the
    values). Points are kept for a target whose contract binds a label
    (`Contract.contractBindsStateLabel`, the `labeled` flag of
    `leaner_denote_close`); other targets pay nothing. `leaner_denote_witness`
    tries the points first for a label's existential: the point's state for
    a `Memory` binder, the encodings of its locals in the binder's domain
    (`RuntimeValue`, `Int`, …; a reference by its referent's current value)
    for the copies, the nested existentials of one label jointly, the body
    normalized and decided conjunct by conjunct.
    Where no call separates the states — `inc_both_twice` increments two
    parameters in place — a copy is a plain existential over values, as
    the Prover reads it: a `RuntimeValue` binder read at one integer field
    is reduced to an `Int` binder (a nominal value holding the integer),
    an `Int` binder's first candidates are the bounds its body states
    (`t < k` gives `t + 1`, `k < t` gives `t - 1`, `k = t` gives `t`), the
    conjuncts not reading the binder are decided once before any candidate,
    and a nested existential is witnessed in turn. The leaf pipeline now
    substitutes the variables its hypothesis destructuring equates (a
    resolved prophecy and the value written) and reduces the projections
    of literal pairs before the deciders. Specification types see through
    references (`seeThroughReferences`): a specification function over a
    `&mut Counter` reads `c.value` as an integer, labeled or not.
    `spec_fun_old_param_labeled` verifies as the Prover does (`inc_both` and
    `step_both` are its intended negatives); `spec_fun_generic_labeled`,
    `labeled_state_arg_mismatch`, and `pure_spec_fun_labeled_mut` verify;
    `Check/Specifications/StateLabels.lean` has the splits over two opaque
    calls and over two inlined `&mut` calls. A universal label is
    instantiated where its clause is used. The `*_mismatch` tests, which the
    Prover's Boogie translator rejects over its copies, are read uniformly
    here (an intended difference).
- **S4 — the rest of the directory.** The verification failures left in the
  directory after S3 (`spec_functions`, `spec_fun_mixed_memory`, …) are
  each attributed: a labels gap, or another registry entry.

## Open

- A free label with no defining operation. `nonlinear_cfg_error.move` reads
  `..S |~ Counter[addr].value == …` with `S` undefined and expects the
  Prover to fail the clause "on non-linear control flow", so on linear
  control flow the Prover reads such a label at a program point of the body
  (the states between its calls, as `intermediate_states.move` describes).
  Validation rejects an undefined label; whether to read it at the body's
  call boundaries, and how to number several, is decided with S3.

## Not in scope

- Labels in in-body `spec { … }` blocks (the Prover does not allow them
  either).
- The anchors `@n` of in-body assertions: they stay as they are; whether
  they and labels should share a notion is a question for later.
