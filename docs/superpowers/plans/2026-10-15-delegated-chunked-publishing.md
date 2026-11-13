# Delegated Chunked Publishing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let A stage a package at a separately deployed Move contract while B independently verifies, signs and publishes it, using orderless staging when supported and a resumable sequence-number fallback.

**Architecture:** A standalone Move module owns A's position-keyed staging resource and binds it to B; finalization checks B's signer, exact BCS package digest, and chunk counts before calling framework publication. Dedicated `aptos move` commands reuse positional chunk splitting and the existing transaction submission backend without altering legacy `0x7` publishing. A fresh CLI localnet tests both modes with independently compiled package bytes.

**Tech Stack:** Move, Aptos framework `code`/`object_code_deployment`/`table_with_length`, Rust `aptos-move-cli`, `aptos-cli-common`, `aptos-crypto::HashValue`, BCS, REST API, Clap, cargo test, Aptos CLI localnet.

**Base:** Worktree `/Users/greg/git/aptos-core/.claude/worktrees/delegated-chunked-publish` on `delegated-chunked-publish`, stacked on `bound-table-info-retry` at `ce26b3b61af`. Read `docs/superpowers/specs/2026-10-15-delegated-chunked-publishing-design.md` first. The parent `large-packages-fixes` (#20744) and bounded-retry PR (#20749) must not be changed; do not copy untracked `exploits/`. Tests use `CARGO_TARGET_DIR=/Users/greg/git/aptos-core/target` for shared build artifacts and `ulimit -n 65536` for suites touching the database. Existing `aptos-framework` positional-builder baseline: 10 passing tests. Do not change consensus, crypto, gas metering, VM or database schema without explicit approval. No framework `.move` file is to be edited, so `head.mrb` must remain unchanged; if any framework `.move` file *is* touched, run `scripts/cargo_build_aptos_cached_packages.sh` and commit the regenerated bundle.

## File boundaries

- New `aptos-move/delegated-chunked-publishing/Move.toml`: independent package, local Aptos framework dependency, named address `delegated_packages = "_"`.
- New `aptos-move/delegated-chunked-publishing/sources/delegated_packages.move`: state layout, chunk operations, authorization, digest, three finalize functions, own cleanup, Move tests; no shared mutable framework code.
- New `aptos-move/cli/src/delegated_publish.rs`: package preimage, digest, positioned batches, feature/mode resolution, staged-state verification, command execution. Split into focused files inside `delegated_publish/` if this exceeds ~500 lines; do not expand `commands.rs` with all of the new logic.
- Modify `aptos-move/cli/src/lib.rs` and `aptos-move/cli/src/commands.rs:106-170` only for module/Clap registration and dispatch. Modify `aptos-move/cli/Cargo.toml` only if an existing workspace dependency is needed; prefer current `aptos-crypto` and `bcs` dependencies.
- New `aptos-move/cli/src/tests/publish/delegated_mock.rs`, register via `aptos-move/cli/src/tests/publish/mod.rs`: CLI/fake-chain tests, separate A/B credentials.
- New `aptos-move/delegated-chunked-publishing/README.md`: deploy/stage/finalize examples, address binding, refund/fallback semantics. Update `crates/aptos/CHANGELOG.md` for new CLI commands.

### Task 1: Standalone package and stage authorization

**Files:** Create `aptos-move/delegated-chunked-publishing/Move.toml`, `aptos-move/delegated-chunked-publishing/sources/delegated_packages.move`.

- [ ] **Step 1 (RED):** Write `#[test(stager = @0xa, publisher = @0xb)]` tests: A opens a staging area naming B; chunks at `(0,0)` and `(1,0)` may arrive in reverse order; identical repeats leave two keys; changed bytes abort `ECHUNK_CONFLICT`; reopening with another publisher aborts `EPUBLISHER_MISMATCH`. Assert `borrow_global<StagingArea>(@0xa).publisher == @0xb` and key count; expected failures specify abort codes. Manifest uses `AptosFramework = { local = "../framework/aptos-framework" }` and `[addresses] delegated_packages = "_"`.
- [ ] **Step 2 (confirm RED):** Run `target/ci/aptos move test --package-dir aptos-move/delegated-chunked-publishing --named-addresses delegated_packages=0xcafe` from repo root (or `cargo run --profile ci -p aptos -- move test ...`). Expect missing-entry/state compile errors.
- [ ] **Step 3 (GREEN):** Implement `StagingArea { publisher: address, chunks: TableWithLength<u64, vector<u8>>, keys: vector<u64> }` at A; `open_staging(stager: &signer, publisher: address)`; `stage_chunks(stager: &signer, part_indices: vector<u16>, chunk_indices: vector<u16>, chunks: vector<vector<u8>>)`, `cleanup_staging(stager: &signer)`. Key = `(part as u64) << 32 | (index as u64)`; require equal lengths and open-before-stage; borrow only from `signer::address_of(stager)`, compare duplicate bytes, and cleanup by removing every key then destroying the table. Define `ENOT_OPEN`, `ECHUNK_CONFLICT`, `EPUBLISHER_MISMATCH`, `ELENGTH_MISMATCH` with `std::error` categories.
- [ ] **Step 4 (verify and commit):** Run the same Move tests; expect all stage/cleanup cases to pass. B cannot stage under A because the entry has no A-address parameter. `git add aptos-move/delegated-chunked-publishing && git commit -m '[framework] Add standalone delegated staging contract'`.

Core state and signer boundary:

```move
struct StagingArea has key {
    publisher: address,
    chunks: TableWithLength<u64, vector<u8>>,
    keys: vector<u64>
}
public entry fun open_staging(stager: &signer, publisher: address) {
    let stager_address = signer::address_of(stager);
    assert!(!exists<StagingArea>(stager_address), error::already_exists(EPUBLISHER_MISMATCH));
    move_to(stager, StagingArea {
        publisher, chunks: table_with_length::new(), keys: vector[]
    });
}
```

For `stage_chunks`, borrow `StagingArea` *only* at `signer::address_of(stager)`; for each key, `contains` → assert identical bytes, else `add` and push key. Never accept `stager: address` as an entry argument.

### Task 2: Digest verification, publication, and rollback

**Files:** Modify `aptos-move/delegated-chunked-publishing/sources/delegated_packages.move`.

- [ ] **Step 1 (RED):** Add tests for `finalize_to_account(publisher: &signer, stager: address, counts: vector<u16>, expected_hash: vector<u8>)`: non-B fails `EWRONG_PUBLISHER`; wrong hash fails `EHASH_MISMATCH`; no metadata/module, zero count or missing chunk fails `EMISSING_CHUNK`; extra staged key fails `EUNEXPECTED_CHUNKS`. A successful test checks the framework emitted `code::PublishPackage`. Add equivalent object publish and upgrade tests: object owner is B; upgrading another account's object fails with framework's ownership abort. A failed transaction must leave A's `StagingArea` intact (assert via follow-up e2e transaction).
- [ ] **Step 2 (confirm RED):** Run the Task 1 Move test command; expect missing finalize entry functions, not a passing empty suite.
- [ ] **Step 3 (GREEN):** Check B's designation before touching chunks. `take_staged_package(stager, counts)` checks positive counts and exact positions, assembles metadata and ordered module bytes, deletes entries and destroys the table; abort rolls all of this back. Define private `struct PackagePreimage { metadata: vector<u8>, code: vector<vector<u8>> }` (no abilities); move the assembled vectors into `PackagePreimage { metadata, code }`, hash `std::hash::sha3_256(bcs::to_bytes(&preimage))`, require 32 bytes and exact match to B's `expected_hash`, then destructure the preimage to pass the *same* metadata and code to publication (avoid cloning the entire package). `finalize_to_account` calls `code::publish_package_txn(publisher, ...)`; `finalize_to_object` calls `object_code_deployment::publish(publisher, ...)`; `finalize_upgrade` calls `object_code_deployment::upgrade(publisher, ..., code_object: Object<PackageRegistry>)`. Never modify crypto or Move VM.
- [ ] **Step 4 (verify and commit):** Re-run Move tests; expect authorization, hash, counts, ownership and success cases to pass. `git add aptos-move/delegated-chunked-publishing/sources && git commit -m '[framework] Verify delegated package digest before publication'`.

Finalization boundary (reuse for all three destinations):

```move
struct PackagePreimage { metadata: vector<u8>, code: vector<vector<u8>> }
fun take_verified(stager: address, publisher: address, counts: vector<u16>, expected: vector<u8>)
    : (vector<u8>, vector<vector<u8>>) acquires StagingArea {
    assert!(exists<StagingArea>(stager), error::not_found(ENOT_OPEN));
    assert!(borrow_global<StagingArea>(stager).publisher == publisher,
        error::permission_denied(EWRONG_PUBLISHER));
    let (metadata, code) = take_staged_package(stager, counts);
    let preimage = PackagePreimage { metadata, code };
    assert!(expected.length() == 32 && hash::sha3_256(bcs::to_bytes(&preimage)) == expected,
        error::invalid_argument(EHASH_MISMATCH));
    let PackagePreimage { metadata, code } = preimage;
    (metadata, code)
}
```

Call `take_verified(stager, signer::address_of(publisher), counts, expected)` before each framework publish; re-read `object_code_deployment.move:80-133` for owner checks.

### Task 3: Rust package digest, positioning and payloads

**Files:** Create `aptos-move/cli/src/delegated_publish.rs`; modify `aptos-move/cli/src/lib.rs` (add `mod delegated_publish`); create `aptos-move/cli/src/tests/publish/delegated_mock.rs` and register via `aptos-move/cli/src/tests/publish/mod.rs`.

- [ ] **Step 1 (RED):** Add tests for `package_digest(&metadata, &code)` using metadata `b"m"`, code `[b"a", b"bc"]`, comparing to `HashValue::sha3_256_of(&bcs::to_bytes(&(b"m".to_vec(), vec![b"a".to_vec(), b"bc".to_vec()])).unwrap())`. Assert swapping module order changes hash; include matching known digest bytes in a Move unit test by hardcoding the Rust-computed value. Add payload tests that a separate `open_staging` payload carries B, `stage_chunks` carries the `(part,index,bytes)` vectors, and finalize payload carries A, counts and expected digest, *not* module bytes.
- [ ] **Step 2 (confirm RED):** Run `CARGO_TARGET_DIR=/Users/greg/git/aptos-core/target cargo test --profile ci -p aptos-move-cli -- delegated_mock` and expect missing digest/payload functions.
- [ ] **Step 3 (GREEN):** Implement `package_digest(metadata: &[u8], code: &[Vec<u8>]) -> CliTypedResult<HashValue>` with `bcs::to_bytes(&(metadata, code))` and `HashValue::sha3_256_of`; match the Move struct serialization byte for byte. Reuse `aptos_framework::chunked_publish::PositionalChunks::new` and its public `PositionedChunk` for splitting and missing positions. Build `EntryFunction::new(ModuleId::new(address, Identifier::new("delegated_packages")?), ident_str!("open_staging"), ...)` and the corresponding `stage_chunks`/`finalize_*` payloads. Batch chunks to <= `CHUNK_SIZE_IN_BYTES` using the size rule of `create_positional_payloads_for` but **without** its appended final publish transaction.
- [ ] **Step 4 (verify and commit):** Run the delegated Rust tests and all ten `cargo test --profile ci -p aptos-framework --lib -- chunked_publish` tests; expect pass and no change to `aptos-move/framework/src/chunked_publish.rs`. `git add aptos-move/cli/src/delegated_publish.rs aptos-move/cli/src/lib.rs aptos-move/cli/src/tests/publish && git commit -m '[cli] Build delegated chunk payloads and canonical digest'`.

Digest implementation and example test (no new dependency):

```rust
fn package_digest(metadata: &[u8], code: &[Vec<u8>]) -> CliTypedResult<HashValue> {
    let preimage = bcs::to_bytes(&(metadata, code))
        .map_err(|e| CliError::UnexpectedError(e.to_string()))?;
    Ok(HashValue::sha3_256_of(&preimage))
}
#[test]
fn digest_binds_order_and_lengths() {
    let one = package_digest(b"m", &[b"a".to_vec(), b"bc".to_vec()]).unwrap();
    let reordered = package_digest(b"m", &[b"bc".to_vec(), b"a".to_vec()]).unwrap();
    assert_ne!(one, reordered);
}
```

Confirm BCS serialization of the Move two-field struct equals Rust's tuple encoding with a cross-language byte vector test before using it on chain.

### Task 4: Transaction mode resolution and concurrent orderless batches

**Files:** Modify `aptos-move/cli/src/delegated_publish.rs`; modify `crates/aptos/src/aptos_context.rs:390-475,630-775` only if nonce windows require a new per-command submission setting; test in `aptos-move/cli/src/tests/publish/delegated_mock.rs` and `crates/aptos/src/aptos_context.rs`.

- [ ] **Step 1 (RED):** Unit-test `resolve_stage_mode(requested, chain_enabled, is_session)` for `(Auto,true,false)->Nonce`, `(Auto,false,false)->Seqnum`, `(Nonce,false,false)->error`, `(Seqnum,true,false)->Seqnum`, `(Auto,true,true)->Seqnum` unless the session exposes verifiable orderless support. Test all `submit_group` signed transactions in nonce mode have distinct replay-protection nonces and not a dependence on consecutive sequence numbers; retained existing sequence-number tests must still pass. Test read failure does not silently choose seqnum.
- [ ] **Step 2 (confirm RED):** Run `cargo test --profile ci -p aptos-move-cli -- delegated_mock` and `cargo test --profile ci -p aptos -- submit_group`; expect missing mode resolver and nonce assertions to fail.
- [ ] **Step 3 (GREEN):** `enum StageMode { Auto, Nonce, Seqnum }` as a Clap value enum on the new stage command (do **not** change global `ReplayProtectionType` default). Read `Features` via `txn_options.rest_client()?.get_account_resource_bcs::<Features>(CORE_CODE_ADDRESS, "0x1::features::Features")`, then `features.is_enabled(FeatureFlag::ORDERLESS_TRANSACTIONS)`. For each stage batch, pass nonce/seqnum via the stage command's transaction options while leaving finalization seqnum. Inspect `ChainSender::sign` and `estimate_gas` in `crates/aptos/src/aptos_context.rs`: if necessary add a nonce-aware branch to `submit_group` so all signed staging txns have distinct random nonces via `upgrade_payload_with_rng`, simulated one by one with correct gas settings; preserve the sequence-number branch and no global behavioral changes. Generate nonces once per signed transaction, and never reuse a signed transaction for a different chunk.
- [ ] **Step 4 (verify and commit):** Run tests above with a fake chain that fails if a nonce is repeated or a sequence number is assumed. `git add aptos-move/cli/src/delegated_publish.rs aptos-move/cli/src/tests/publish/delegated_mock.rs crates/aptos/src/aptos_context.rs && git commit -m '[cli] Stage delegated chunks with orderless nonces when supported'` (omit the last path if unchanged).

### Task 5: Stage-only and independent finalization CLI

**Files:** Modify `aptos-move/cli/src/delegated_publish.rs` (split into `delegated_publish/{mod.rs,stage.rs,finalize.rs}` if needed), `aptos-move/cli/src/commands.rs:106-170`, `aptos-move/cli/src/lib.rs`, `aptos-move/cli/src/tests/publish/delegated_mock.rs`, `crates/aptos/CHANGELOG.md`.

- [ ] **Step 1 (RED):** Mock tests: `--module-address 0xcafe --publisher B` creates A's staging before chunks; a resumed run skips identical positions and sends only missing ones; a different publisher or conflicting chunk fails **without cleanup**; a failed/discarded chunk exits nonzero; B-signed `--stager A --module-address 0xcafe` submits one final transaction with the *locally* derived hash. Reject missing/incorrect ABI and B signer mismatch. Use `TransactionSummary { success: None, pending: None, vm_status: Some("Discarded".into()), .. }` for failure.
- [ ] **Step 2 (confirm RED):** Run `CARGO_TARGET_DIR=/Users/greg/git/aptos-core/target cargo test --profile ci -p aptos-move-cli -- delegated_mock`; expect missing command registration.
- [ ] **Step 3 (GREEN):** Add `MoveTool::StageDelegatedPackage` and `MoveTool::FinalizeDelegatedPackage` plus `execute_serialized` dispatch. Both flatten existing `MovePackageOptions`, `IncludedArtifactsArgs`, `TransactionOptions`; require `--module-address`. Stage requires `--publisher`; finalize requires `--stager` and destination Account/NewObject/Upgrade (`--object-address` for Upgrade). Reuse `build_package_options`, `dispatch_transactions`, `dispatch_transaction`, `did_not_succeed`, `chunk_windows` and `PositionalChunks`. Read A's `StagingArea` resource and table entries using `get_account_resource_bcs` and `get_table_item_bcs::<String, Vec<u8>>`; compare bytes and designated B. Add explicit A-signed `cleanup_staging` option/command; never clear automatically. Wait for each stage window and report failures; compute digest only from the local package. Keep legacy commands unchanged.
- [ ] **Step 4 (verify and commit):** Run all `cargo test --profile ci -p aptos-move-cli` with raised file descriptor limit. Expect existing tests and new mocks passing. `git add aptos-move/cli crates/aptos/CHANGELOG.md && git commit -m '[cli] Add delegated stage and finalize commands'`.

### Task 6: New-object expected sequence and destination binding

**Files:** Modify `aptos-move/cli/src/delegated_publish.rs` (or `stage.rs`/`finalize.rs`), `aptos-move/cli/src/tests/publish/delegated_mock.rs`.

- [ ] **Step 1 (RED):** Tests derive `create_object_code_deployment_address(B, expected_next_seq)` at A's stage time, where `expected_next_seq = B's current on-chain sequence number + 1` (the contract's `object_seed` adds 1); when B's current sequence changes between upload and finalization, the finalize command rejects before sending. For a valid sequence, final payload uses B's current seqnum (`expected_next_seq - 1`) and compiled code uses the object address. Account publishing compiles for B; upgrades compile for the already-owned object. Never guess an unknown sequence.
- [ ] **Step 2 (confirm RED):** Run delegated mock filter; expect object destination checks to fail or be absent.
- [ ] **Step 3 (GREEN):** Require `--expected-publisher-sequence` on A's stage command for NewObject, defined as the value `account::get_sequence_number(B) + 1` that Move will put into `object_seed` when B finalizes. Derive its object address via `aptos_types::object_address::create_object_code_deployment_address(B, expected_seq)`. Require the same value at B's finalize; fetch B's current account sequence (or session AccountResource), require `current_seq.checked_add(1) == Some(expected_seq)`, and send finalization with transaction sequence number `current_seq`, rejecting a nonce final option. Recompile with derived named address and compare staged bytes/hash. If B's next sequence changes, A can cleanup and restage for the new address.
- [ ] **Step 4 (verify and commit):** Re-run delegated mocks and all CLI tests; commit `[cli] Bind new-object delegation to publisher sequence`.

### Task 7: Fresh localnet, negative transactions, and large-package gas

**Files:** Create `aptos-move/delegated-chunked-publishing/README.md`, update `crates/aptos/CHANGELOG.md`; use scratch artifacts under `/tmp/delegated-publish-test/` for node state, accounts and test packages (never commit keys).

- [ ] **Step 1 (baseline and deploy):** `ulimit -n 65536; CARGO_TARGET_DIR=/Users/greg/git/aptos-core/target cargo build --profile ci -p aptos`; start `target/ci/aptos node run-localnet --test-dir /tmp/delegated-publish-test/localnet --force-restart --assume-yes`. Confirm REST at `http://127.0.0.1:8080/v1`. Deploy independent package with `target/ci/aptos move publish --package-dir aptos-move/delegated-chunked-publishing --named-addresses delegated_packages=<deployer> --url http://127.0.0.1:8080 --private-key-file <scratch-file> --assume-yes`; assert module is returned by REST `GET /v1/accounts/<deployer>/module/delegated_packages`.
- [ ] **Step 2 (RED live cases):** Fund three accounts A, B and C. A stages a package with publisher B and forced nonce; confirm distinct nonces in returned transaction summaries. Attempt C-signed finalize and B-signed finalize against a different local package; both must fail on-chain with expected abort and leave `StagingArea` at A. Attempt finalization with a missing position; must fail without consuming state. Confirm a failed upload stops and reports staged subset.
- [ ] **Step 3 (GREEN live cases):** B compiles the same local package and finalizes to B; assert code and `PackageRegistry` at B, no A staging resource, and `storage_fee_refund_octas > 0` in final transaction. Repeat with `--stage-mode seqnum`, kill staging mid-window, and resume; verify no duplicate keys and contiguous A sequence numbers. For NewObject use B's expected next seq, confirm object address and owner B; after an intervening B transaction ensure preflight fails and leaves staging. Exercise B-owned object upgrade. Measure package digest/gas on 786KB test package and verify finalization fits on-chain gas limit.
- [ ] **Step 4 (docs and commit):** Document exact deploy, stage, finalize, cleanup and forced fallback invocations in the README, explain that the deletion refund goes to the *final fee payer*, and add CLI changelog. `git add aptos-move/delegated-chunked-publishing/README.md crates/aptos/CHANGELOG.md && git commit -m '[docs] Document independent delegated publish deployment'`.

### Task 8: Full verification and review

**Files:** Only files changed in Tasks 1–7; no framework Move sources. Keep new code on `delegated-chunked-publish`, leaving `large-packages-fixes` and `bound-table-info-retry` unmodified.

- [ ] **Step 1:** Run `cargo +nightly fmt --check`, `CARGO_TARGET_DIR=/Users/greg/git/aptos-core/target cargo check -p aptos -p aptos-move-cli`, `cargo test --profile ci -p aptos-move-cli`, `cargo test --profile ci -p aptos`, and standalone Move tests. Use `ulimit -n 65536`. Record exact outputs; no successful claim without seeing passing output. Clippy: `cargo clippy -p aptos-move-cli -p aptos --all-targets -- --check-cfg='cfg(fuzzing)' -Wunexpected_cfgs -Wclippy::all` with the workspace's existing lint allowances from `.cargo/config.toml`.
- [ ] **Step 2:** Review security-critical paths manually: A cannot stage to B's account; C cannot consume A's staging; hash cannot be chosen from chain state; B cannot publish under C; wrong hash and framework failure roll back removals; positions, counts and BCS digest are unambiguous; no unbounded wait or duplicate nonce. Run `git diff --check && git status --short && git diff --stat bound-table-info-retry...HEAD`. `aptos-move/framework/cached-packages/src/head.mrb` must not have changed. Stop the localnet process before completion.
- [ ] **Step 3:** Ask for code review (security-aware review of the new signer boundary and nonce path); address only findings verified against source and rerun impacted tests. The plan ends with a clean branch; create a stacked PR against `bound-table-info-retry` only if the user explicitly requests it or as part of a separately approved ship step.

## Review checklist against the spec

- Separate module/address and independent package: Tasks 1, 7.
- A designates B; B signs; cleanup, exact chunks and SHA3-256 of BCS metadata+ordered code: Tasks 1–3.
- CLI local package for both signers, ABI check and resume: Tasks 3, 5.
- Auto orderless with override, seqnum fallback, committed window handling: Tasks 4–5.
- B seqnum for new object and mismatch safety; object upgrade: Tasks 2, 6–7.
- Wrong publisher/hash/chunks, storage-refund payer and big-package gas: Tasks 2, 5, 7–8.
- Existing `0x7` and single-account paths untouched; no cached bundle regen: file boundaries and Task 8.
