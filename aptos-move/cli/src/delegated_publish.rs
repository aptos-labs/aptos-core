// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_cli_common::{CliError, CliTypedResult, TransactionOptions};
use aptos_crypto::HashValue;
use aptos_framework::chunked_publish::{
    chunk_position, PositionalChunks, PositionedChunk, CHUNK_SIZE_IN_BYTES,
};
use aptos_types::{
    on_chain_config::Features,
    transaction::{EntryFunction, TransactionPayload},
};
use clap::ValueEnum;
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::IdentStr, language_storage::ModuleId,
};
use serde::Deserialize;
use std::{collections::HashSet, str::FromStr};

/// Replay protection for the staging transactions only. Finalization chooses independently.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum StageMode {
    Auto,
    Nonce,
    Seqnum,
}

/// Decide from the *verified* chain feature flag; a failed lookup is an error, not `false`.
/// Offline sessions cannot attest to nonce support, so auto uses sequence numbers there.
pub(crate) fn resolve_stage_mode(
    requested: StageMode,
    chain_enabled: bool,
    is_session: bool,
) -> CliTypedResult<StageMode> {
    match requested {
        StageMode::Seqnum => Ok(StageMode::Seqnum),
        StageMode::Nonce if chain_enabled && !is_session => Ok(StageMode::Nonce),
        StageMode::Nonce => Err(CliError::CommandArgumentError(
            "Orderless staging requires an online chain with ORDERLESS_TRANSACTIONS enabled".into(),
        )),
        StageMode::Auto if chain_enabled && !is_session => Ok(StageMode::Nonce),
        StageMode::Auto => Ok(StageMode::Seqnum),
    }
}

/// Query the chain's on-chain feature bitset; never infer lack of support from a REST error.
pub(crate) async fn chain_orderless_enabled(options: &TransactionOptions) -> CliTypedResult<bool> {
    let features = options
        .rest_client()?
        .get_account_resource_bcs::<Features>(AccountAddress::ONE, "0x1::features::Features")
        .await
        .map_err(|err| CliError::ApiError(format!("Failed to read orderless feature flag: {err}")))?
        .into_inner();
    Ok(features.is_orderless_txns_enabled())
}

/// Reject a package compiled at any address other than the intended account or code object.
fn check_compiled_destination(code: &[Vec<u8>], destination: AccountAddress) -> CliTypedResult<()> {
    if code.is_empty() {
        return Err(CliError::CommandArgumentError(
            "Package has no modules".into(),
        ));
    }
    for bytes in code {
        let module = move_binary_format::CompiledModule::deserialize(bytes)
            .map_err(|e| CliError::MoveCompilationError(format!("Invalid compiled module: {e}")))?;
        if module.self_id().address() != &destination {
            return Err(CliError::CommandArgumentError(format!(
                "Compiled module {} is not addressed to expected destination {destination}",
                module.self_id()
            )));
        }
    }
    Ok(())
}

/// SHA3-256 of the BCS encoding of (metadata bytes, ordered module byte vectors).
/// This is byte-for-byte the BCS encoding of Move's two-field PackagePreimage.
pub(crate) fn package_digest(metadata: &[u8], code: &[Vec<u8>]) -> CliTypedResult<HashValue> {
    let preimage =
        bcs::to_bytes(&(metadata, code)).map_err(|e| CliError::UnexpectedError(e.to_string()))?;
    Ok(HashValue::sha3_256_of(&preimage))
}

fn payload(address: AccountAddress, function: &IdentStr, args: Vec<Vec<u8>>) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(address, ident_str!("delegated_packages").to_owned()),
        function.to_owned(),
        vec![],
        args,
    ))
}

/// Must be sent and committed before any chunk transactions.
pub(crate) fn open_staging(
    address: AccountAddress,
    publisher: AccountAddress,
    generation: AccountAddress,
) -> TransactionPayload {
    payload(address, ident_str!("open_staging"), vec![
        bcs::to_bytes(&publisher).expect("address serializes"),
        bcs::to_bytes(&generation).expect("address serializes"),
    ])
}

/// A-signed explicit cleanup for a staging area, including one targeting another package.
fn cleanup_staging(address: AccountAddress, generation: AccountAddress) -> TransactionPayload {
    payload(address, ident_str!("cleanup_staging"), vec![bcs::to_bytes(
        &generation,
    )
    .expect("address serializes")])
}

/// Stage only the selected chunks (possibly the missing subset of a resumed upload).
/// Never appends a publish transaction: finalization is separately signed by B.
/// The limit is on raw chunk bytes per batch (not the serialized transaction size),
/// as in the existing positional splitter. The latter adds BCS and entry-function overhead.
pub(crate) fn stage_chunks(
    address: AccountAddress,
    generation: AccountAddress,
    chunks: Vec<PositionedChunk>,
    chunk_size: usize,
) -> Vec<TransactionPayload> {
    assert!(
        (1..=CHUNK_SIZE_IN_BYTES).contains(&chunk_size),
        "chunk size must be in 1..={CHUNK_SIZE_IN_BYTES}"
    );
    let mut result = Vec::new();
    let mut parts = Vec::new();
    let mut indices = Vec::new();
    let mut bytes = Vec::new();
    let mut size = 0;
    for chunk in chunks {
        assert!(chunk.bytes.len() <= chunk_size, "chunk exceeds batch size");
        if size + chunk.bytes.len() > chunk_size {
            result.push(stage_batch(address, generation, &parts, &indices, &bytes));
            parts.clear();
            indices.clear();
            bytes.clear();
            size = 0;
        }
        size += chunk.bytes.len();
        parts.push(chunk.part);
        indices.push(chunk.index);
        bytes.push(chunk.bytes);
    }
    if !bytes.is_empty() {
        result.push(stage_batch(address, generation, &parts, &indices, &bytes));
    }
    result
}

