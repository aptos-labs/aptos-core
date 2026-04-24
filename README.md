# TrX: Artifact for CCS 2026

This is the artifact for the paper "TrX: Encrypted Mempools in High
Performance BFT Protocols." It is a fork of the open-source blockchain
Aptos, which includes an implementation of a new batch threshold encryption
scheme and integration of this scheme into the consensus protocol.

## Batch encryption scheme

Located in `crates/batch-encryption`. To run benchmarks:

```
cd crates/batch-encryption
cargo bench
```

The benchmarks in the paper were obtained using a Google Cloud VM of type
`c3d-standard-60`, using the script `crates/batch-encryption/run_benchmarks.sh`.


## Consensus integration

The integration of the batch encryption scheme into the Aptos consensus
protocol is spread across the `types/`, `consensus/consensus-types/`, and
`consensus/` crates. The relevant files are:

### Shared types

- **`types/src/decryption.rs`** — Wraps the `batch-encryption` crate's
  `TRXSuccinct` scheme into the high-level types used throughout consensus
  (`EncryptionKey`, `Ciphertext`, `DigestKey`, `EvalProofs`,
  `DecryptionKey`, etc.). Defines `DecConfig` (per-epoch encryption setup),
  `DecMetadata` (block-identifying info attached to shares),
  `DecShare`/`FastDecShare` (a validator's decryption-key-share message,
  with verification and threshold aggregation), and `DecKey` (the
  reconstructed per-block decryption key). Also defines the prototype
  parameters (`PROTOTYPE_BATCH_SIZE`, `PROTOTYPE_THRESHOLD_FAST_PATH`,
  etc.) and the `DECRYPTION_POOL` Rayon thread pool used for parallel
  ciphertext preparation and decryption.
- **`types/src/transaction/mod.rs`** — Adds an encrypted-transaction
  variant carrying a `Ciphertext`, plus the encrypt/verify helpers used by
  clients and validators.

### Pipeline types

- **`consensus/consensus-types/src/payload.rs`** — Block payload
  variants for encrypted batches; uses `DECRYPTION_POOL` to drive parallel
  decryption of a payload's ciphertexts.
- **`consensus/consensus-types/src/pipelined_block.rs`** — Defines the
  per-stage future types (`DigestResult`, `EvalProofsResult`,
  `DecryptionShareResult`, `DecryptionResult`) that thread the encryption
  protocol through the block-processing pipeline.

### Epoch setup

- **`consensus/src/epoch_manager.rs`** — At each epoch boundary, runs
  `TRXSuccinct::setup_for_testing` with the prototype parameters to
  produce the encryption key, digest key, master-secret-key shares, and
  verification keys for both the fast and slow paths, then builds the
  `DecConfig` for the epoch and hands it off to the execution client.
- **`consensus/src/state_replication.rs`**,
  **`consensus/src/state_computer.rs`**,
  **`consensus/src/pipeline/execution_client.rs`** — Plumb the per-epoch
  `DecConfig` (slow path and fast path) from the epoch manager down into
  the pipeline builder and the decryption manager.

### Decryption pipeline

- **`consensus/src/pipeline/pipeline_builder.rs`** — Adds the four
  encryption-protocol phases to the per-block consensus pipeline: digest
  computation (`TRXSuccinct::digest`), per-validator share derivation
  (`derive_decryption_key_share`), evaluation-proof computation
  (`eval_proofs_compute_all`), ciphertext preparation (`prepare_ct`), and
  final batch decryption (`decrypt`).
- **`consensus/src/rand/rand_gen/dec_manager.rs`** — Top-level manager
  that drives the share-broadcast protocol for a block, collects
  `DecShare`/`FastDecShare` messages, and produces the reconstructed
  `DecKey` once the threshold is reached.
- **`consensus/src/rand/rand_gen/dec_store.rs`** — Per-block store of
  received decryption shares; performs threshold aggregation against the
  `DecConfig`, parallelized via `DECRYPTION_POOL`.
- **`consensus/src/rand/rand_gen/network_messages.rs`** — Wire format
  for the decryption protocol: the `DecMessage` enum
  (`RequestDecShare`, `DecShare`, `FastDecShare`) and its verification
  against the epoch's `DecConfig`.
- **`consensus/src/rand/rand_gen/reliable_broadcast_state.rs`** —
  Reliable-broadcast state machine used to disseminate decryption shares.

### Test transaction generation

- **`crates/transaction-generator-lib/src/p2p_transaction_generator.rs`**
  — Uses the prototype `EncryptionKey` to produce encrypted P2P
  transactions for load tests and benchmarks.
