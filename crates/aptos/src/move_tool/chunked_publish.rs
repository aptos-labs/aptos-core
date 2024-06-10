// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        types::{CliError, CliTypedResult, TransactionOptions, TransactionSummary},
        utils::prompt_yes_with_override,
    },
    move_tool::{ChunkedPublishPayloads, PackagePublicationData, MAX_PUBLISH_PACKAGE_SIZE},
};
use aptos_api_types::AptosErrorCode;
use aptos_framework::BuiltPackage;
use aptos_rest_client::{error::RestError, Client};
use aptos_types::transaction::{EntryFunction, TransactionPayload};
use colored::Colorize;
use move_core_types::{account_address::AccountAddress, ident_str, language_storage::ModuleId};

pub(crate) const LARGE_PACKAGES_MODULE_ADDRESS: &str =
    "0xa29df848eebfe5d981f708c2a5b06d31af2be53bbd8ddc94c8523f4b903f7adb"; // mainnet and testnet

/// These modes create a single transaction for publishing a package.
pub(crate) enum PackagePublishMode {
    AccountDeploy,
    ObjectDeploy,
    ObjectUpgrade,
}

/// These modes create multiple transactions for publishing a package.
pub(crate) enum ChunkedPackagePublishMode {
    AccountDeployChunked,
    ObjectDeployChunked,
    ObjectUpgradeChunked,
}

pub(crate) fn create_package_publication_data_from_built_package(
    package: BuiltPackage,
    package_publish_mode: PackagePublishMode,
    object_address: Option<AccountAddress>,
) -> CliTypedResult<PackagePublicationData> {
    let compiled_units = package.extract_code();
    let metadata = package.extract_metadata()?;
    let metadata_serialized = bcs::to_bytes(&metadata).expect("PackageMetadata has BCS");

    let payload = match package_publish_mode {
        PackagePublishMode::AccountDeploy => {
            aptos_cached_packages::aptos_stdlib::code_publish_package_txn(
                metadata_serialized.clone(),
                compiled_units.clone(),
            )
        },
        PackagePublishMode::ObjectDeploy => {
            aptos_cached_packages::aptos_stdlib::object_code_deployment_publish(
                metadata_serialized.clone(),
                compiled_units.clone(),
            )
        },
        PackagePublishMode::ObjectUpgrade => {
            aptos_cached_packages::aptos_stdlib::object_code_deployment_upgrade(
                metadata_serialized.clone(),
                compiled_units.clone(),
                object_address.expect("Object address must be provided for upgrading object code."),
            )
        },
    };

    Ok(PackagePublicationData {
        metadata_serialized,
        compiled_units,
        payload,
    })
}

pub(crate) async fn create_chunked_publish_payloads_from_built_package(
    package: BuiltPackage,
    chunked_package_publish_mode: ChunkedPackagePublishMode,
    object_address: Option<AccountAddress>,
) -> CliTypedResult<ChunkedPublishPayloads> {
    let compiled_units = package.extract_code();
    let metadata = package.extract_metadata()?;
    let metadata_serialized = bcs::to_bytes(&metadata).expect("PackageMetadata has BCS");

    let maybe_object_address =
        if let ChunkedPackagePublishMode::ObjectUpgradeChunked = chunked_package_publish_mode {
            object_address
        } else {
            None
        };

    let payloads = chunk_package_and_create_payloads(
        metadata_serialized,
        compiled_units,
        chunked_package_publish_mode,
        maybe_object_address,
    )
    .await;

    Ok(ChunkedPublishPayloads { payloads })
}

