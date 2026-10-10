# Lemmas and proof blocks

Status: decided 2026-10-03 (user): Move `spec lemma` declarations and
`proof { … }` blocks are translated into LeanerLang, not replaced by proof
files per test. L1–L4 implemented 2026-10-03 ([Milestones](#milestones)).

## Why

The Move Prover (language 2.4) verifies lemmas and lets a specification
carry a proof block that guides the solver. 33 of its unit tests use them.
The exchange leaves lemmas out and drops proof blocks, so under Leaner a
function whose proof needs a lemma fails (`specialize_generic_caller`'s
`count_all`), and the intended failures of the proof tests do not occur: a
false lemma, an unmet lemma requirement, or a false proof assertion verifies
without a message (`proof/lemma`, `proof/assert`, `proof/calc`).

## Move semantics

The Prover's translation (`move-model/src/spec_translator.rs`,
`translate_proof`) defines the meaning:

- **Lemma** `lemma L<T>(params) { requires R; ensures E; decreases m }
  proof { P }`: verified as a function without a body: assume `R`, run `P`,
  assert `E`. A lemma reads its parameters' types as type assumptions.
- **Function proof**: the statements of `P` run at the function's entry,
  after its preconditions; `post S` runs `S` at each return, before the
  postconditions. `old` occurs only under `post`.
- **Statements.** `assert e` checks `e`, then assumes it. `assume [trusted]
  e` assumes `e` (with a warning). `apply L(a)` checks `R[a]`, then assumes
  `E[a]`. `forall xs {triggers} [weight = n] apply L(a)` assumes
  `∀ xs. R[a] ⇒ E[a]`. `calc(e1 op e2 op …)` asserts each step. `let x = e`
  binds `e` where it stands. `if c P else Q` guards each action of `P` with
  `c` and of `Q` with `¬c`, evaluated where the action runs. `split e`
  verifies once per value of a Boolean, or per variant of an enum.
- **Recursion.** An `apply` of a lemma of the same recursion group must
  decrease the measure (`decreases`, or the integer parameters in order),
  lexicographically, with the decreasing component non-negative before the
  step: `n0 < c0 ∧ 0 ≤ c0`, or `n0 = c0` and the rest decreases. A
  `forall … apply` of the same group is rejected.

## Representation

**Lemmas in LIR.** A namespace holds `lemmas : Array LemmaDecl`: name,
generics, parameters (a signature), locals, a contract of `requires`,
`ensures`, and `decreases` conditions, and a proof: the guarded steps
below, as one `SpecBlock`. Recursion groups are derived (strongly connected
components of the lemmas the steps apply), not carried.

**Steps.** A proof is a sequence of conditions, as the Prover flattens it:
`if` becomes an implication from the path condition, `let` is substituted
where it is used (by `old(e)` in a `post` statement when bound outside
one), and `calc` becomes one assertion per step. Kinds:

- `assertion`, `assumption`: as in-body statements today.
- `apply` (new): its expression is a formula over lemma instances,
  `φ ::= L(a) | c ==> φ | ∀ xs. ψ` with `ψ ::= L(a) | c ==> ψ | ∀ xs. ψ`.
  The instance `L(a)` is the new operation `SpecOperation.lemma L range`
  (as `functionCall`), valid only in an `apply` condition. `apply L(a)`
  owes `R[a]` and gives `E[a]`; under `c ==>` both are guarded by `c`;
  `apply ∀ xs. ψ` owes nothing and gives `∀ xs` of the instances' implications
  `R[a] → E[a]`. Triggers and weights are solver hints and are not carried.
- `split` (new): a case split, on a Boolean or an enum's variant. The
  Prover's guard on a split only limits where it splits; the split is
  semantically no step, so Leaner splits unguarded.

**Function proofs** need no new LIR structure: the Move frontend places the
steps as in-body specification statements, at the start of the body and
before each return, where the Prover places them. Before a return, the
returned values are bound to locals that `result` denotes.

**LeanerLang.** An item `spec lemma L<T>(params) where` with `requires`,
`ensures`, and `decreases` clauses, then `proof` and the steps, one per
line: `assert e`, `assume e`, `apply φ`, `split e`. In a body, `spec apply
φ` and `spec split e` join `spec assert` and `spec assume`.

## Verification

**A lemma** becomes two definitions over the bundle of its parameters, as a recursive
specification function's (`L.requires`, `L.ensures`, taking what the
conditions read besides the parameters), and a theorem

