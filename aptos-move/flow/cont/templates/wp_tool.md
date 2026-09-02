{# WP tool reference #}
{% if once(name="wp_tool") %}

### WP tool

`{{ tool(name="move_package_wp") }}` derives function conditions and writes
them to source. Give it `package_path` and optionally a `filter`:

- omitted infers every function in the package;
- `filter: "module"` restricts it to one module;
- `filter: "module::function"` restricts it to a single function. Use this to
  rework one function without touching the others.

Choose `spec_output` from the requested output location:

- `inline` (default) writes function contracts into source files;
- `file` writes `.spec.move` files and leaves ordinary function contracts out
  of the source. Loop invariants still belong at their source loops.

Weakest preconditions are exact in the absence of loops, so a scope with no
loop, or whose loops already carry adequate invariants, yields a complete
contract in one call — including the cast, overflow, and division obligations
the source never names.

Run it on any scope, loops included. The call succeeds and writes what it could
derive; a loop whose invariant does not constrain its modified state is reported
as a warning against that function, and the condition it could not constrain is
not emitted. So a warning means that function's contract is incomplete, while
every function it did not name is finished.

A warning names the loop and its source location, lists the loop-carried state,
and gives bounded loop-head facts for the first few iterations. Those facts are
the raw material for the invariant: look for a predicate that holds at `head[0]`
and is preserved across one back-edge. They are observations, not a proof, and
hold only within the displayed bound. Where several loops in one function are
unrolled, facts at a loop reached through an earlier one hold for that bounded
prefix only, and say so.

A second warning reports that a function's aborts could not be characterized
exactly, so its `aborts_if` clauses are a lower bound and the contract carries
`aborts_if_is_partial`. It names the reasons; the two common ones are an abort
that did not survive an uninvariant loop and an abort that rides on an
unspecified callee. Both are the same repair as above — supply the invariant or
the callee contract and rerun. Complete the abort behavior before removing the
pragma: deleting it alone turns an incomplete contract into a false claim of
exactness, and the candidate check rejects either form.

A scope can also contain a helper with no specification of its own. Where the
derived contract has to speak about that helper through a behavioral predicate
(`result_of`, `aborts_of`), the prover rejects it and names the helper: a
predicate over a function that publishes no contract has no sound meaning.
Infer the helper first, then re-run for its caller — callee before caller, the
same order the rest of a dependency closure needs. A recursive or mutually
recursive helper needs no special handling: its derived contract states the
result in terms of itself or its sibling, which is exact.

Work one warned function at a time: add its invariants, rerun with
`filter: "module::function"`, and read the result. An invariant that is too weak
leaves the warning in place, so repeat on that function until it is gone before
moving to the next. Remove the stale WP-generated **function clauses** before
each rerun, keeping the invariants and spec helpers. Preserve all user-written
clauses.

{% endif %}
