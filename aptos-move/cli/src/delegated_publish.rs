// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_cli_common::{CliError, CliTypedResult, TransactionOptions};
use aptos_crypto::HashValue;
use aptos_framework::chunked_publish::{PositionedChunk, CHUNK_SIZE_IN_BYTES};
use aptos_types::{
    on_chain_config::Features,
    transaction::{EntryFunction, TransactionPayload},
};
use clap::ValueEnum;
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::IdentStr, language_storage::ModuleId,
};

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
) -> TransactionPayload {
    payload(address, ident_str!("open_staging"), vec![bcs::to_bytes(
        &publisher,
    )
    .expect("address serializes")])
}

/// Stage only the selected chunks (possibly the missing subset of a resumed upload).
/// Never appends a publish transaction: finalization is separately signed by B.
/// The limit is on raw chunk bytes per batch (not the serialized transaction size),
/// as in the existing positional splitter. The latter adds BCS and entry-function overhead.
pub(crate) fn stage_chunks(
    address: AccountAddress,
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
            result.push(stage_batch(address, &parts, &indices, &bytes));
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
        result.push(stage_batch(address, &parts, &indices, &bytes));
    }
    result
}

fn stage_batch(
    address: AccountAddress,
    parts: &[u16],
    indices: &[u16],
    bytes: &[Vec<u8>],
) -> TransactionPayload {
    payload(address, ident_str!("stage_chunks"), vec![
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
        let open = open_staging(module, publisher);
        let args = entry(&open, module, "open_staging").args();
        assert_eq!(args.len(), 1);
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
        let payloads = stage_chunks(module, package.chunks.clone(), 6);
        assert_eq!(payloads.len(), 3);
        let mut staged = vec![];
        for payload in &payloads {
            let args = entry(payload, module, "stage_chunks").args();
            assert_eq!(args.len(), 3);
            let parts: Vec<u16> = bcs::from_bytes(&args[0]).unwrap();
            let indices: Vec<u16> = bcs::from_bytes(&args[1]).unwrap();
            let bytes: Vec<Vec<u8>> = bcs::from_bytes(&args[2]).unwrap();
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
        assert!(stage_chunks(module, vec![], 6).is_empty());
        let staged_positions = [(0, 0), (1, 1)].into_iter().collect();
        let missing = package.missing(&staged_positions);
        let resumed = stage_chunks(module, missing.clone(), 6);
        let resumed_positions: Vec<(u16, u16)> = resumed
            .iter()
            .flat_map(|payload| {
                let args = entry(payload, module, "stage_chunks").args();
                let parts: Vec<u16> = bcs::from_bytes(&args[0]).unwrap();
                let indices: Vec<u16> = bcs::from_bytes(&args[1]).unwrap();
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
        let payloads = stage_chunks(module, chunks, CHUNK_SIZE_IN_BYTES);
        assert_eq!(payloads.len(), 2);
        for (payload, expected) in payloads.iter().zip([CHUNK_SIZE_IN_BYTES, 1]) {
            let args = entry(payload, module, "stage_chunks").args();
            let bytes: Vec<Vec<u8>> = bcs::from_bytes(&args[2]).unwrap();
            assert_eq!(bytes.iter().map(Vec::len).sum::<usize>(), expected);
        }
    }

    #[test]
    #[should_panic(expected = "chunk size must be in")]
    fn delegated_publish_rejects_oversized_batch_budget() {
        stage_chunks(address("0xcafe"), vec![], CHUNK_SIZE_IN_BYTES + 1);
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
