# Pipeline

`PipelineBuilder` constructs a graph of spawned, shared futures for each admitted block.
Execution, commit-vote signing, pre-commit, ledger persistence and notifications are
owned by that graph. A stage awaits its local inputs and the required parent-stage
result; it does not advance through a second buffer state machine.

## Execution and commit

The execution path is materialize → decrypt → prepare → randomness check → execute
→ ledger update. Payload fetching starts before a QC; a QC supplies voter information
for fetching. Execution and ledger update preserve their parent dependencies.

Commit-vote signing waits for the local execution result and any of the existing
authorization signals: local order-vote publication, an order proof, or a commit proof.
`execution_ledger_info` builds the ledger info used by both local signing and certificate
matching, including reconfiguration suffix timestamps.

Pre-commit waits for the local result, parent pre-commit and ordering evidence. An epoch
ending block, or paused/disabled optimistic pre-commit, additionally waits for a commit
proof. Ledger commit waits for pre-commit, parent commit and a covering proof. Only the
proof target writes the final ledger info; its ancestors finish as committed prefix.

Observer publication, mempool feedback, state-sync notifications and block-tree/payload
notifications remain graph branches with their existing dependencies.

## Auxiliary certificate and prefix state

`CommitPipeline` owns an inbox for early commit votes/decisions, certificate accumulators,
an ordered-prefix index, block retention and reliable-broadcast handles. It observes
pipeline execution and local-vote futures without scheduling execution or signing.

`CommitCertificate` accepts early votes by author, then admits only signatures over the
exact expected ledger info once execution completes. Certificates use the existing
validator voting-power and signature verification routines. A remote decision can
complete certification before local signing finishes.

A checked certificate for a later block is delivered to every outstanding block in its
ordered prefix. Ancestors do not require individual certificates. Ordered intake remains
unrestricted by a fixed pending-round count so recovery can reach a cached proof's target.
The inbox for unknown-block votes and proofs remains bounded.

## Lifecycle

Proof-delivered blocks finish persistence before reset cancels an uncommitted suffix.
Reset drains executor/storage futures and queued blocks, drops observation futures and
broadcast guards, and acknowledges only after pending commit publication completes.
Verification completions carry a local reset generation to reject stale results.

Ordered batches fan out directly to the certificate service and enabled share services.
Share services retain only weak per-block subscriptions and retry handles; they resolve
randomness/key inputs independently, without ordered-prefix queues or a readiness
coordinator. Parent futures enforce execution order. Disabled services resolve explicit
absent inputs during ordering. Share aggregation, metadata checks and durable self-share
recovery remain external. Reset rotates result channels so old aggregation tasks cannot
resolve new subscriptions at reused rounds.

## Validation

Run `cargo test -p aptos-consensus pipeline:: --lib` for certificate matching,
deduplication, covering-proof recovery and reset tests. The recovery test deliberately
uses a round-22 proof without local votes or individual ancestor certificates.
