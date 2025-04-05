// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::{EntryFunction, TransactionPayload};
use move_core_types::{account_address::AccountAddress, ident_str, language_storage::ModuleId};

/// The default address where the `large_packages.move` module is deployed.
/// This address is used on both mainnet and testnet.
pub const LARGE_PACKAGES_MODULE_ADDRESS: &str =
    "0x0e1ca3011bdd07246d4d16d909dbb2d6953a86c4735d5acf5865d962c630cce7";

/// The default chunk size for splitting code and metadata to fit within the transaction size limits.
pub const CHUNK_SIZE_IN_BYTES: usize = 55_000;

pub enum PublishType {
    AccountDeploy,
    ObjectDeploy,
    ObjectUpgrade,
}

pub fn chunk_package_and_create_payloads(
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
