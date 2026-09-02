{# Shared specification editing and simplification guidance #}
{% if once(name="spec_editing_ref") %}

{% include "templates/spec_lang.md" %}
{% include "templates/core_tools.md" %}

## Editing and simplifying specifications

Edit the contract rather than executable behavior. In an inference task, a
behavior-preserving loop/inline-HOF rewrite is appropriate only when it is
needed to express a sound invariant.

{% if acceptance_check_enabled %}
Revise a specification with targeted edits to the clauses that change. Rewriting
a whole module to adjust one condition risks disturbing unrelated user-written
code, and its output is dominated by text that was already correct.
{% endif %}

### Simplification order

1. **Repair the abstraction first.** Resolve the loop or callee facts behind a
   `vacuous` or `sathard` clause before simplifying the resulting expression.
2. **Replace unbounded quantifier encodings.** Express frames with `modifies`
   and accumulations with a bounded or recursive helper when equivalent. If a
   quantifier is genuinely required, give it valid uninterpreted-function
   triggers.
3. **Normalize mechanical state expressions.** Replace nested `update_field`
   terms with direct struct construction when all fields are known. Consolidate
   unrolled cases into an equivalent general condition when justified.
4. **Simplify arithmetic and boolean structure.** Remove only clauses implied by
   retained clauses or language guarantees. Flatten repeated updates and factor
   repeated subexpressions without changing overflow or abort behavior.
5. **Verify the replacement.** Keep `[inferred]` on inferred replacements and
   rerun the prover after each meaningful simplification.

Every simplification must preserve result, abort, precondition, and frame
semantics.

### Pragmas and trust boundaries

{% if evaluation_mode %}
- Never disable or skip verification. `pragma verify = false`,
  `verify_duration_estimate`, partial abort coverage, axioms, and unproved
  assumptions cannot count as a repaired proof.
{% else %}
- Do not add `pragma verify = false`, `verify_duration_estimate`, an axiom, or an
  unproved assumption as an automatic fallback. Use one only when the user or an
  explicit project policy accepts that trusted boundary. Add an adjacent comment
  recording why it is trusted and any timeout/proof evidence observed.
{% endif %}

{% endif %}
