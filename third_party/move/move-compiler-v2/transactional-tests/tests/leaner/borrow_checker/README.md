# Leaner borrow-checker comparisons

These transactional tests compare three acceptance boundaries for reference
programs authored in Leaner:

1. LeanerLang's LIR borrow analysis, whose diagnostics (`LIR-SEMANTIC-BORROW-*`)
   are reported at their source location when the module is emitted as XIR;
2. compiler-v2's stackless-bytecode reference-safety analysis; and
3. the production Move bytecode verifier and VM.

The files are grouped by expected boundary:

- positive files (without a `reject_` or `leaner_permissive_` prefix) are
  accepted by all three layers and record successful VM executions, or an
  intentional VM abort followed by a state check.
- `leaner_permissive_*.lean` is accepted by Leaner's borrow analysis but is
  expected to be rejected by a stricter downstream checker.  The expected
  output records exactly which downstream layer rejects it.
- `reject_*.lean` is rejected by Leaner's borrow analysis and records the
  exact source-positioned borrow error. An overlapping loan is reported where
  the conflicting loan is created or the conflicting access happens, not where
  a poisoned handle is later used.

The Rust VM's `runtime_ref_checks` suite is a dynamic policy reference, not a
test of Leaner.  Cases ported here use ordinary retained Leaner source so the
same input passes through the full XIR, compiler-v2, bytecode-verifier, and VM
pipeline whenever the preceding layer accepts it.

A Lean transactional source is elaborated as a whole.  Consequently a file
whose publish step is rejected cannot also contain an executable module.
Execution successes and downstream compiler errors therefore appear in
separate `.exp` files in this directory.

The dedicated `leaner` configuration retains compiler-v2 reference-safety
diagnostics. A `no-reference-safety` comparison configuration suppresses only
that compiler diagnostic for `leaner_permissive_*`, allowing the same XIR to
continue to bytecode generation and the production verifier. Their distinct
`.leaner.exp` and `.no-reference-safety.exp` baselines record the two outcomes;
the latter is not an acceptance mode used outside these comparison tests.

Current differences are:

| Source | compiler-v2 reference checker | Production verifier / VM with compiler check suppressed |
| --- | --- | --- |
| `leaner_permissive_unused_handle.lean` | Rejects transfer while another mutable borrow is live | Verifier accepts after optimization removes the unused handle; VM returns `5` |

The unused handle is a deliberate difference.

## Coverage map

| Policy surface | Positive transaction | Negative transaction |
| --- | --- | --- |
| mutable activation and use | `accepted`, `repeated_writes` | `reject_poisoned_use`, `reject_poisoned_write`, `reject_poisoned_return` |
| reborrowing and lineage | `accepted` (`child_then_parent`) | `reject_parent_while_child`, `reject_poisoned_reborrow`, `reject_nested_poison` |
| immutable references | `accepted` (`multiple_immutable`) | `reject_immutable_activation`, `reject_immutable_after_mutation` |
| freezing | `freeze`, `vectors` | `reject_poisoned_freeze` |
| call arguments and separation | `calls` | `reject_call_separation`, `reject_poisoned_call`, `reject_read_only_call` |
| returned-reference derivations | `returns` | `reject_local_return`, `reject_global_return` |
| branches and loops | `loop_carried` | `reject_branch_poison`, `reject_loop_poison` |
| globals and abort rollback | `globals` | `reject_global_owner_invalidation` |
| vector element aliases and structural mutation | `vectors` | `reject_vector_alias`, `reject_vector_mutation`, `reject_vector_swap`, `reject_vector_pop` |
| direct and mutual recursion over references | `recursion` | |