fn stage_batch(
    address: AccountAddress,
    generation: AccountAddress,
    parts: &[u16],
    indices: &[u16],
    bytes: &[Vec<u8>],
) -> TransactionPayload {
    payload(address, ident_str!("stage_chunks"), vec![
        bcs::to_bytes(&generation).expect("address serializes"),
        bcs::to_bytes(&parts).expect("part indices serialize"),
        bcs::to_bytes(&indices).expect("chunk indices serialize"),
        bcs::to_bytes(&bytes).expect("chunk bytes serialize"),
    ])
}

fn finalize_args(stager: AccountAddress, counts: &[u16], digest: HashValue) -> Vec<Vec<u8>> {
    vec![
        bcs::to_bytes(&stager).expect("address serializes"),
        bcs::to_bytes(&counts).expect("counts serialize"),
        bcs::to_bytes(&digest.to_vec()).expect("digest serializes"),
    ]
}

pub(crate) fn finalize_to_account(
    address: AccountAddress,
    stager: AccountAddress,
    counts: &[u16],
    digest: HashValue,
) -> TransactionPayload {
    payload(
        address,
        ident_str!("finalize_to_account"),
        finalize_args(stager, counts, digest),
    )
}

pub(crate) fn finalize_to_object(
    address: AccountAddress,
    stager: AccountAddress,
    counts: &[u16],
    digest: HashValue,
) -> TransactionPayload {
    payload(
        address,
        ident_str!("finalize_to_object"),
        finalize_args(stager, counts, digest),
    )
}

