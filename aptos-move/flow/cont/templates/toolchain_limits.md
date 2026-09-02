{# Toolchain capability reference — shared verbatim by every inference tactic #}
{% if once(name="toolchain_limits") %}

### Toolchain capabilities and limits

Establish these from the reference rather than by probing the compiler with
trial declarations.

- **Pragmas are context-specific.** An invalid `pragma` is a compile error whose
  diagnostic lists every pragma valid at that position. Read that list instead
  of guessing further names.
- **There is no induction tactic.** No pragma, lemma, or annotation makes the
  solver prove a property by induction over a symbolic bound. A quantified fact
  about `n` repetitions has to be arranged so the solver only ever needs one
  step of it.
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
