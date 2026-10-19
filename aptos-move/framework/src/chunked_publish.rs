// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::{
    chain_id::ChainId,
    transaction::{EntryFunction, TransactionPayload},
};
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::IdentStr, language_storage::ModuleId,
};
use std::collections::HashSet;

/// Address of the `large_packages` module on mainnet, which doesn't have it at `0x7`
/// (aptos-experimental is never released to mainnet).
pub const LARGE_PACKAGES_PROD_MODULE_ADDRESS: &str =
    "0x0e1ca3011bdd07246d4d16d909dbb2d6953a86c4735d5acf5865d962c630cce7";

/// Address of the `large_packages` module on networks whose genesis includes aptos-experimental:
/// testnet, devnet and localnet.
pub const LARGE_PACKAGES_DEV_MODULE_ADDRESS: &str = "0x7";

/// The default chunk size for splitting code and metadata to fit within the transaction size limits.
pub const CHUNK_SIZE_IN_BYTES: usize = 55_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublishType {
    AccountDeploy,
    ObjectDeploy,
    ObjectUpgrade,
}

/// Entry functions of the `large_packages` module that a chunked publish stages through.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LargePackagesAbi {
    /// `stage_code_chunk*`, which every version of the module has. Chunks are appended in the order
    /// they are staged, so they have to be staged in order.
    Append,
    /// `stage_chunks*`. Every chunk carries its position, so chunks can be staged in any order and
    /// the module detects a chunk that is staged twice.
    Positional,
}

impl LargePackagesAbi {
    /// Determines which entry functions a `large_packages` module has from the names of its
    /// functions.
    pub fn from_function_names<'a>(mut names: impl Iterator<Item = &'a str>) -> Self {
        if names.any(|name| name == "stage_chunks") {
            Self::Positional
        } else {
            Self::Append
        }
    }
}

/// Returns the address of the `large_packages` module that chunked publishing uses by default.
pub fn default_large_packages_module_address(chain_id: &ChainId) -> &'static str {
    if chain_id.is_mainnet() {
        LARGE_PACKAGES_PROD_MODULE_ADDRESS
    } else {
        LARGE_PACKAGES_DEV_MODULE_ADDRESS
    }
}

/// Splits a package into chunks and creates the transactions that stage them through the
/// `large_packages` module, the last of which publishes the package.
pub fn chunk_package_and_create_payloads(
    metadata: Vec<u8>,
    package_code: Vec<Vec<u8>>,
    publish_type: PublishType,
    object_address: Option<AccountAddress>,
    large_packages_module_address: AccountAddress,
    chunk_size: usize,
    abi: LargePackagesAbi,
) -> Vec<TransactionPayload> {
    match abi {
        LargePackagesAbi::Append => create_append_payloads(
            metadata,
            package_code,
            publish_type,
            object_address,
            large_packages_module_address,
            chunk_size,
        ),
        LargePackagesAbi::Positional => create_positional_payloads(
            metadata,
            package_code,
            publish_type,
            object_address,
            large_packages_module_address,
            chunk_size,
        ),
    }
}

fn create_append_payloads(
    metadata: Vec<u8>,
    package_code: Vec<Vec<u8>>,
    publish_type: PublishType,
    object_address: Option<AccountAddress>,
    large_packages_module_address: AccountAddress,
    chunk_size: usize,
) -> Vec<TransactionPayload> {
    // Chunk the metadata
    let mut metadata_chunks = create_chunks(metadata, chunk_size);
    // Separate last chunk for special handling
    let mut metadata_chunk = metadata_chunks.pop().expect("Metadata is required");

    let mut taken_size = metadata_chunk.len();
    let mut payloads = metadata_chunks
        .into_iter()
        .map(|chunk| {
            large_packages_stage_code_chunk(chunk, vec![], vec![], large_packages_module_address)
        })
        .collect::<Vec<_>>();

    let mut code_indices: Vec<u16> = vec![];
    let mut code_chunks: Vec<Vec<u8>> = vec![];

    for (idx, module_code) in package_code.into_iter().enumerate() {
        let chunked_module = create_chunks(module_code, chunk_size);
        for chunk in chunked_module {
            if taken_size + chunk.len() > chunk_size {
                // Create a payload and reset accumulators
                let payload = large_packages_stage_code_chunk(
                    metadata_chunk,
                    code_indices.clone(),
                    code_chunks.clone(),
                    large_packages_module_address,
                );
                payloads.push(payload);

                metadata_chunk = vec![];
                code_indices.clear();
                code_chunks.clear();
                taken_size = 0;
            }

            code_indices.push(idx as u16);
            taken_size += chunk.len();
            code_chunks.push(chunk);
        }
    }

    // The final call includes staging the last metadata and code chunk, and then publishing or upgrading the package on-chain.
    let payload = match publish_type {
        PublishType::AccountDeploy => large_packages_stage_code_chunk_and_publish_to_account(
            metadata_chunk,
            code_indices,
            code_chunks,
            large_packages_module_address,
        ),
        PublishType::ObjectDeploy => large_packages_stage_code_chunk_and_publish_to_object(
            metadata_chunk,
            code_indices,
            code_chunks,
            large_packages_module_address,
        ),
        PublishType::ObjectUpgrade => large_packages_stage_code_chunk_and_upgrade_object_code(
            metadata_chunk,
            code_indices,
            code_chunks,
            object_address.expect("ObjectAddress is missing"),
            large_packages_module_address,
        ),
    };
    payloads.push(payload);

    payloads
}