async fn chunk_package_and_create_payloads(
    metadata: Vec<u8>,
    package_code: Vec<Vec<u8>>,
    chunked_package_publish_mode: ChunkedPackagePublishMode,
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
            if taken_size + chunk.len() > MAX_PUBLISH_PACKAGE_SIZE {
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
    let payload = match chunked_package_publish_mode {
        ChunkedPackagePublishMode::AccountDeployChunked => {
            large_packages_stage_code_chunk_and_publish_to_account(
                metadata_chunk,
                code_indices,
                code_chunks,
            )
        },
        ChunkedPackagePublishMode::ObjectDeployChunked => {
            large_packages_stage_code_chunk_and_publish_to_object(
                metadata_chunk,
                code_indices,
                code_chunks,
            )
        },
        ChunkedPackagePublishMode::ObjectUpgradeChunked => {
            large_packages_stage_code_chunk_and_upgrade_object_code(
                metadata_chunk,
                code_indices,
                code_chunks,
                object_address,
            )
        },
    };
    payloads.push(payload);

    payloads
}

// Create chunks of data based on the defined maximum chunk size.
pub fn create_chunks(data: Vec<u8>) -> Vec<Vec<u8>> {
    data.chunks(MAX_PUBLISH_PACKAGE_SIZE)
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
pub(crate) fn large_packages_cleanup_staging_area() -> TransactionPayload {
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

async fn is_staging_area_empty(txn_options: &TransactionOptions) -> CliTypedResult<bool> {
    let url = txn_options.rest_options.url(&txn_options.profile_options)?;
    let client = Client::new(url);

    let staging_area_response = client
        .get_account_resource(
            txn_options.profile_options.account_address()?,
            &format!(
                "{}::large_packages::StagingArea",
                LARGE_PACKAGES_MODULE_ADDRESS
            ),
        )
        .await;

    match staging_area_response {
        Ok(response) => match response.into_inner() {
            Some(_) => Ok(false), // StagingArea is not empty
            None => Ok(true),     // TODO: determine which case this is
        },
        Err(RestError::Api(aptos_error_response))
            if aptos_error_response.error.error_code == AptosErrorCode::ResourceNotFound =>
        {
            Ok(true) // The resource doesn't exist
        },
        Err(rest_err) => Err(CliError::from(rest_err)),
    }
}

pub(crate) async fn submit_chunked_publish_transactions(
    payloads: Vec<TransactionPayload>,
    txn_options: &TransactionOptions,
) -> CliTypedResult<TransactionSummary> {
    let mut publishing_result = Err(CliError::UnexpectedError(
        "No payload provided for batch transaction run".to_string(),
    ));
    let payloads_length = payloads.len() as u64;
    let mut tx_hashes = vec![];

    let account_address = txn_options.profile_options.account_address()?;

    if !is_staging_area_empty(txn_options).await? {
        let message = format!(
            "The resource {}::large_packages::StagingArea under account {} is not empty.\
        \nThis may cause package publishing to fail if the data is unexpected. \
        \nUse the `aptos move clear-staging-area` command to clean up the `StagingArea` resource under the account.",
            LARGE_PACKAGES_MODULE_ADDRESS, account_address,
        )
        .bold();
        println!("{}", message);
        prompt_yes_with_override("Do you want to proceed?", txn_options.prompt_options)?;
    }

    for (idx, payload) in payloads.into_iter().enumerate() {
        println!("Transaction {} of {}", idx + 1, payloads_length);
        let result = txn_options
            .submit_transaction(payload)
            .await
            .map(TransactionSummary::from);

        match result {
            Ok(tx_summary) => {
                let tx_hash = tx_summary.transaction_hash.to_string();
                println!("Submitted Successfully ({})\n", &tx_hash);
                tx_hashes.push(tx_hash);
                publishing_result = Ok(tx_summary);
            },

            Err(e) => {
                println!("Caution: An error occurred while submitting chunked publish transactions. \
                \nDue to this error, there may be incomplete data left in the `StagingArea` resource. \
                \nThis could cause further errors if you attempt to run the chunked publish command again. \
                \nTo avoid this, use the `aptos move clear-staging-area` command to clean up the `StagingArea` resource under your account before retrying.");
                return Err(e);
            },
        }
    }

    println!(
        "{}",
        "All Transactions Submitted Successfully.".bold().green()
    );
    let tx_hash_formatted = format!(
        "Submitted Transactions:\n[\n    {}\n]",
        tx_hashes
            .iter()
            .map(|tx| format!("\"{}\"", tx))
            .collect::<Vec<_>>()
            .join(",\n    ")
    );
    println!("\n{}\n", tx_hash_formatted);
    publishing_result
}
