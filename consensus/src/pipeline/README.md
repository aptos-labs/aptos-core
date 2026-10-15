# Consensus execution pipeline

Round-based consensus represents block progress with `PipelineFutures`, created
when a block is inserted into the existing `BlockTree`. Futures express both
within-block and parent-block dependencies. The tree is the sole owner/index of
known blocks; there is no separate commit buffer, ordered-prefix queue, pending
persistence map, or completion-event scheduler on this path.

## Commit graph

`PipelineFutures::commit_pipeline` contains the consensus commit stages:

- `commit_proof_fut` resolves from this block's certificate or a descendant's
  certificate, supplying the existing storage graph.
- `certificate_fut` waits for the block's ordering proof and ledger update,
  records execution feedback for backpressure as soon as ordered execution
  completes, then aggregates verified votes or accepts a remote certificate. It does not
  depend on our local vote, pre-commit, or persistence.
- `broadcast_fut` waits for `commit_vote_fut` and ordering as the batch target, owns reliable broadcast and
  retries, and stops when a commit proof arrives for this block or a descendant.
- `certify_ancestors_fut` waits for `certificate_fut`, obtains the uncommitted
  ancestor path from the same block tree, ensures execution results are recorded, and resolves
  each ancestor's commit-proof input. Only the target of an ordered batch
  aggregates its certificate; intermediate nodes consume the descendant proof.
- `epoch_change_fut` waits for durable commit and post-commit notifications,
  drains the remaining suffix at an epoch boundary, then publishes the epoch
  change proof.

The existing storage graph remains authoritative: `pre_commit_fut` depends on
ledger update, ordering and parent pre-commit; `commit_ledger_fut` depends on its
commit proof, pre-commit and parent ledger commit. Ancestors certified by a later
block finish their storage dependencies; only the exact certificate target
persists the final ledger info. State-sync notification, payload notification,
and the tree's commit/pruning callback retain their existing dependencies.

## Input boundary and lifecycle

`CommitPipelineRuntime` supplies verified network inputs to bounded per-block
mailboxes through tree lookup. Before insertion, a bounded inbox retains one
vote per author per round and commit proofs within the configured round window.
It does not retain known blocks or decide their next stage. A weak reference to
the tree avoids an ownership cycle through the futures.

The execution client resolves ordering inputs. The existing randomness and
secret-sharing services continue to supply their results in this checkpoint;
their readiness output resolves the block's randomness/key inputs directly.
It is not a commit-registration channel. Later checkpoints can remove those
services' readiness queues without changing the commit graph.

Reset closes input admission and invalidates in-flight verification results.
It snapshots blocks from the tree, lets proof-delivered storage and notifications
finish, cancels *all* uncertified nodes, and only then waits for their
non-abortable executor/storage futures. This ordering prevents a cancelled child
from waiting forever on a parent whose cancellation has not yet been issued.
The first epoch-ending certificate seals proof admission atomically with prefix
delivery, preventing concurrent descendants from extending the certified suffix.
A state-sync reset clears old inputs before resuming; epoch shutdown keeps
admission closed. Tree pruning follows durable completion through the existing
post-commit callback and preserves the execution window.

## Legacy consumers

DAG and consensus observers continue using `BufferManager` and the phase
adapters. They do not use `CommitPipelineRuntime` or its certificate graph.
Their migration is outside this change. Legacy tests remain in place alongside
consensus future-graph tests and the multi-validator epoch-reset smoke test.
