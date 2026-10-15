# Recover overwritten DKG resources after pruning

A validator restarting after one DKG session finishes, while another session still
holds up reconfiguration, may need the public DKG resources from the boundary that
started its current epoch. Normal recovery reads that boundary from local state-KV
history. Increasing pruning retention after those values have been deleted cannot
restore them.

This opt-in recovery path accepts a bounded BCS file containing the two public DKG
resources and cryptographic proofs. It does not restore a database or modify
on-chain state. It does not disable randomness or relax transcript, epoch,
validator-set, key, transaction, or state-root validation.

## Prerequisites

* Use binaries containing this change on the recipient and for the export tool.
* The recipient must already have the authenticated epoch-ending ledger info that
  began the affected epoch in its own database. The donor cannot supply a new trust
  anchor. This feature cannot repair missing/corrupt ledger infos or an unrelated
  database fork.
* A consistent donor database checkpoint on the same chain must retain the exact
  boundary transaction, transaction accumulator proof, state-KV values, **and
  state-Merkle proof history**. Ledger pruning retention alone does not guarantee
  the last requirement. Epoch snapshot retention does not preserve pruned KV
  values. Export fails if any required history is unavailable.
* A snapshot restored after the boundary is insufficient. A suitable earlier
  snapshot replayed to the boundary can be used as a donor if it retains the
  required proof data. This tool does not perform that restoration or replay.

If no such donor exists, this feature cannot reconstruct the lost public data.
Do not substitute unverified REST JSON, validator keys, local shares, or a newer
DKG transcript.

## Export and install

Build the existing debugger and node using the normal project build process:

```sh
cargo build --release -p aptos-debugger -p aptos-node
```

Use an offline, consistent donor **DB checkpoint directory**, not a running
node's database. Pass the recipient's affected epoch, not its dealer epoch:

```sh
aptos-debugger aptos-db debug export-dkg-recovery \
  --db-dir /operator/donor-checkpoint \
  --epoch <affected-epoch> \
  --output /operator/current-epoch-dkg.bcs
```

The command opens the checkpoint read-only with all pruning disabled, verifies
both resource proofs before writing, and refuses to overwrite an existing output
file. The output contains only public on-chain DKG resources and proofs, never
validator credentials or locally derived shares. A partial output after an I/O
failure is invalid and must not be installed.

Transfer that public file to the recipient using the operator's normal file
transfer process. Keep it operator-controlled and readable by the node service.
The maximum supported file size is 64 MiB. Configure an absolute regular-file path:

```yaml
consensus:
  dkg_recovery_bundle_path: /operator/current-epoch-dkg.bcs
```

Retain the node's current keys and safety state. Restart through the normal
validator operating procedure. The bundle is opened only if the current-epoch DKG
resource has been overwritten and the local historical read fails. A successful
local read, including an authenticated resource absence, takes precedence.

The recipient authenticates the exact boundary transaction against its own ledger
info, obtains the state checkpoint root from that authenticated transaction, then
verifies inclusion or non-inclusion of both fixed DKG resource keys. Existing
session/transcript checks run after decoding. Only consensus's key-derivation
snapshot is replaced; the DKG managers continue to see the latest on-chain state.

An invalid, stale, missing, oversized, or unprovable bundle never replaces the
resource. The existing caller error/fallback policy is unchanged: rejected
recovery does **not** mean the validator is able to participate. Inspect recovery
errors and verify sustained synchronization, successful consensus participation,
and absence of execution/hash errors using normal operational monitoring. A
successful process start alone is insufficient.

## Epoch changes and removal

A bundle is bound to one epoch and one exact boundary. It cannot authorize data
for a subsequent epoch or another chain. It is ignored when historical recovery
is unnecessary. Remove the config entry after the affected epoch has been passed
and normal operation is verified. A later epoch requiring recovery needs a new
bundle (or retained local history). There is no automatic download, persistent
cache, or background mutation of consensus configuration.

Rollback removes the optional configuration entry and uses the ordinary supported
binary rollback procedure, subject to database compatibility. Do not restore old
safety state or private keys. This feature writes no historical values to the DB,
so there is no imported database history to undo.
