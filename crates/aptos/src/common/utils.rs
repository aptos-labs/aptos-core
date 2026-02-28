// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// ── Re-exports from aptos-cli-common ──
//
// These utility functions were previously duplicated locally. They now live in
// `aptos-cli-common` and are re-exported here so that existing `use
// crate::common::utils::...` paths continue to work.
use crate::common::types::{account_address_from_public_key, CliError, CliTypedResult};
use aptos_build_info::build_information;
pub use aptos_cli_common::{
    append_file_extension, check_if_file_exists, create_dir_if_not_exist, current_dir,
    deserialize_address_str, deserialize_material_with_prefix, dir_default_to_current,
    explorer_account_link, explorer_transaction_link, fund_account, get_account,
    get_account_with_state, get_auth_key, get_sequence_number, parse_json_file, parse_map,
    prompt_yes, prompt_yes_with_override, read_dir_files, read_from_file, read_line,
    serialize_address_str, serialize_material_with_prefix, start_logger, strip_private_key_prefix,
    to_common_result, to_common_success_result, view_json_option_str, wait_for_transactions,
    write_to_file, write_to_file_with_opts, write_to_user_only_file,
};
use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey};
use aptos_keygen::KeyGen;
use aptos_rest_client::Client;
use aptos_types::{
    account_address::create_multisig_account_address,
    chain_id::ChainId,
    on_chain_config::{FeatureFlag, Features},
};
use move_core_types::language_storage::CORE_CODE_ADDRESS;
use std::collections::BTreeMap;

/// Build information for the full Aptos CLI binary.
///
/// This uses the `build_information!()` macro from `aptos-build-info` which
/// captures git revision, build profile, etc. The `aptos-cli-common` crate has a
/// simpler version that only reports the package version.
pub fn cli_build_information() -> BTreeMap<String, String> {
    build_information!()
}

/// Retrieves the value of the specified feature flag from the rest client.
pub async fn get_feature_flag(client: &Client, flag: FeatureFlag) -> CliTypedResult<bool> {
    let features = client
        .get_account_resource_bcs::<Features>(CORE_CODE_ADDRESS, "0x1::features::Features")
        .await?
        .into_inner();
    Ok(features.is_enabled(flag))
}

/// Retrieves the chain id from the rest client.
pub async fn chain_id(rest_client: &Client) -> CliTypedResult<ChainId> {
    let state = rest_client
        .get_ledger_information()
        .await
        .map_err(|err| CliError::ApiError(err.to_string()))?
        .into_inner();
    Ok(ChainId::new(state.chain_id))
}

/// Generate a vanity account for Ed25519 single signer scheme, either standard or multisig.
///
/// The default authentication key for an Ed25519 account is the same as the account address. Hence
/// for a standard account, this function generates Ed25519 private keys until finding one that has
/// an authentication key (account address) that begins with the given vanity prefix.
///
/// For a multisig account, this function generates private keys until finding one that can create
/// a multisig account with the given vanity prefix as its first transaction (sequence number 0).
///
/// Note that while a valid hex string must have an even number of characters, a vanity prefix can
/// have an odd number of characters since account addresses are human-readable.
///
/// `vanity_prefix_ref` is a reference to a hex string vanity prefix, optionally prefixed with "0x".
/// For example "0xaceface" or "d00d".
pub fn generate_vanity_account_ed25519(
    vanity_prefix_ref: &str,
    vanity_postfix_ref: &str,
    multisig: bool,
) -> CliTypedResult<Ed25519PrivateKey> {
    let vanity_prefix_ref = vanity_prefix_ref
        .strip_prefix("0x")
        .unwrap_or(vanity_prefix_ref); // Optionally strip leading 0x from input string.
    if vanity_prefix_ref.starts_with('0') {
        // because of `AccountAddress::short_str_lossless()` `...trim_start_matches('0')...`
        return Err(CliError::CommandArgumentError(
            "The vanity prefix must not start with 0".to_owned(),
        ));
    }

    let mut to_check_if_is_hex = String::from(vanity_prefix_ref) + vanity_postfix_ref;
    // If an odd number of characters append a 0 for verifying that prefix contains valid hex.
    if to_check_if_is_hex.len() % 2 != 0 {
        to_check_if_is_hex += "0"
    };
    hex::decode(to_check_if_is_hex).  // Check that the vanity prefix can be decoded into hex.
        map_err(|error| CliError::CommandArgumentError(format!(
            "The vanity prefix/postfix could not be decoded to hex: {}", error)))?;
    let mut key_generator = KeyGen::from_os_rng(); // Get random key generator.
    loop {
        // Generate new keys until finding a match against the vanity prefix.
        let private_key = key_generator.generate_ed25519_private_key();
        let mut account_address =
            account_address_from_public_key(&Ed25519PublicKey::from(&private_key));
        if multisig {
            account_address = create_multisig_account_address(account_address, 0)
        };
        let addr = account_address.short_str_lossless();
        if addr.starts_with(vanity_prefix_ref) && addr.ends_with(vanity_postfix_ref) {
            return Ok(private_key);
        };
    }
}