// Create chunks of data based on the defined maximum chunk size.
fn create_chunks(data: Vec<u8>, chunk_size: usize) -> Vec<Vec<u8>> {
    data.chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect()
}

/// Chunks staged by one transaction of a positional chunked publish.
#[derive(Default)]
struct ChunkBatch {
    part_indices: Vec<u16>,
    chunk_indices: Vec<u16>,
    chunks: Vec<Vec<u8>>,
    size: usize,
}

impl ChunkBatch {
    fn into_args(self) -> Vec<Vec<u8>> {
        vec![
            bcs::to_bytes(&self.part_indices).expect("part indices serialize"),
            bcs::to_bytes(&self.chunk_indices).expect("chunk indices serialize"),
            bcs::to_bytes(&self.chunks).expect("chunks serialize"),
        ]
    }
}

/// A chunk of a package at its position, for [`LargePackagesAbi::Positional`]. Part 0 of a package is
/// its metadata and part `i + 1` is the code of module `i`; `index` is the chunk's index in its part.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PositionedChunk {
    pub part: u16,
    pub index: u16,
    pub bytes: Vec<u8>,
}

/// The key under which the `large_packages` module stores the chunk at `(part, index)`.
pub fn chunk_key(part: u16, index: u16) -> u64 {
    (u64::from(part) << 32) | u64::from(index)
}

/// The position `(part, index)` of the chunk stored under `key`, or `None` if `key` isn't the key of
/// any chunk that the `stage_chunks*` functions can stage.
pub fn chunk_position(key: u64) -> Option<(u16, u16)> {
    Some((
        u16::try_from(key >> 32).ok()?,
        u16::try_from(key & 0xFFFF_FFFF).ok()?,
    ))
}

/// All chunks of a package for [`LargePackagesAbi::Positional`].
#[derive(Clone, Debug)]
pub struct PositionalChunks {
    /// Every chunk, sorted by position, which is also the order in which a full publish stages them.
    pub chunks: Vec<PositionedChunk>,
    /// The number of chunks in each part, which the final transaction passes to the module.
    pub chunk_counts: Vec<u16>,
}

impl PositionalChunks {
    /// Splits the metadata and the code of the modules of a package into chunks of at most
    /// `chunk_size` bytes.
    pub fn new(metadata: Vec<u8>, package_code: Vec<Vec<u8>>, chunk_size: usize) -> Self {
        let parts: Vec<Vec<u8>> = std::iter::once(metadata).chain(package_code).collect();
        let chunk_counts = parts
            .iter()
            .map(|part| to_u16(part.len().div_ceil(chunk_size)))
            .collect();
        let chunks = parts
            .iter()
            .enumerate()
            .flat_map(|(part, bytes)| {
                bytes
                    .chunks(chunk_size)
                    .enumerate()
                    .map(move |(index, chunk)| PositionedChunk {
                        part: to_u16(part),
                        index: to_u16(index),
                        bytes: chunk.to_vec(),
                    })
            })
            .collect();
        Self {
            chunks,
            chunk_counts,
        }
    }

