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

// const LARGE_PACKAGES_MODULE_ADDRESS: &'static str =
//     "0x1eca74e7baed8cfc36cd4f534019038f262bfa031cd931d80a2065c38366125b"; // mainnet
const LARGE_PACKAGES_MODULE_ADDRESS: &'static str =
    "0x4a96cb56a3c1169c8cbc065fb9ad4d6a27e230a7e37a9306075d71da63b13b37"; // testnet

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
        .map(|chunk| {
            large_packages_stage_code_chunk(chunk, vec![], vec![], false, false, false, None)
        })
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
                    false,
                    false,
                    false,
                    None,
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

    let (is_account_deploy, is_object_deploy, is_object_upgrade, object_address) =
        match chunked_package_publish_mode {
            ChunkedPackagePublishMode::AccountDeployChunked => (true, false, false, None),
            ChunkedPackagePublishMode::ObjectDeployChunked => (false, true, false, None),
            ChunkedPackagePublishMode::ObjectUpgradeChunked => (false, false, true, object_address),
        };

    let payload = large_packages_stage_code_chunk(
        metadata_chunk,
        code_indices,
        code_chunks,
        is_account_deploy,
        is_object_deploy,
        is_object_upgrade,
        object_address,
    );
    payloads.push(payload);

    payloads
}

// Create chunks of data based on the defined maximum chunk size.
pub fn create_chunks(data: Vec<u8>) -> Vec<Vec<u8>> {
    data.chunks(MAX_PUBLISH_PACKAGE_SIZE)
        .map(|chunk| chunk.to_vec())
        .collect()
}

// TODO: move to `aptos_cached_packages` when `large_packages` is included in the aptos framework
// Create a transaction payload for staging or publishing the large package.
fn large_packages_stage_code_chunk(
    metadata_chunk: Vec<u8>,
    code_indices: Vec<u16>,
    code_chunks: Vec<Vec<u8>>,
    is_account_deploy: bool,
    is_object_deploy: bool,
    is_object_upgrade: bool,
    code_object: Option<AccountAddress>,
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
            bcs::to_bytes(&is_account_deploy).unwrap(),
            bcs::to_bytes(&is_object_deploy).unwrap(),
            bcs::to_bytes(&is_object_upgrade).unwrap(),
            bcs::to_bytes(&code_object).unwrap(),
        ],
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
            "The resource {}::large_packages::StagingArea is not empty.\
        \nThis may cause package publishing to fail if the data is unexpected.",
            account_address
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
