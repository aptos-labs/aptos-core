{# Keep only treatment-specific orchestration here. Shared inference guidance
   belongs in spec_inf_rules.md so it renders identically in every arm. #}
{% if once(name="spec_inf_tasks") %}

## Specification inference

Infer a complete, readable specification for the requested function or module.
Preserve executable behavior and user-written specifications.

{% if tactic_selectable %}
### Tactic

Two hybrid tactics are available; the default is
**{{ inference_tactic | replace(from="_", to="-") }}**. An invocation may name
the other as its first word -- `/move-inf hybrid-guided` or
`/move-inf hybrid-flexible` -- and whatever follows is the scope. This
invocation's arguments: `$ARGUMENTS`. Follow the section for the tactic in
effect and no other.
{% endif %}
{% if inference_tactic == "agent_only" %}

### Direct tactic

Derive the contract and any loop invariants directly from the implementation
and relevant dependency contracts. Check the candidate as you go, and use its
diagnostics to refine your work.
{% endif %}
{% if inference_tactic == "hybrid_flexible" or tactic_selectable %}

### Flexible hybrid tactic{% if tactic_selectable %} (`hybrid-flexible`){% endif %}

`{{ tool(name="move_package_wp") }}` is available as an inference pass. Decide
whether and when to use it, and how to combine it with direct reasoning and
invariant synthesis.

It runs on any scope, loops included. Where the loops carry adequate invariants
it derives the complete normal and abort behavior in one call, including the
cast, overflow, and division obligations the source never names. Where they do
not, it warns against the functions concerned and leaves the rest finished: the
warning names the loop and gives bounded loop-head facts to build an invariant
from. Rerun with `filter: "module::function"` to rework one warned function at a
time, repeating on it until its warning is gone.

Once no warnings remain, finish in one pass: simplify as much as the contract
needs, then check the candidate, and stop when it accepts.

What WP derives is exact but written for a machine — the path guard that
reached each obligation is restated in every clause, a widened intermediate is
spelled out wherever it appears, and some clauses are subsumed by others. How
much of that is worth rewriting is your call, and a contract that already reads
well needs nothing. Where you do rewrite, preserve the meaning exactly: no
weakened condition, no dropped obligation, no widened abort. Do not lift a
partial expression such as a division above the guard that makes it defined; a
`let` is evaluated where it is bound, so name it with a spec function instead.
{% endif %}
{% if inference_tactic == "hybrid_guided" or tactic_selectable %}

### Guided hybrid tactic{% if tactic_selectable %} (`hybrid-guided`){% endif %}

Follow this order:

1. **Run WP over the requested scope.** Call
   `{{ tool(name="move_package_wp") }}` with the requested output location,
   whether or not the scope has loops. Every function it does not warn about is
   finished; the warnings list the ones that are not.
2. **Take one warned function.** Its warning names the loop needing an invariant
   and gives bounded loop-head facts for the first iterations. Write an
   invariant that holds at entry and is preserved by one back-edge: bounds, the
   accumulator relation, and any resource or frame fact the contract needs.
3. **Rerun WP for that function alone**, with
   `filter: "module::function"`, after removing its stale WP-generated function
   clauses and keeping the invariants and helpers. An invariant that is too weak
   leaves the warning in place, so repeat from step 2 on the same function until
   it is gone, then take the next warned function.
4. **Simplify what WP derived**, once no warnings remain and before checking.
   Its output is exact but written for a machine: the path guard that reached
   each obligation is restated in every clause, a widened intermediate is
   spelled out wherever it appears, and some clauses are subsumed by others.
   Rewrite the contract to say the same thing as a reader would — name a
   repeated guard or quantity with a `let` or a spec function, drop a clause
   another already covers, and state each condition in the terms the source
   uses. Preserve the meaning exactly: this step may not weaken a condition,
   drop an obligation, or widen an abort. Re-verify after rewriting, and if the
   simplified form does not prove, keep the form that does.
5. **Check the candidate.** On rejection, repeat whichever steps its diagnostic
   implicates: a warned loop returns to step 2, a contract that needs reworking
   to step 1, and anything else is repaired where the diagnostic points. Then
   check again.

{% endif %}

{% include "templates/candidate_check.md" %}

{% include "templates/verification_tasks.md" %}

{% endif %}
