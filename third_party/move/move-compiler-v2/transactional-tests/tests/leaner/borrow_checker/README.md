# Leaner borrow-checker comparisons

These transactional tests compare three acceptance boundaries for reference
programs authored in Leaner:

1. Leaner's poison-aware source checker, invoked by each `spec` declaration;
2. compiler-v2's stackless-bytecode reference-safety analysis; and
3. the production Move bytecode verifier and VM.

The files are grouped by expected boundary:

- positive files (without a `reject_` or `leaner_permissive_` prefix) are
  accepted by all three layers and record successful VM executions, or an
  intentional VM abort followed by a state check. `loop_carried.lean` also
  checks that distinct mutable source bindings survive Lean normalization as
  distinct XIR locals.
- `leaner_permissive_*.lean` is accepted by Leaner's source checker but is
  expected to be rejected by a stricter downstream checker.  The expected
  output records exactly which downstream layer rejects it.
- `reject_*.lean` is rejected by Leaner during source elaboration and records
  the exact source-positioned borrow error.

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

Current deliberate differences are:

| Source | compiler-v2 reference checker | Production verifier / VM with compiler check suppressed |
| --- | --- | --- |
| `leaner_permissive_unused_handle.lean` | Rejects transfer while another mutable borrow is live | Verifier accepts after optimization removes the unused handle; VM returns `5` |
| `leaner_permissive_read_only_call.lean` | Rejects transfer of the overlapping mutable argument | Verifier rejects with `CALL_BORROWED_MUTABLE_REFERENCE_ERROR` |

## Coverage map

| Policy surface | Positive transaction | Negative transaction |
| --- | --- | --- |
| mutable activation and use | `accepted`, `repeated_writes` | `reject_poisoned_use`, `reject_poisoned_write`, `reject_poisoned_return` |
| reborrowing and lineage | `accepted` (`child_then_parent`) | `reject_parent_while_child`, `reject_poisoned_reborrow`, `reject_nested_poison` |
| immutable references | `accepted` (`multiple_immutable`) | `reject_immutable_activation`, `reject_immutable_after_mutation` |
| freezing | `freeze`, `vectors` | `reject_poisoned_freeze` |
| call summaries and separation | `calls`, `leaner_permissive_read_only_call` | `reject_call_separation`, `reject_poisoned_call` |
| returned-reference derivations | `returns` | `reject_local_return`, `reject_global_return` |
| branches and loops | `loop_carried` | `reject_branch_poison`, `reject_loop_poison` |
| globals and abort rollback | `globals` | `reject_global_owner_invalidation` |
| vector alias abstraction and structural mutation | `vectors` | `reject_vector_alias`, `reject_vector_mutation`, `reject_vector_pop`, `reject_vector_swap` |
| direct and mutual recursive summaries | `recursion` | bounded post-fixpoint failure is exercised at the policy level because valid source SCCs have a finite summary lattice |