pub(crate) fn finalize_upgrade(
    address: AccountAddress,
    stager: AccountAddress,
    counts: &[u16],
    digest: HashValue,
    object: AccountAddress,
) -> TransactionPayload {
    let mut args = finalize_args(stager, counts, digest);
    // Object<PackageRegistry> is represented by its address at entry ABI level.
    args.push(bcs::to_bytes(&object).expect("address serializes"));
    payload(address, ident_str!("finalize_upgrade"), args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_framework::chunked_publish::{chunk_key, PositionalChunks};
    use aptos_types::transaction::EntryFunction;

    fn address(s: &str) -> AccountAddress {
        AccountAddress::from_hex_literal(s).unwrap()
    }

    fn entry<'a>(
        payload: &'a TransactionPayload,
        module_address: AccountAddress,
        function: &str,
    ) -> &'a EntryFunction {
        let TransactionPayload::EntryFunction(entry) = payload else {
            panic!("not an entry function")
        };
        assert_eq!(entry.module().address(), &module_address);
        assert_eq!(entry.module().name().as_str(), "delegated_packages");
        assert_eq!(entry.function().as_str(), function);
        assert!(entry.ty_args().is_empty());
        entry
    }

    #[test]
    fn delegated_publish_resolves_stage_mode_from_verified_features() {
        for (requested, enabled, session, expected) in [
            (StageMode::Auto, true, false, StageMode::Nonce),
            (StageMode::Auto, false, false, StageMode::Seqnum),
            (StageMode::Auto, true, true, StageMode::Seqnum),
            (StageMode::Seqnum, true, false, StageMode::Seqnum),
            (StageMode::Seqnum, false, true, StageMode::Seqnum),
            (StageMode::Nonce, true, false, StageMode::Nonce),
        ] {
            assert_eq!(
                resolve_stage_mode(requested, enabled, session).unwrap(),
                expected
            );
        }
        for (enabled, session) in [(false, false), (true, true)] {
            assert!(resolve_stage_mode(StageMode::Nonce, enabled, session)
                .unwrap_err()
                .to_string()
                .contains("ORDERLESS_TRANSACTIONS"));
        }
    }

    #[test]
    fn delegated_publish_digest_matches_move_known_vector() {
        // Move PackagePreimage { metadata: b"meta", code: vector[b"code"] }.
        // BCS preimage: 04 6d657461 01 04 636f6465.
        let digest = package_digest(b"meta", &[b"code".to_vec()]).unwrap();
        assert_eq!(
            digest,
            HashValue::from_slice(
                hex::decode("438f72562b990e09cb3383e87ea342434773e84285e85ceadc7d71dd831b812f")
                    .unwrap()
            )
            .unwrap()
        );
        assert_eq!(
            bcs::to_bytes(&(b"meta".as_slice(), &[b"code".to_vec()][..])).unwrap(),
            hex::decode("046d6574610104636f6465").unwrap()
        );
        let code = vec![b"a".to_vec(), b"bc".to_vec()];
        let expected = HashValue::sha3_256_of(&bcs::to_bytes(&(b"m".to_vec(), &code)).unwrap());
        assert_eq!(package_digest(b"m", &code).unwrap(), expected);
        assert_ne!(
            package_digest(b"m", &code).unwrap(),
            package_digest(b"m", &[b"bc".to_vec(), b"a".to_vec()]).unwrap()
        );
        assert_ne!(
            package_digest(b"m", &code).unwrap(),
            package_digest(b"ma", &code).unwrap()
        );
    }

    #[test]
    fn delegated_publish_staging_is_separate_and_positioned() {
        let module = address("0xcafe");
        let publisher = address("0xb");
        let generation = address("0xd");
        let open = open_staging(module, publisher, generation);
        let cleanup = cleanup_staging(module, generation);
        let cleanup_args = entry(&cleanup, module, "cleanup_staging").args();
        assert_eq!(cleanup_args.len(), 1);
        assert_eq!(
            bcs::from_bytes::<AccountAddress>(&cleanup_args[0]).unwrap(),
            generation
        );
        let args = entry(&open, module, "open_staging").args();
        assert_eq!(args.len(), 2);
        assert_eq!(
            bcs::from_bytes::<AccountAddress>(&args[1]).unwrap(),
            generation
        );
        assert_eq!(
            bcs::from_bytes::<AccountAddress>(&args[0]).unwrap(),
            publisher
        );

        let package = PositionalChunks::new(vec![1; 5], vec![vec![2; 7], vec![3; 2]], 3);
        assert_eq!(package.chunk_counts, vec![2, 3, 1]);
        assert_eq!(
            package
                .chunks
                .iter()
                .map(|c| chunk_key(c.part, c.index))
                .collect::<Vec<_>>(),
            vec![0, 1, 1 << 32, (1 << 32) | 1, (1 << 32) | 2, 2 << 32]
        );
        let payloads = stage_chunks(module, generation, package.chunks.clone(), 6);
        assert_eq!(payloads.len(), 3);
        let mut staged = vec![];
        for payload in &payloads {
            let args = entry(payload, module, "stage_chunks").args();
            assert_eq!(args.len(), 4);
            assert_eq!(
                bcs::from_bytes::<AccountAddress>(&args[0]).unwrap(),
                generation
            );
            let parts: Vec<u16> = bcs::from_bytes(&args[1]).unwrap();
            let indices: Vec<u16> = bcs::from_bytes(&args[2]).unwrap();
            let bytes: Vec<Vec<u8>> = bcs::from_bytes(&args[3]).unwrap();
            assert!(bytes.iter().map(Vec::len).sum::<usize>() <= 6);
            staged.extend(
                parts
                    .into_iter()
                    .zip(indices)
                    .zip(bytes)
                    .map(|((part, index), bytes)| (part, index, bytes)),
            );
        }
        assert_eq!(
            staged,
            package
                .chunks
                .iter()
                .map(|c| (c.part, c.index, c.bytes.clone()))
                .collect::<Vec<_>>()
        );
        assert!(stage_chunks(module, generation, vec![], 6).is_empty());
        let staged_positions = [(0, 0), (1, 1)].into_iter().collect();
        let missing = package.missing(&staged_positions);
        let resumed = stage_chunks(module, generation, missing.clone(), 6);
        let resumed_positions: Vec<(u16, u16)> = resumed
            .iter()
            .flat_map(|payload| {
                let args = entry(payload, module, "stage_chunks").args();
                assert_eq!(
                    bcs::from_bytes::<AccountAddress>(&args[0]).unwrap(),
                    generation
                );
                let parts: Vec<u16> = bcs::from_bytes(&args[1]).unwrap();
                let indices: Vec<u16> = bcs::from_bytes(&args[2]).unwrap();
                parts.into_iter().zip(indices).collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(
            resumed_positions,
            missing
                .iter()
                .map(|c| (c.part, c.index))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn delegated_publish_batches_respect_raw_chunk_budget_at_boundary() {
        let module = address("0xcafe");
        let chunks = vec![
            PositionedChunk {
                part: 0,
                index: 0,
                bytes: vec![1; CHUNK_SIZE_IN_BYTES],
            },
            PositionedChunk {
                part: 1,
                index: 0,
                bytes: vec![2; 1],
            },
        ];
        let payloads = stage_chunks(module, address("0xd"), chunks, CHUNK_SIZE_IN_BYTES);
        assert_eq!(payloads.len(), 2);
        for (payload, expected) in payloads.iter().zip([CHUNK_SIZE_IN_BYTES, 1]) {
            let args = entry(payload, module, "stage_chunks").args();
            let bytes: Vec<Vec<u8>> = bcs::from_bytes(&args[3]).unwrap();
            assert_eq!(bytes.iter().map(Vec::len).sum::<usize>(), expected);
        }
    }

    #[test]
    #[should_panic(expected = "chunk size must be in")]
    fn delegated_publish_rejects_oversized_batch_budget() {
        stage_chunks(
            address("0xcafe"),
            address("0xd"),
            vec![],
            CHUNK_SIZE_IN_BYTES + 1,
        );
    }

    #[test]
    fn delegated_publish_resume_rejects_wrong_publisher_and_unexpected_positions() {
        let publisher = address("0xb");
        let package = PositionalChunks::new(b"metadata".to_vec(), vec![b"code".to_vec()], 4);
        let mut area = DelegatedStagingAreaLayout {
            publisher,
            generation: address("0xd"),
            chunks: DelegatedTableWithLengthLayout {
                inner: DelegatedTableLayout {
                    handle: address("0xabc"),
                },
                length: 2,
            },
            keys: vec![0, 1u64 << 32],
        };
        assert_eq!(
            staged_positions(&area, publisher, &package).unwrap().len(),
            2
        );
        assert!(staged_positions(&area, address("0xc"), &package).is_err());
        area.keys[1] = 5u64 << 32;
        assert!(staged_positions(&area, publisher, &package).is_err());
        area.keys[1] = 0;
        assert!(staged_positions(&area, publisher, &package).is_err());
        area.keys[1] = 1u64 << 32;
        area.chunks.length = 3;
        assert!(staged_positions(&area, publisher, &package).is_err());
    }

    #[test]
    fn delegated_publish_both_signers_must_bind_same_upgrade_object() {
        let publisher = address("0xb");
        let object = address("0xcafe");
        assert_eq!(
            check_destination(DelegatedDestination::Account, None, None, publisher).unwrap(),
            publisher
        );
        assert_eq!(
            check_destination(DelegatedDestination::Upgrade, None, Some(object), publisher)
                .unwrap(),
            object
        );
        assert!(check_destination(DelegatedDestination::Upgrade, None, None, publisher).is_err());
        assert!(
            check_destination(DelegatedDestination::Account, None, Some(object), publisher)
                .is_err()
        );
        assert!(check_destination(
            DelegatedDestination::NewObject,
            Some(1),
            Some(object),
            publisher
        )
        .is_err());
    }

    #[test]
    fn delegated_publish_cleanup_requires_progress_until_area_is_gone() {
        let generation = address("0xd");
        assert!(check_cleanup_progress(12, Some((generation, 2)), generation).is_ok());
        assert!(check_cleanup_progress(2, None, generation).is_ok());
        assert!(check_cleanup_progress(12, Some((generation, 12)), generation).is_err());
        assert!(check_cleanup_progress(2, Some((generation, 3)), generation).is_err());
        assert!(check_cleanup_progress(12, Some((address("0xe"), 2)), generation).is_err());
    }

    #[test]
    fn delegated_publish_object_seed_is_next_publisher_sequence() {
        let publisher = address("0xb");
        let expected =
            aptos_types::object_address::create_object_code_deployment_address(publisher, 8);
        assert_eq!(
            validate_destination_sequence(DelegatedDestination::NewObject, Some(8), publisher)
                .unwrap(),
            Some(expected)
        );
        assert!(
            validate_destination_sequence(DelegatedDestination::NewObject, None, publisher)
                .is_err()
        );
        assert!(
            validate_destination_sequence(DelegatedDestination::Account, Some(8), publisher)
                .is_err()
        );
        assert!(
            validate_destination_sequence(DelegatedDestination::Upgrade, None, publisher)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn delegated_publish_resume_rejects_different_bytes() {
        let package = PositionalChunks::new(b"metadata".to_vec(), vec![b"code".to_vec()], 4);
        assert!(check_staged_bytes(&package, 0, 0, b"meta").is_ok());
        assert!(check_staged_bytes(&package, 0, 0, b"evil").is_err());
        assert!(check_staged_bytes(&package, 2, 0, b"evil").is_err());
    }

    #[test]
    fn delegated_publish_finalization_carries_only_stager_counts_digest_and_optional_object() {
        let module = address("0xcafe");
        let stager = address("0xa");
        let object = address("0x123");
        let counts = vec![2, 1];
        let digest = package_digest(b"meta", &[b"code".to_vec()]).unwrap();
        for (payload, name, has_object) in [
            (
                finalize_to_account(module, stager, &counts, digest),
                "finalize_to_account",
                false,
            ),
            (
                finalize_to_object(module, stager, &counts, digest),
                "finalize_to_object",
                false,
            ),
            (
                finalize_upgrade(module, stager, &counts, digest, object),
                "finalize_upgrade",
                true,
            ),
        ] {
            let args = entry(&payload, module, name).args();
            assert_eq!(args.len(), if has_object { 4 } else { 3 });
            assert_eq!(bcs::from_bytes::<AccountAddress>(&args[0]).unwrap(), stager);
            assert_eq!(bcs::from_bytes::<Vec<u16>>(&args[1]).unwrap(), counts);
            assert_eq!(
                bcs::from_bytes::<Vec<u8>>(&args[2]).unwrap(),
                digest.to_vec()
            );
            if has_object {
                assert_eq!(bcs::from_bytes::<AccountAddress>(&args[3]).unwrap(), object);
            }
        }
    }
}

/// Upload a package under the signing account; publication requires a separate B-signed command.
#[derive(clap::Parser)]
pub struct StageDelegatedPackage {
    #[clap(long, value_parser = aptos_cli_common::load_account_arg)]
    module_address: AccountAddress,
    #[clap(long, value_parser = aptos_cli_common::load_account_arg)]
    publisher: AccountAddress,
    #[clap(long, value_enum, default_value = "account")]
    destination: DelegatedDestination,
    #[clap(long, value_parser = aptos_cli_common::load_account_arg)]
    object_address: Option<AccountAddress>,
    /// For a new object: the publisher's current on-chain sequence number + 1.
    #[clap(long)]
    expected_publisher_sequence: Option<u64>,
    #[clap(flatten)]
    move_options: crate::move_types::MovePackageOptions,
    #[clap(flatten)]
    included_artifacts: crate::commands::IncludedArtifactsArgs,
    #[clap(flatten)]
    txn_options: TransactionOptions,
    #[clap(long, value_enum, default_value = "auto")]
    stage_mode: StageMode,
    #[clap(long, default_value_t = CHUNK_SIZE_IN_BYTES)]
    chunk_size: usize,
    #[clap(long, default_value_t = 10, value_parser = clap::value_parser!(u16).range(1..=100))]
    parallelism: u16,
    #[clap(skip)]
    pub env: std::sync::Arc<crate::MoveEnv>,
}

impl crate::WithMoveEnv for StageDelegatedPackage {
    fn attach_env(mut self, env: std::sync::Arc<crate::MoveEnv>) -> Self {
        self.env = env;
        self
    }
}

/// An incompatible or absent contract cannot be used as a staging target.
async fn check_delegated_module(
    options: &TransactionOptions,
    address: AccountAddress,
) -> CliTypedResult<()> {
    use aptos_api_types::AptosErrorCode;
    use aptos_rest_client::error::RestError;
    use aptos_transaction_simulation_session::Session;
    use aptos_types::state_store::{state_key::StateKey, TStateView};
    use move_binary_format::access::ModuleAccess;

    let bytes = if let Some(path) = &options.session {
        Session::load(path)?
            .state_store()
            .get_state_value_bytes(&StateKey::module(
                &address,
                ident_str!("delegated_packages"),
            ))
            .map_err(|e| {
                CliError::UnexpectedError(format!("Failed to read delegated module: {e}"))
            })?
    } else {
        match options
            .rest_client()?
            .get_account_module_bcs(address, "delegated_packages")
            .await
        {
            Ok(response) => Some(response.into_inner()),
            Err(RestError::Api(response))
                if response.error.error_code == AptosErrorCode::ModuleNotFound =>
            {
                None
            },
            Err(error) => return Err(CliError::from(error)),
        }
    };
    let unsupported = || {
        CliError::CommandArgumentError(format!(
        "Unsupported delegated_packages module at {address}: expected the stage, cleanup and finalize entry functions"
    ))
    };
    let bytes = bytes.ok_or_else(unsupported)?;
    let module =
        move_binary_format::CompiledModule::deserialize(&bytes).map_err(|_| unsupported())?;
    let entries: std::collections::HashSet<_> = module
        .function_defs()
        .iter()
        .filter(|def| def.is_entry)
        .map(|def| {
            module
                .identifier_at(module.function_handle_at(def.function).name)
                .as_str()
        })
        .collect();
    if ![
        "open_staging",
        "stage_chunks",
        "cleanup_staging",
        "finalize_to_account",
        "finalize_to_object",
        "finalize_upgrade",
    ]
    .into_iter()
    .all(|entry| entries.contains(entry))
    {
        return Err(unsupported());
    }
    if module.self_id().address() != &address {
        return Err(unsupported());
    }
    Ok(())
}

#[derive(Deserialize)]
struct DelegatedStagingAreaLayout {
    publisher: AccountAddress,
    generation: AccountAddress,
    chunks: DelegatedTableWithLengthLayout,
    keys: Vec<u64>,
}

#[derive(Deserialize)]
struct DelegatedTableWithLengthLayout {
    inner: DelegatedTableLayout,
    length: u64,
}

#[derive(Deserialize)]
struct DelegatedTableLayout {
    handle: AccountAddress,
}

/// Reject a different publisher, out-of-package positions, duplicated keys, or an invalid length.
fn staged_positions(
    area: &DelegatedStagingAreaLayout,
    publisher: AccountAddress,
    package: &PositionalChunks,
) -> CliTypedResult<Vec<(u64, u16, u16)>> {
    let invalid = |reason: &str| {
        CliError::CommandArgumentError(format!(
            "Cannot resume delegated staging: {reason}. Clean the area explicitly to start again."
        ))
    };
    if area.publisher != publisher {
        return Err(invalid("the staged publisher differs"));
    }
    if area.chunks.length != area.keys.len() as u64 {
        return Err(invalid("staged table length differs from its key list"));
    }
    let mut seen = HashSet::new();
    area.keys
        .iter()
        .map(|key| {
            let (part, index) =
                chunk_position(*key).ok_or_else(|| invalid("invalid chunk position"))?;
            if !seen.insert((part, index)) || package.get(part, index).is_none() {
                return Err(invalid("duplicate or unexpected chunk position"));
            }
            Ok((*key, part, index))
        })
        .collect()
}

fn check_staged_bytes(
    package: &PositionalChunks,
    part: u16,
    index: u16,
    bytes: &[u8],
) -> CliTypedResult<()> {
    if package
        .get(part, index)
        .is_some_and(|chunk| chunk.bytes == bytes)
    {
        Ok(())
    } else {
        Err(CliError::CommandArgumentError(format!(
            "Staged bytes at ({part}, {index}) differ from this package; clean the area explicitly to start again"
        )))
    }
}

/// Read the stager's resource and staged table items; no missing or corrupt entry is reused.
async fn read_delegated_staging(
    options: &TransactionOptions,
    module: AccountAddress,
    stager: AccountAddress,
    publisher: AccountAddress,
    package: &PositionalChunks,
) -> CliTypedResult<Option<(AccountAddress, HashSet<(u16, u16)>)>> {
    use aptos_api_types::AptosErrorCode;
    use aptos_rest_client::error::RestError;
    use aptos_transaction_simulation_session::Session;
    use aptos_types::state_store::{state_key::StateKey, TStateView};
    use move_core_types::language_storage::StructTag;
    let name = format!("{module}::delegated_packages::StagingArea");
    if let Some(path) = &options.session {
        let key = StateKey::resource(
            &stager,
            &StructTag::from_str(&name).map_err(|e| CliError::UnexpectedError(e.to_string()))?,
        )
        .map_err(|e| CliError::UnexpectedError(e.to_string()))?;
        let session = Session::load(path)?;
        let store = session.state_store();
        let Some(bytes) = store
            .get_state_value_bytes(&key)
            .map_err(|e| CliError::UnexpectedError(format!("Failed to read staging area: {e}")))?
        else {
            return Ok(None);
        };
        let area: DelegatedStagingAreaLayout = bcs::from_bytes(&bytes).map_err(|e| {
            CliError::UnexpectedError(format!("Invalid delegated staging area: {e}"))
        })?;
        let positions = staged_positions(&area, publisher, package)?;
        let mut staged = HashSet::new();
        for (key, part, index) in positions {
            let item = store
                .get_state_value_bytes(&StateKey::table_item(
                    &aptos_types::state_store::table::TableHandle(area.chunks.inner.handle),
                    &bcs::to_bytes(&key)?,
                ))
                .map_err(|e| {
                    CliError::UnexpectedError(format!("Failed to read staged chunk: {e}"))
                })?
                .ok_or_else(|| {
                    CliError::UnexpectedError(format!(
                        "Staged chunk at ({part}, {index}) is missing"
                    ))
                })?;
            let bytes: Vec<u8> = bcs::from_bytes(&item)
                .map_err(|e| CliError::UnexpectedError(format!("Invalid staged chunk: {e}")))?;
            check_staged_bytes(package, part, index, &bytes)?;
            staged.insert((part, index));
        }
        Ok(Some((area.generation, staged)))
    } else {
        let client = options.rest_client()?;
        let response = match client
            .get_account_resource_bcs::<DelegatedStagingAreaLayout>(stager, &name)
            .await
        {
            Ok(response) => response,
            Err(RestError::Api(response))
                if response.error.error_code == AptosErrorCode::ResourceNotFound =>
            {
                return Ok(None);
            },
            Err(error) => return Err(CliError::from(error)),
        };
        // Read every chunk from the same ledger snapshot as A's key list. Another process may
        // stage or clean up concurrently; never mix a current resource with older/newer items.
        let version = response.state().version;
        let area = response.into_inner();
        let positions = staged_positions(&area, publisher, package)?;
        let mut staged = HashSet::new();
        for (key, part, index) in positions {
            let bytes = client
                .get_table_item_bcs_at_version::<String, Vec<u8>>(
                    area.chunks.inner.handle,
                    "u64",
                    "vector<u8>",
                    key.to_string(),
                    version,
                )
                .await
                .map_err(|e| {
                    CliError::ApiError(format!(
                        "Failed to read staged chunk at ({part}, {index}): {e}"
                    ))
                })?
                .into_inner();
            check_staged_bytes(package, part, index, &bytes)?;
            staged.insert((part, index));
        }
        Ok(Some((area.generation, staged)))
    }
}

// A pending, failed, or discarded transaction is not proof of commitment.
fn committed(
    outcome: CliTypedResult<aptos_cli_common::TransactionSummary>,
) -> CliTypedResult<aptos_cli_common::TransactionSummary> {
    let summary = outcome?;
    if summary.success != Some(true) {
        return Err(CliError::SimulationError(summary.vm_status.unwrap_or_else(
            || "Staging transaction was not confirmed successful".into(),
        )));
    }
    Ok(summary)
}

async fn submit_stage_batches(
    payloads: Vec<TransactionPayload>,
    options: &TransactionOptions,
    env: &crate::MoveEnv,
    parallelism: usize,
) -> CliTypedResult<usize> {
    let total = payloads.len();
    let mut submitted = 0;
    for group in payloads.chunks(parallelism) {
        let outcomes = crate::dispatch_transactions(group.to_vec(), options, env).await?;
        // The submitter may stop early. Never treat missing outcomes as success.
        let incomplete = outcomes.len() != group.len();
        let mut failure = None;
        for outcome in outcomes {
            if let Err(error) = committed(outcome) {
                failure.get_or_insert(error);
            }
        }
        if let Some(error) = failure {
            return Err(error);
        }
        if incomplete {
            return Err(CliError::UnexpectedError(
                "Some staging transactions were not submitted".into(),
            ));
        }
        submitted += group.len();
    }
    debug_assert_eq!(submitted, total);
    Ok(submitted)
}

#[async_trait::async_trait]
impl aptos_cli_common::CliCommand<String> for StageDelegatedPackage {
    fn command_name(&self) -> &'static str {
        "StageDelegatedPackage"
    }

    async fn execute(self) -> CliTypedResult<String> {
        use aptos_cli_common::ReplayProtectionType;
        use aptos_framework::chunked_publish::PositionalChunks;
        let options = &self.txn_options;
        // Offline simulation cannot attest to the chain's nonce feature flag.
        let offline =
            options.session.is_some() || options.local || options.benchmark || options.profile_gas;
        let enabled = if offline {
            false
        } else {
            chain_orderless_enabled(options).await?
        };
        let mode = resolve_stage_mode(self.stage_mode, enabled, offline)?;
        if !(1..=CHUNK_SIZE_IN_BYTES).contains(&self.chunk_size) {
            return Err(CliError::CommandArgumentError(format!(
                "--chunk-size must be between 1 and {CHUNK_SIZE_IN_BYTES}"
            )));
        }
        let expected_destination = check_destination(
            self.destination,
            self.expected_publisher_sequence,
            self.object_address,
            self.publisher,
        )?;
        check_delegated_module(options, self.module_address).await?;
        let package = crate::commands::build_package_options(
            &self.move_options,
            &self.included_artifacts,
            &self.env,
        )
        .map_err(|e| CliError::MoveCompilationError(format!("{e:#}")))?;
        let metadata = bcs::to_bytes(&package.extract_metadata()?)?;
        let code = package.extract_code();
        check_compiled_destination(&code, expected_destination)?;
        let chunks = PositionalChunks::new(metadata, code, self.chunk_size);
        let stager = options.get_public_key_and_address()?.1;
        let staged = read_delegated_staging(
            options,
            self.module_address,
            stager,
            self.publisher,
            &chunks,
        )
        .await?;
        let generation = staged
            .as_ref()
            .map(|(id, _)| *id)
            .unwrap_or_else(|| AccountAddress::new(rand::random()));
        let staged_positions = staged
            .as_ref()
            .map(|(_, positions)| positions.clone())
            .unwrap_or_default();
        let missing = chunks.missing(&staged_positions);
        let payloads = stage_chunks(self.module_address, generation, missing, self.chunk_size);
        let mut txn_options = options.clone();
        // Open in its own A-signed committed transaction before any chunk can be sent.
        if staged.is_none() {
            txn_options.replay_protection_type = ReplayProtectionType::Seqnum;
            committed(
                crate::dispatch_transaction(
                    open_staging(self.module_address, self.publisher, generation),
                    &txn_options,
                    &self.env,
                )
                .await,
            )?;
        }
        txn_options.replay_protection_type = match mode {
            StageMode::Nonce => ReplayProtectionType::Nonce,
            StageMode::Seqnum => ReplayProtectionType::Seqnum,
            StageMode::Auto => unreachable!("mode is resolved"),
        };
        let count = submit_stage_batches(
            payloads,
            &txn_options,
            &self.env,
            usize::from(self.parallelism),
        )
        .await?;
        Ok(format!("Staged {count} chunk transactions under the signing account for publisher {}. Finalization requires the publisher's signature.", self.publisher))
    }
}

/// The separate B-signed transaction consuming exactly A's already staged package.
#[derive(clap::Parser)]
pub struct FinalizeDelegatedPackage {
    #[clap(long, value_parser = aptos_cli_common::load_account_arg)]
    module_address: AccountAddress,
    #[clap(long, value_parser = aptos_cli_common::load_account_arg)]
    stager: AccountAddress,
    #[clap(long, value_enum, default_value = "account")]
    destination: DelegatedDestination,
    #[clap(long, value_parser = aptos_cli_common::load_account_arg)]
    object_address: Option<AccountAddress>,
    /// For a new object: the publisher's current on-chain sequence number + 1.
    #[clap(long)]
    expected_publisher_sequence: Option<u64>,
    #[clap(flatten)]
    move_options: crate::move_types::MovePackageOptions,
    #[clap(flatten)]
    included_artifacts: crate::commands::IncludedArtifactsArgs,
    #[clap(flatten)]
    txn_options: TransactionOptions,
    #[clap(long, default_value_t = CHUNK_SIZE_IN_BYTES)]
    chunk_size: usize,
    #[clap(skip)]
    pub env: std::sync::Arc<crate::MoveEnv>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum DelegatedDestination {
    Account,
    NewObject,
    Upgrade,
}

impl crate::WithMoveEnv for FinalizeDelegatedPackage {
    fn attach_env(mut self, env: std::sync::Arc<crate::MoveEnv>) -> Self {
        self.env = env;
        self
    }
}

#[async_trait::async_trait]
impl aptos_cli_common::CliCommand<String> for FinalizeDelegatedPackage {
    fn command_name(&self) -> &'static str {
        "FinalizeDelegatedPackage"
    }

    async fn execute(self) -> CliTypedResult<String> {
        use aptos_cli_common::ReplayProtectionType;
        let options = &self.txn_options;
        if !(1..=CHUNK_SIZE_IN_BYTES).contains(&self.chunk_size) {
            return Err(CliError::CommandArgumentError(format!(
                "--chunk-size must be between 1 and {CHUNK_SIZE_IN_BYTES}"
            )));
        }
        let publisher = options.get_public_key_and_address()?.1;
        let expected_destination = check_destination(
            self.destination,
            self.expected_publisher_sequence,
            self.object_address,
            publisher,
        )?;
        if let Some(expected) = self.expected_publisher_sequence {
            let current = publisher_sequence_number(options, publisher).await?;
            if current.checked_add(1) != Some(expected) {
                return Err(CliError::CommandArgumentError(format!(
                    "Publisher sequence changed: expected next object seed {expected}, current sequence {current}. Staging remains intact."
                )));
            }
        }
        check_delegated_module(options, self.module_address).await?;
        let package = crate::commands::build_package_options(
            &self.move_options,
            &self.included_artifacts,
            &self.env,
        )
        .map_err(|e| CliError::MoveCompilationError(format!("{e:#}")))?;
        let metadata = bcs::to_bytes(&package.extract_metadata()?)?;
        let code = package.extract_code();
        check_compiled_destination(&code, expected_destination)?;
        let digest = package_digest(&metadata, &code)?;
        let chunks = PositionalChunks::new(metadata, code, self.chunk_size);
        let (_, staged) = read_delegated_staging(
            options,
            self.module_address,
            self.stager,
            publisher,
            &chunks,
        )
        .await?
        .ok_or_else(|| {
            CliError::CommandArgumentError(format!("No delegated staging area at {}", self.stager))
        })?;
        if staged.len() != chunks.chunks.len() {
            return Err(CliError::CommandArgumentError(format!(
                "Delegated package is incomplete: {} of {} chunks staged",
                staged.len(),
                chunks.chunks.len()
            )));
        }
        let payload = match self.destination {
            DelegatedDestination::Account => finalize_to_account(
                self.module_address,
                self.stager,
                &chunks.chunk_counts,
                digest,
            ),
            DelegatedDestination::NewObject => finalize_to_object(
                self.module_address,
                self.stager,
                &chunks.chunk_counts,
                digest,
            ),
            DelegatedDestination::Upgrade => finalize_upgrade(
                self.module_address,
                self.stager,
                &chunks.chunk_counts,
                digest,
                self.object_address.expect("checked above"),
            ),
        };
        let mut final_options = options.clone();
        final_options.replay_protection_type = ReplayProtectionType::Seqnum;
        committed(crate::dispatch_transaction(payload, &final_options, &self.env).await)?;
        Ok(format!(
            "Publisher {publisher} verified and published package staged by {}",
            self.stager
        ))
    }
}

/// Require A and B to agree on the account, the exact new object seed or the upgrade object.
fn check_destination(
    destination: DelegatedDestination,
    expected_sequence: Option<u64>,
    object: Option<AccountAddress>,
    publisher: AccountAddress,
) -> CliTypedResult<AccountAddress> {
    match (destination, object) {
        (DelegatedDestination::Upgrade, Some(address)) => {
            validate_destination_sequence(destination, expected_sequence, publisher)?;
            Ok(address)
        },
        (DelegatedDestination::Upgrade, None) => Err(CliError::CommandArgumentError(
            "--object-address is required to upgrade a code object".into(),
        )),
        (_, Some(_)) => Err(CliError::CommandArgumentError(
            "--object-address is only for --destination upgrade".into(),
        )),
        (_, None) => Ok(
            validate_destination_sequence(destination, expected_sequence, publisher)?
                .unwrap_or(publisher),
        ),
    }
}

/// The expected object seed is the publisher's current sequence number plus one.
fn validate_destination_sequence(
    destination: DelegatedDestination,
    expected: Option<u64>,
    publisher: AccountAddress,
) -> CliTypedResult<Option<AccountAddress>> {
    match (destination, expected) {
        (DelegatedDestination::NewObject, Some(sequence)) if sequence > 0 => Ok(Some(
            aptos_types::object_address::create_object_code_deployment_address(publisher, sequence),
        )),
        (DelegatedDestination::NewObject, _) => Err(CliError::CommandArgumentError(
            "--expected-publisher-sequence (current sequence + 1) is required for a new object"
                .into(),
        )),
        (_, Some(_)) => Err(CliError::CommandArgumentError(
            "--expected-publisher-sequence is only valid with --destination new-object".into(),
        )),
        (_, None) => Ok(None),
    }
}

async fn publisher_sequence_number(
    options: &TransactionOptions,
    publisher: AccountAddress,
) -> CliTypedResult<u64> {
    use aptos_transaction_simulation::SimulationStateStore;
    use aptos_transaction_simulation_session::Session;
    use aptos_types::account_config::AccountResource;
    match &options.session {
        Some(path) => Ok(Session::load(path)?
            .state_store()
            .get_resource::<AccountResource>(publisher)?
            .map_or(0, |account| account.sequence_number)),
        None => aptos_cli_common::get_sequence_number(&options.rest_client()?, publisher).await,
    }
}

/// A cleanup must remove entries or the area; otherwise repeating it cannot make progress.
fn check_cleanup_progress(
    before: usize,
    after: Option<(AccountAddress, usize)>,
    generation: AccountAddress,
) -> CliTypedResult<()> {
    if after.is_some_and(|(id, remaining)| id != generation || remaining >= before) {
        return Err(CliError::UnexpectedError(
            "Delegated cleanup did not reduce the number of staged chunks".into(),
        ));
    }
    Ok(())
}

/// Discard an incomplete delegated package. Only its stager's signer can do this.
#[derive(clap::Parser)]
pub struct CleanupDelegatedStaging {
    #[clap(long, value_parser = aptos_cli_common::load_account_arg)]
    module_address: AccountAddress,
    #[clap(flatten)]
    txn_options: TransactionOptions,
    #[clap(skip)]
    pub env: std::sync::Arc<crate::MoveEnv>,
}

impl crate::WithMoveEnv for CleanupDelegatedStaging {
    fn attach_env(mut self, env: std::sync::Arc<crate::MoveEnv>) -> Self {
        self.env = env;
        self
    }
}

#[async_trait::async_trait]
impl aptos_cli_common::CliCommand<String> for CleanupDelegatedStaging {
    fn command_name(&self) -> &'static str {
        "CleanupDelegatedStaging"
    }

    async fn execute(self) -> CliTypedResult<String> {
        use aptos_cli_common::ReplayProtectionType;
        check_delegated_module(&self.txn_options, self.module_address).await?;
        let mut options = self.txn_options;
        options.replay_protection_type = ReplayProtectionType::Seqnum;
        let stager = options.get_public_key_and_address()?.1;
        let mut before = delegated_staging_key_count(&options, self.module_address, stager).await?;
        let Some((generation, mut remaining)) = before else {
            return Ok("No delegated staging area to clean".into());
        };
        while before.is_some() {
            committed(
                crate::dispatch_transaction(
                    cleanup_staging(self.module_address, generation),
                    &options,
                    &self.env,
                )
                .await,
            )?;
            let after = delegated_staging_key_count(&options, self.module_address, stager).await?;
            check_cleanup_progress(remaining, after, generation)?;
            before = after;
            if let Some((_, next)) = after {
                remaining = next;
            }
        }
        Ok("Signing stager's delegated staging area cleaned up".into())
    }
}

/// Read only the number of remaining chunks; cleanup is allowed for stale or unknown packages.
async fn delegated_staging_key_count(
    options: &TransactionOptions,
    module: AccountAddress,
    stager: AccountAddress,
) -> CliTypedResult<Option<(AccountAddress, usize)>> {
    use aptos_api_types::AptosErrorCode;
    use aptos_rest_client::error::RestError;
    use aptos_transaction_simulation_session::Session;
    use aptos_types::state_store::{state_key::StateKey, TStateView};
    use move_core_types::language_storage::StructTag;

    let name = format!("{module}::delegated_packages::StagingArea");
    if let Some(path) = &options.session {
        let tag =
            StructTag::from_str(&name).map_err(|e| CliError::UnexpectedError(e.to_string()))?;
        let key = StateKey::resource(&stager, &tag)
            .map_err(|e| CliError::UnexpectedError(e.to_string()))?;
        let bytes = Session::load(path)?
            .state_store()
            .get_state_value_bytes(&key)
            .map_err(|e| CliError::UnexpectedError(format!("Failed to read staging area: {e}")))?;
        bytes
            .map(|raw| {
                bcs::from_bytes::<DelegatedStagingAreaLayout>(&raw)
                    .map(|area| (area.generation, area.keys.len()))
                    .map_err(|e| {
                        CliError::UnexpectedError(format!("Invalid delegated staging area: {e}"))
                    })
            })
            .transpose()
    } else {
        match options
            .rest_client()?
            .get_account_resource_bcs::<DelegatedStagingAreaLayout>(stager, &name)
            .await
        {
            Ok(response) => {
                let area = response.into_inner();
                Ok(Some((area.generation, area.keys.len())))
            },
            Err(RestError::Api(response))
                if response.error.error_code == AptosErrorCode::ResourceNotFound =>
            {
                Ok(None)
            },
            Err(error) => Err(CliError::from(error)),
        }
    }
}
