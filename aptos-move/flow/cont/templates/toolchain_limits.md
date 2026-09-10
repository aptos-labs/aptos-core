{# Toolchain capability reference — shared verbatim by every inference tactic #}
{% if once(name="toolchain_limits") %}

### Toolchain capabilities and limits

Establish these from the reference rather than by probing the compiler with
trial declarations.

- **Pragmas are context-specific.** An invalid `pragma` is a compile error whose
  diagnostic lists every pragma valid at that position. Read that list instead
  of guessing further names.
- **Induction is a recursive lemma.** A lemma may `apply` itself (or a
  lemma of the same group) in its proof at a smaller instance:
  `lemma pow_pos(b: num, e: num) { ensures ... } proof { if (e > 0) { apply pow_pos(b, e - 1); } }`.
  Every such application must decrease the lemma's measure, by default the
  tuple of its integer parameters in declaration order, or as declared with
  `decreases e;` / `decreases (e1, e2);` (lexicographic); the prover reports
  "does not decrease the measure" at the application otherwise. `forall ...
  apply` of a lemma from its own group is rejected. Outside lemmas there is
  still no induction: a quantified fact about `n` repetitions has to be
  arranged so the solver only ever needs one step of it.
- **Align a recursive spec helper with the loop.** When a closed form such as
  `base * 2^n` is unprovable, define a helper whose recursion performs exactly
  one iteration of the loop and state the invariant in terms of it. Each proof
  obligation then unfolds definitionally: the invariant is preserved by one
  helper step, and the loop-exit condition makes the postcondition immediate.
  Saturating the helper at the overflow bound lets the same definition express
  exact abort behavior.
- **Keep helper recursion single.** Two mutually reinforcing recursive helpers
  in one contract multiply quantifier instantiations and commonly time out
  where one recursion-aligned helper proves quickly.

{% endif %}