    /// The chunk at `(part, index)`, if the package has one.
    pub fn get(&self, part: u16, index: u16) -> Option<&PositionedChunk> {
        self.chunks
            .binary_search_by_key(&(part, index), |chunk| (chunk.part, chunk.index))
            .ok()
            .map(|position| &self.chunks[position])
    }

    /// The chunks that aren't at one of the `staged` positions, in staging order.
    pub fn missing(&self, staged: &HashSet<(u16, u16)>) -> Vec<PositionedChunk> {
        self.chunks
            .iter()
            .filter(|chunk| !staged.contains(&(chunk.part, chunk.index)))
            .cloned()
            .collect()
    }
}

// Creates payloads for the `stage_chunks*` functions that stage every chunk of a package.
fn create_positional_payloads(
    metadata: Vec<u8>,
    package_code: Vec<Vec<u8>>,
    publish_type: PublishType,
    object_address: Option<AccountAddress>,
    large_packages_module_address: AccountAddress,
    chunk_size: usize,
) -> Vec<TransactionPayload> {
    let package = PositionalChunks::new(metadata, package_code, chunk_size);
    create_positional_payloads_for(
        package.chunks,
        &package.chunk_counts,
        publish_type,
        object_address,
        large_packages_module_address,
        chunk_size,
    )
}

/// Creates the transactions that stage `chunks`, the last of which then publishes the package.
/// `chunk_counts` is the number of chunks in each part of the whole package. Chunks are packed into
/// transactions of at most `chunk_size` bytes.
///
/// `chunks` can be only some of the chunks of the package: to resume an earlier publish that did not
/// complete, the chunks that are already staged are left out.
pub fn create_positional_payloads_for(
    chunks: Vec<PositionedChunk>,
    chunk_counts: &[u16],
    publish_type: PublishType,
    object_address: Option<AccountAddress>,
    large_packages_module_address: AccountAddress,
    chunk_size: usize,
) -> Vec<TransactionPayload> {
    let mut batches = vec![ChunkBatch::default()];
    for chunk in chunks {
        if batches
            .last()
            .is_some_and(|batch| batch.size + chunk.bytes.len() > chunk_size)
        {
            batches.push(ChunkBatch::default());
        }
        let batch = batches.last_mut().expect("there is always a batch");
        batch.part_indices.push(chunk.part);
        batch.chunk_indices.push(chunk.index);
        batch.size += chunk.bytes.len();
        batch.chunks.push(chunk.bytes);
    }
    // The final transaction also carries the chunk counts, two bytes per part.
    let chunk_counts_size = std::mem::size_of_val(chunk_counts);
    if batches
        .last()
        .is_some_and(|batch| batch.size + chunk_counts_size > chunk_size)
    {
        batches.push(ChunkBatch::default());
    }

    let final_batch = batches.pop().expect("there is always a batch");
    let mut payloads: Vec<TransactionPayload> = batches
        .into_iter()
        .map(|batch| {
            large_packages_entry_function(
                large_packages_module_address,
                ident_str!("stage_chunks"),
                batch.into_args(),
            )
        })
        .collect();

    // The final transaction stages the last chunks, and then publishes or upgrades the package.
    let mut args = final_batch.into_args();
    args.push(bcs::to_bytes(&chunk_counts).expect("chunk counts serialize"));
    let function = match publish_type {
        PublishType::AccountDeploy => ident_str!("stage_chunks_and_publish_to_account"),
        PublishType::ObjectDeploy => ident_str!("stage_chunks_and_publish_to_object"),
        PublishType::ObjectUpgrade => {
            let code_object = object_address.expect("ObjectAddress is missing");
            args.push(bcs::to_bytes(&code_object).expect("object address serializes"));
            ident_str!("stage_chunks_and_upgrade_object_code")
        },
    };
    payloads.push(large_packages_entry_function(
        large_packages_module_address,
        function,
        args,
    ));
    payloads
}

fn to_u16(value: usize) -> u16 {
    u16::try_from(value).expect("package has too many modules or chunks to publish in chunks")
}

fn large_packages_entry_function(
    large_packages_module_address: AccountAddress,
    function: &IdentStr,
    args: Vec<Vec<u8>>,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            large_packages_module_address,
            ident_str!("large_packages").to_owned(),
        ),
        function.to_owned(),
        vec![],
        args,
    ))
}

