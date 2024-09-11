// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::{EntryFunction, TransactionPayload};
use move_core_types::{account_address::AccountAddress, ident_str, language_storage::ModuleId};

pub const LARGE_PACKAGES_MODULE_ADDRESS: &str =
    "0xa29df848eebfe5d981f708c2a5b06d31af2be53bbd8ddc94c8523f4b903f7adb"; // mainnet and testnet

/// Maximum code & metadata chunk size to be included in a transaction
pub const MAX_CHUNK_SIZE_IN_BYTES: usize = 60_000;

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
) -> Vec<TransactionPayload> {
    // Chunk the metadata
    let mut metadata_chunks = create_chunks(metadata);
    // Separate last chunk for special handling
    let mut metadata_chunk = metadata_chunks.pop().expect("Metadata is required");

    let mut taken_size = metadata_chunk.len();
    let mut payloads = metadata_chunks
        .into_iter()
        .map(|chunk| large_packages_stage_code_chunk(chunk, vec![], vec![]))
        .collect::<Vec<_>>();

    let mut code_indices: Vec<u16> = vec![];
    let mut code_chunks: Vec<Vec<u8>> = vec![];

    for (idx, module_code) in package_code.into_iter().enumerate() {
        let chunked_module = create_chunks(module_code);
        for chunk in chunked_module {
            if taken_size + chunk.len() > MAX_CHUNK_SIZE_IN_BYTES {
                // Create a payload and reset accumulators
                let payload = large_packages_stage_code_chunk(
                    metadata_chunk,
                    code_indices.clone(),
                    code_chunks.clone(),
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
        ),
        PublishType::ObjectDeploy => large_packages_stage_code_chunk_and_publish_to_object(
            metadata_chunk,
            code_indices,
            code_chunks,
        ),
        PublishType::ObjectUpgrade => large_packages_stage_code_chunk_and_upgrade_object_code(
            metadata_chunk,
            code_indices,
            code_chunks,
            object_address,
        ),
    };
    payloads.push(payload);

    payloads
}

// Create chunks of data based on the defined maximum chunk size.
fn create_chunks(data: Vec<u8>) -> Vec<Vec<u8>> {
    data.chunks(MAX_CHUNK_SIZE_IN_BYTES)
        .map(|chunk| chunk.to_vec())
        .collect()
}

// Create a transaction payload for staging chunked data to the staging area.
fn large_packages_stage_code_chunk(
    metadata_chunk: Vec<u8>,
    code_indices: Vec<u16>,
    code_chunks: Vec<Vec<u8>>,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            AccountAddress::from_hex_literal(LARGE_PACKAGES_MODULE_ADDRESS).unwrap(),
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
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            AccountAddress::from_hex_literal(LARGE_PACKAGES_MODULE_ADDRESS).unwrap(),
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
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            AccountAddress::from_hex_literal(LARGE_PACKAGES_MODULE_ADDRESS).unwrap(),
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
    code_object: Option<AccountAddress>,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            AccountAddress::from_hex_literal(LARGE_PACKAGES_MODULE_ADDRESS).unwrap(),
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
pub fn large_packages_cleanup_staging_area() -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            AccountAddress::from_hex_literal(LARGE_PACKAGES_MODULE_ADDRESS).unwrap(),
            ident_str!("large_packages").to_owned(),
        ),
        ident_str!("cleanup_staging_area").to_owned(),
        vec![],
        vec![],
    ))
}
