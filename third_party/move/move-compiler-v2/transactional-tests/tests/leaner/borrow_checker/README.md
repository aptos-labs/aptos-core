# Leaner borrow-checker comparisons

These transactional tests compare three acceptance boundaries for reference
programs authored in Leaner:

1. Leaner's poison-aware source checker, invoked by each `spec` declaration;
2. compiler-v2's stackless-bytecode reference-safety analysis; and
3. the production Move bytecode verifier and VM.

The files are grouped by expected boundary:

- `accepted.lean` is accepted by all three layers and records successful VM
  executions.
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
| `leaner_permissive_loop_carried.lean` | Rejects copying/dropping the owner across the reference-carrying loop | Verifier rejects with `COPYLOC_EXISTS_BORROW_ERROR` |