// Create a transaction payload for staging chunked data to the staging area.
fn large_packages_stage_code_chunk(
    metadata_chunk: Vec<u8>,
    code_indices: Vec<u16>,
    code_chunks: Vec<Vec<u8>>,
    large_packages_module_address: AccountAddress,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            large_packages_module_address,
            ident_str!("large_packages").to_owned(),
        ),
        ident_str!("stage_code_chunk").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&metadata_chunk).unwrap(),
            bcs::to_bytes(&code_indices).unwrap(),
            bcs::to_bytes(&code_chunks).unwrap(),
        ],
    ))
}

// Create a transaction payload for staging chunked data and finally publishing the package to an account.
fn large_packages_stage_code_chunk_and_publish_to_account(
    metadata_chunk: Vec<u8>,
    code_indices: Vec<u16>,
    code_chunks: Vec<Vec<u8>>,
    large_packages_module_address: AccountAddress,
) -> TransactionPayload {
    // TODO[Orderless]: Change this to payload v2 format.
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            large_packages_module_address,
            ident_str!("large_packages").to_owned(),
        ),
        ident_str!("stage_code_chunk_and_publish_to_account").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&metadata_chunk).unwrap(),
            bcs::to_bytes(&code_indices).unwrap(),
            bcs::to_bytes(&code_chunks).unwrap(),
        ],
    ))
}

// Create a transaction payload for staging chunked data and finally publishing the package to an object.
fn large_packages_stage_code_chunk_and_publish_to_object(
    metadata_chunk: Vec<u8>,
    code_indices: Vec<u16>,
    code_chunks: Vec<Vec<u8>>,
    large_packages_module_address: AccountAddress,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            large_packages_module_address,
            ident_str!("large_packages").to_owned(),
        ),
        ident_str!("stage_code_chunk_and_publish_to_object").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&metadata_chunk).unwrap(),
            bcs::to_bytes(&code_indices).unwrap(),
            bcs::to_bytes(&code_chunks).unwrap(),
        ],
    ))
}

// Create a transaction payload for staging chunked data and finally upgrading the object package.
fn large_packages_stage_code_chunk_and_upgrade_object_code(
    metadata_chunk: Vec<u8>,
    code_indices: Vec<u16>,
    code_chunks: Vec<Vec<u8>>,
    code_object: AccountAddress,
    large_packages_module_address: AccountAddress,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            large_packages_module_address,
            ident_str!("large_packages").to_owned(),
        ),
        ident_str!("stage_code_chunk_and_upgrade_object_code").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&metadata_chunk).unwrap(),
            bcs::to_bytes(&code_indices).unwrap(),
            bcs::to_bytes(&code_chunks).unwrap(),
            bcs::to_bytes(&code_object).unwrap(),
        ],
    ))
}