```lean
theorem L.lemma (p₁ : D₁) … (pₙ : Dₙ) (h : L.requires (p₁, …, pₙ, ())) :
    L.ensures (p₁, …, pₙ, ())
```

`L.requires` states the integer parameters' bounds by their types first:
an `apply` with an argument out of range owes them, where the Prover would
assume what it never proved. The proof introduces the hypothesis and runs
the steps as cuts, each owed obligation discharged by the closer on a goal
without a body; then the closer proves `L.ensures`. A recursion group is
one `mutual` block whose theorems recurse by well-founded recursion on the
measure (`termination_by`, each component as the natural number
`(m + 1).toNat`, which descends exactly when the Prover's condition holds).
Each recursive `apply` first proves the Prover's decrease condition as a
cut, which `decreasing_by` takes, so a failing decrease is reported at the
`apply`.

A step's proposition is a definition over the bundle too (`L.step_i`), and
so is each measure component (`L.measure_i`), so the generated proof names
them as terms.

**An instance in a formula** translates to a marker over the two
definitions at the instance's bundle: `LemmaApplication (L.requires b)
(L.ensures b)`, which is the premise, outside quantifiers, and
`LemmaInstance (L.requires b) (L.ensures b)`, the implication, under one.
Where a formula is owed (at an `apply` site, or a lemma's step), the closer
introduces the premises and binders, reduces each `LemmaApplication` to its
premise, an obligation reported as the lemma's requirement, and proves each
`LemmaInstance` by the lemma theorem. Where it then holds, each
`LemmaApplication` becomes the conclusion by the lemma theorem. The lemma
theorem is named by elaborating its identifier, which inside its own
recursion group is the recursive reference.

**An `apply` site in a function** is an assertion site: the compiler gives
`apply` and `split` conditions a site as it gives assertions one (no step).
What follows assumes the facts. A `split` site on a Boolean owes `e ∨ ¬e`
and what follows splits on it; on an enum it is no step, since the closer
splits a match on the value where the body or a clause has one.

**Trust.** An `assume` in a function's proof is an in-body assumption: the
theorem takes that it holds (`AssumptionsHold`,
[`denotation.md`](denotation.md), "Assumption ledger"). An `assume` at step
`k` of a lemma's proof is the definition `L.lemmaTrusted_k`: of every bundle
meeting the premise and the assumptions before it, the step holds. The
lemma's theorem takes it as a hypothesis, and so does every theorem using
the lemma — another lemma's, a function's (`lemmaTrustDependencies`), and a
caller's that uses that function's theorem — as a caller takes a callee's
assumptions. `assume [trusted] true`, the Prover tests' marker of a lemma
taken on trust, assumes nothing and is no step.

**A lemma not established** has no theorem to use: Lean declares a
theorem whose proof fails over `sorry`, so only the theorems verified
without an error are marked established. An application of it gives
only its premise, and an instance of it under a quantifier is owed as its
implication, which the closer may still prove. The functions applying it
are verified with what remains, and the lemma's failure is reported once,
at the lemma.

**Messages.** An `apply` whose requirement fails reports at the `apply`,
naming the lemma; a lemma whose `ensures` fails reports at the clause; a
recursive `apply` that does not decrease reports at the `apply`.

## Milestones

| | Scope | Gate |
|---|---|---|
| L1 | Exchange: lemmas and proof blocks in XAST (version 7), as the model holds them; decoding. | The proof tests export. |
| L2 | LIR `LemmaDecl`, `apply` and `split` conditions, `SpecOperation.lemma`; RawUnit schema, validation (instances only in `apply` positions, references resolve, arities); LeanerLang items and statements; printer. | The Move corpus round trip stays a fixed point. |
| L3 | Move frontend: lemmas, function proofs placed at entry and before returns, `let` substitution, `calc` steps. | The proof tests print. |
| L4 | Verification: lemma theorems, recursion groups, `apply` and `split` sites, the trust ledger, messages. | Check fixture `Specifications/Lemmas`. |
| L5 | Prover tests `proof/*`, `folds_of*`, `specialize_generic_caller`; registry. | Baselines read. |
