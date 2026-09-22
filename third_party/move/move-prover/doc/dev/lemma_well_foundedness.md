# Well-Founded Recursive Lemma Application

Tracking: #20275.

## Problem

A lemma proof may `apply` a lemma, including itself. `expand_lemma_apply`
(move-model `spec_translator.rs`) expands every application the same way:
`assert requires(args); assume ensures(args)`. Inside `L`'s own proof this
assumes `L`'s conclusion while proving it, and nothing relates `args` to `L`'s
parameters. Guarded recursion therefore gives real induction, but circular
recursion is admitted silently:

```move
lemma probe_same(base: num, exp: num) {
    ensures base >= 2 && exp >= 0 ==> math_pow(base, exp) <= 1;   // false
} proof { apply probe_same(base, exp); }                            // verifies

lemma probe_ascending(base: num, exp: num) {
    ensures base >= 2 && exp >= 0 ==> math_pow(base, exp) <= 1;   // false
} proof { apply probe_ascending(base, exp + 1); }                   // verifies
```

Both verify today; a control lemma with the same claim and no proof is
rejected, so the checker works and only the recursion is unguarded. Once
accepted, a false lemma can be applied anywhere.

The legitimate use is common and worth keeping:

```move
lemma math_pow_pos(base: num, exp: num) {
    ensures base >= 1 && exp >= 0 ==> math_pow(base, exp) >= 1;
} proof {
    if (exp > 0) { apply math_pow_pos(base, exp - 1); }
}
```

## Design

Every recursive lemma has a termination measure, declared or defaulted, and
every application of a lemma from the same recursion group is checked to
decrease it. This is Dafny's treatment restricted to the proof language's
existing statements.

### Surface syntax

```move
lemma L(params) {
    decreases e;              // measure: one integer expression, or
    decreases (e1, ..., en);  // a tuple, ordered lexicographically
    requires ...;
    ensures ...;
} proof { ... }
```

`decreases` is an existing `ConditionKind`; today `def_ana_spec_block_member`
rejects it for every context ("condition kind is not supported"). It becomes
accepted in a lemma's condition block and stays rejected elsewhere. The parser
already takes one expression after `decreases`, so a multi-component measure
is written as a tuple expression; each component must have an integer type.
At most one `decreases` per lemma.

### Default measure

A recursive lemma without `decreases` gets the Dafny default: the tuple of its
integer-typed parameters (`num`, `u8`..`u256`) in declaration order. For

```move
lemma math_pow_pos(base: num, exp: num) { ... }
proof { if (exp > 0) { apply math_pow_pos(base, exp - 1); } }
```

the default is `(base, exp)`, and the application decreases it: `base` is
unchanged and `exp - 1 < exp` with `0 <= exp` from the guard. The common
pattern therefore needs no annotation. A lemma that recurses on a parameter
other than the first varying one (say `L(n, i)` recursing on `i` while `n`
changes) fails the default check; the diagnostic names the defaulted measure
so the author knows to write `decreases i`. A lemma with no integer parameter
has an empty default and must declare one.

Within a group, members without `decreases` use their defaults; the arity rule
below applies to the resulting tuples, so mixing defaulted and declared
measures is fine as long as the arities agree.

### Recursion groups

Build the lemma call graph of a module: an edge `L -> L'` for every
`Proof::Apply` or `Proof::ForallApply` of `L'` in `L`'s proof, including
inside `IfElse`, `Block`, `Post`, and `Split`. Its strongly connected
components are the recursion groups. A lemma is *recursive* when its group has
more than one member or a self-edge. Lemmas of other modules cannot be in the
same group: proofs can only apply lemmas that are already declared, and modules
form a DAG.

### Obligations

Let `L` have measure `m(params) = (e1, ..., en)` and let `apply L'(args)`
occur in `L`'s proof with `L'` in `L`'s group and measure `m'`. Groups are
checked with one measure shape: every member of a group must declare a
`decreases` of the same arity (the usual mutual-recursion convention). At the
application site, before the existing requires assertion, emit

```
assert path_cond ==> m'(args) <_lex m(params)
```

where `<_lex` is the lexicographic order over `num` made well-founded by
bounding the component that decreases, unfolded at the site as