// Cleanup account's `StagingArea` resource.
pub fn large_packages_cleanup_staging_area(
    large_packages_module_address: AccountAddress,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            large_packages_module_address,
            ident_str!("large_packages").to_owned(),
        ),
        ident_str!("cleanup_staging_area").to_owned(),
        vec![],
        vec![],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::chain_id::NamedChain;

    #[test]
    fn default_large_packages_module_address_by_network() {
        assert_eq!(
            default_large_packages_module_address(&ChainId::mainnet()),
            LARGE_PACKAGES_PROD_MODULE_ADDRESS
        );
        // Every other network gets the module at 0x7 from its genesis.
        for chain_id in [
            ChainId::testnet(),
            ChainId::new(NamedChain::DEVNET.id()),
            ChainId::test(),
        ] {
            assert_eq!(
                default_large_packages_module_address(&chain_id),
                LARGE_PACKAGES_DEV_MODULE_ADDRESS
            );
        }
    }

    #[test]
    fn abi_from_function_names() {
        assert_eq!(
            LargePackagesAbi::from_function_names(["stage_code_chunk", "stage_chunks"].into_iter()),
            LargePackagesAbi::Positional
        );
        assert_eq!(
            LargePackagesAbi::from_function_names(
                ["stage_code_chunk", "cleanup_staging_area"].into_iter()
            ),
            LargePackagesAbi::Append
        );
    }

    fn entry_function(payload: &TransactionPayload) -> &EntryFunction {
        let TransactionPayload::EntryFunction(entry_function) = payload else {
            panic!("expected an entry function payload, got {:?}", payload);
        };
        entry_function
    }

    /// The (part, index, chunk) triples that a positional payload stages.
    fn staged_chunks(entry_function: &EntryFunction) -> Vec<(u16, u16, Vec<u8>)> {
        let args = entry_function.args();
        let part_indices: Vec<u16> = bcs::from_bytes(&args[0]).unwrap();
        let chunk_indices: Vec<u16> = bcs::from_bytes(&args[1]).unwrap();
        let chunks: Vec<Vec<u8>> = bcs::from_bytes(&args[2]).unwrap();
        part_indices
            .into_iter()
            .zip(chunk_indices)
            .zip(chunks)
            .map(|((part, index), chunk)| (part, index, chunk))
            .collect()
    }

    #[test]
    fn positional_payloads_stage_every_chunk_once() {
        let metadata: Vec<u8> = (0..25).collect();
        let code: Vec<Vec<u8>> = vec![(100..130).collect(), vec![7; 5]];
        let payloads = chunk_package_and_create_payloads(
            metadata.clone(),
            code.clone(),
            PublishType::AccountDeploy,
            None,
            AccountAddress::ONE,
            10,
            LargePackagesAbi::Positional,
        );

        let (last, staging) = payloads.split_last().unwrap();
        for payload in staging {
            assert_eq!(entry_function(payload).function().as_str(), "stage_chunks");
        }
        let last = entry_function(last);
        assert_eq!(
            last.function().as_str(),
            "stage_chunks_and_publish_to_account"
        );
        let chunk_counts: Vec<u16> = bcs::from_bytes(&last.args()[3]).unwrap();
        assert_eq!(chunk_counts, vec![3, 3, 1]);

        // Every transaction stays within the chunk size, and every chunk is staged exactly once.
        let mut parts: Vec<Vec<(u16, Vec<u8>)>> = vec![vec![]; 3];
        for payload in &payloads {
            let chunks = staged_chunks(entry_function(payload));
            let size: usize = chunks.iter().map(|(_, _, chunk)| chunk.len()).sum();
            assert!(size <= 10);
            for (part, index, chunk) in chunks {
                parts[part as usize].push((index, chunk));
            }
        }
        let reassembled: Vec<Vec<u8>> = parts
            .into_iter()
            .map(|mut chunks| {
                chunks.sort();
                let indices: Vec<u16> = chunks.iter().map(|(index, _)| *index).collect();
                assert_eq!(indices, (0..indices.len() as u16).collect::<Vec<_>>());
                chunks.into_iter().flat_map(|(_, chunk)| chunk).collect()
            })
            .collect();
        assert_eq!(reassembled, [vec![metadata], code].concat());
    }

    #[test]
    fn positional_payloads_upgrade_object() {
        let object = AccountAddress::from_hex_literal("0xcafe").unwrap();
        let payloads = chunk_package_and_create_payloads(
            vec![1; 4],
            vec![vec![2; 4]],
            PublishType::ObjectUpgrade,
            Some(object),
            AccountAddress::ONE,
            20,
            LargePackagesAbi::Positional,
        );

        // Both parts and the chunk counts fit in a single transaction.
        assert_eq!(payloads.len(), 1);
        let last = entry_function(&payloads[0]);
        assert_eq!(
            last.function().as_str(),
            "stage_chunks_and_upgrade_object_code"
        );
        assert_eq!(staged_chunks(last).len(), 2);
        assert_eq!(
            bcs::from_bytes::<AccountAddress>(&last.args()[4]).unwrap(),
            object
        );
    }

    #[test]
    fn positional_payloads_leave_room_for_chunk_counts() {
        // The last chunk fills its transaction, so the chunk counts get a transaction of their own.
        let payloads = chunk_package_and_create_payloads(
            vec![1; 10],
            vec![vec![2; 10]],
            PublishType::AccountDeploy,
            None,
            AccountAddress::ONE,
            10,
            LargePackagesAbi::Positional,
        );

        assert_eq!(payloads.len(), 3);
        assert!(staged_chunks(entry_function(&payloads[2])).is_empty());
    }

    #[test]
    fn chunk_keys_round_trip() {
        for (part, index) in [(0, 0), (1, 0), (0, 1), (7, 300), (u16::MAX, u16::MAX)] {
            assert_eq!(chunk_position(chunk_key(part, index)), Some((part, index)));
        }
        // The part is in the upper 32 bits and the index in the lower ones, as in the Move module.
        assert_eq!(chunk_key(1, 0), 1 << 32);
        assert_eq!(chunk_key(0, 5), 5);
        // Keys that no `stage_chunks*` call can produce.
        assert_eq!(chunk_position(1 << 48), None);
        assert_eq!(chunk_position((1 << 32) | (1 << 16)), None);
    }

    #[test]
    fn positional_chunks_are_sorted_and_counted() {
        let package = PositionalChunks::new(
            (0..25).collect(),
            vec![(100..130).collect(), vec![7; 5]],
            10,
        );

        assert_eq!(package.chunk_counts, vec![3, 3, 1]);
        let positions: Vec<(u16, u16)> = package
            .chunks
            .iter()
            .map(|chunk| (chunk.part, chunk.index))
            .collect();
        assert_eq!(positions, vec![
            (0, 0),
            (0, 1),
            (0, 2),
            (1, 0),
            (1, 1),
            (1, 2),
            (2, 0)
        ]);
        assert_eq!(
            package.get(0, 2).unwrap().bytes,
            (20..25).collect::<Vec<u8>>()
        );
        assert_eq!(package.get(2, 0).unwrap().bytes, vec![7; 5]);
        assert!(package.get(2, 1).is_none());
        assert!(package.get(3, 0).is_none());
    }

    #[test]
    fn missing_chunks_leave_out_staged_ones() {
        let package = PositionalChunks::new(vec![1; 25], vec![vec![2; 25]], 10);
        let staged: HashSet<(u16, u16)> = [(0, 0), (0, 2), (1, 1)].into_iter().collect();

        let positions: Vec<(u16, u16)> = package
            .missing(&staged)
            .iter()
            .map(|chunk| (chunk.part, chunk.index))
            .collect();
        assert_eq!(positions, vec![(0, 1), (1, 0), (1, 2)]);

        let everything: HashSet<(u16, u16)> = package
            .chunks
            .iter()
            .map(|chunk| (chunk.part, chunk.index))
            .collect();
        assert!(package.missing(&everything).is_empty());
    }

    #[test]
    fn resumed_payloads_stage_only_the_given_chunks() {
        let package = PositionalChunks::new(vec![1; 25], vec![vec![2; 25]], 10);
        let staged: HashSet<(u16, u16)> = [(0, 0), (0, 1), (1, 0)].into_iter().collect();

        let payloads = create_positional_payloads_for(
            package.missing(&staged),
            &package.chunk_counts,
            PublishType::AccountDeploy,
            None,
            AccountAddress::ONE,
            10,
        );

        // The missing chunks take 5, 10 and 5 bytes: one transaction each, and the last publishes.
        assert_eq!(payloads.len(), 3);
        let positions: Vec<(u16, u16)> = payloads
            .iter()
            .flat_map(|payload| staged_chunks(entry_function(payload)))
            .map(|(part, index, _)| (part, index))
            .collect();
        assert_eq!(positions, vec![(0, 2), (1, 1), (1, 2)]);
        let last = entry_function(payloads.last().unwrap());
        assert_eq!(
            last.function().as_str(),
            "stage_chunks_and_publish_to_account"
        );
        // The counts are those of the whole package, not of what is staged now.
        assert_eq!(bcs::from_bytes::<Vec<u16>>(&last.args()[3]).unwrap(), vec![
            3, 3
        ]);
    }

    #[test]
    fn resumed_payloads_with_everything_staged_only_publish() {
        let package = PositionalChunks::new(vec![1; 25], vec![vec![2; 25]], 10);
        let object = AccountAddress::from_hex_literal("0xcafe").unwrap();

        let payloads = create_positional_payloads_for(
            vec![],
            &package.chunk_counts,
            PublishType::ObjectUpgrade,
            Some(object),
            AccountAddress::ONE,
            10,
        );

        assert_eq!(payloads.len(), 1);
        let last = entry_function(&payloads[0]);
        assert_eq!(
            last.function().as_str(),
            "stage_chunks_and_upgrade_object_code"
        );
        assert!(staged_chunks(last).is_empty());
        assert_eq!(bcs::from_bytes::<Vec<u16>>(&last.args()[3]).unwrap(), vec![
            3, 3
        ]);
        assert_eq!(
            bcs::from_bytes::<AccountAddress>(&last.args()[4]).unwrap(),
            object
        );
    }
}