```
(e1' < e1 && 0 <= e1) || (e1' == e1 && (e2' < e2 && 0 <= e2)) || ...
```

with `ei` the enclosing lemma's components and `ei'` the applied lemma's
components at `args`. Each strict step happens on a component that is at least
0 before the step, so no component can descend forever, and the earlier
components are equal whenever a later one is the one decreasing.

Errors:

- Recursive lemma whose measure is empty (no `decreases` and no integer
  parameter): model error, "recursive lemma `L` needs a `decreases` clause".
- `forall ... apply L'(args)` of a group member inside `L`'s proof: model
  error. The quantified form instantiates the lemma at every binding; there is
  no single decreasing instance to check, and unrolling it under a measure is
  exactly the unbounded recursion the check exists to exclude. Quantified
  application of lemmas outside the group is unaffected.
- `decreases` on a non-recursive lemma: accepted and unused (a warning is
  optional). Arity mismatch within a group: model error.

The obligation is a `ProofAction::Assert` like the requires check, so the
Boogie backend and the bytecode pipeline are unchanged and the diagnostic
appears at the `apply` site: "lemma application does not decrease the measure
`(base, exp)`", printing the declared or defaulted tuple.

### What the translator needs

`expand_lemma_apply` currently sees only the applied lemma. It gets the
enclosing lemma from `self.fun_env`: a lemma proof is verified as a
`FunctionKind::Lemma` function, and `find_lemma_by_name` already maps that
function back to its `LemmaDecl` (used by `sync_lemma_from_spec`). The
recursion groups are computed once per module after all lemma proofs are
analyzed (`def_ana_lemma` runs with every lemma name registered, so mutual
recursion is already representable) and stored on the module data.

## Implementation plan

1. `parse_lemma_spec_member` (legacy parser): accept `decreases` in a lemma
   block. `module_builder.rs`: accept `ConditionKind::Decreases` when the spec
   block context is a lemma; keep the rejection for functions. Reject more
   than one.
2. `module_builder.rs` (end of module analysis): collect lemma applications
   from each proof, compute SCCs (`petgraph` is a workspace dependency; move-model would inherit it), check
   group-wide arity after filling in default measures, store `lemma_groups`
   and the effective measure per lemma on the module.
3. `spec_translator.rs`: in `expand_lemma_apply`, look up the enclosing lemma
   via `fun_env`; if the applied lemma shares its group, emit the decrease
   assertion. In `expand_forall_lemma_apply`, error on an in-group target.
4. `sourcifier.rs`: print `decreases` on lemmas (round-trip).
5. Tests in `move-prover/tests/sources/functional/`, each with an `.exp`
   baseline: `lemma_induction.move` (`math_pow_pos` with `decreases exp`,
   verifies; and again without the clause, verifies on the default),
   `lemma_circular.move` (`probe_same`, `probe_ascending`, each line marked
   `// error:`), `lemma_mutual.move` (two lemmas, `decreases n` both,
   verifies; a variant with mismatched arity errors),
   `lemma_default_wrong_param.move` (recursion on the second parameter while
   the first changes: fails on the default, passes with `decreases i`),
   `lemma_no_measure.move` (recursive, no integer parameter: model error),
   `lemma_forall_self.move` (model error).
6. Docs: the lemma section of the user guide, and the `move-inf` skill text,
   which currently states that no lemma can prove a property by induction.

Roughly 250 lines of Rust plus baselines; no backend change.

## Alternatives considered

- **Explicit `decreases` only.** Simpler to explain, but it taxes the common
  `exp - 1` pattern with an annotation that says nothing the signature does
  not. The default covers that pattern; its only cost is a diagnostic that has
  to print the defaulted tuple, which it does.
- **Syntactic guard check** (require the recursive `apply` to sit under
  `if (p > 0)` with `p - k` as the argument). Covers `math_pow_pos` and
  nothing else; structural recursion over vectors, mutual recursion, and
  lexicographic descent all need the semantic check anyway.
- **Forbid recursion.** Loses the only induction mechanism the prover has;
  run 4 of the pow experiment needed it for the closed-form abort condition.
