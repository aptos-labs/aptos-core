// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Utility functions for file I/O, prompting, account queries, explorer links,
//! and other common CLI operations.

use crate::{CliError, CliResult, CliTypedResult, GlobalConfig, Network, PromptOptions};
use aptos_crypto::{ValidCryptoMaterial, ValidCryptoMaterialStringExt};
use aptos_logger::{debug, Level};
use aptos_rest_client::{aptos_api_types::HashValue, Account, Client, FaucetClient, State};
use itertools::Itertools;
use move_core_types::account_address::AccountAddress;
use reqwest::Url;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    collections::BTreeMap,
    env,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
    time::{Duration, Instant, SystemTime},
};

/// Prompts for confirmation until a yes or no is given explicitly
pub fn prompt_yes(prompt: &str) -> bool {
    let mut result: Result<bool, ()> = Err(());

    // Read input until a yes or a no is given
    while result.is_err() {
        println!("{} [yes/no] >", prompt);
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            continue;
        }
        result = match input.trim().to_lowercase().as_str() {
            "yes" | "y" => Ok(true),
            "no" | "n" => Ok(false),
            _ => Err(()),
        };
    }
    result.unwrap()
}

/// Reads a line from input
pub fn read_line(input_name: &str) -> CliTypedResult<String> {
    let mut input_buf = String::new();
    let _ = std::io::stdin()
        .read_line(&mut input_buf)
        .map_err(|err| CliError::IO(input_name.to_string(), err))?;

    Ok(input_buf)
}

/// A result wrapper for displaying either a correct execution result or an error.
///
/// The purpose of this is to have a pretty easy to recognize JSON output format e.g.
///
/// {
///   "Result":{
///     "encoded":{ ... }
///   }
/// }
///
/// {
///   "Error":"Failed to run command"
/// }
///
#[derive(Debug, Serialize)]
enum ResultWrapper<T> {
    Result(T),
    Error(String),
}

impl<T> From<CliTypedResult<T>> for ResultWrapper<T> {
    fn from(result: CliTypedResult<T>) -> Self {
        match result {
            Ok(inner) => ResultWrapper::Result(inner),
            Err(inner) => ResultWrapper::Error(format!("{:#}", inner)),
        }
    }
}

/// For pretty printing outputs in JSON. You can opt out of printing the error as
/// JSON by setting `jsonify_error` to false.
///
/// If a telemetry callback has been registered via [`crate::register_telemetry`],
/// it will be invoked to report the command latency and outcome.
pub async fn to_common_result<T: Serialize>(
    command: &str,
    start_time: Instant,
    result: CliTypedResult<T>,
    jsonify_error: bool,
) -> CliResult {
    let latency = start_time.elapsed();

    // Report telemetry if a callback has been registered.
    if let Some(telemetry) = crate::telemetry_callback() {
        if !telemetry.is_disabled() {
            telemetry.send_event(command, latency.as_secs_f64(), result.is_ok());
        }
    }

    // Return early with a non JSON error if requested.
    if let Err(err) = &result {
        if !jsonify_error {
            return Err(format!("{:#}", err));
        }
    }

    let is_err = result.is_err();
    let result = ResultWrapper::<T>::from(result);
    let string = serde_json::to_string_pretty(&result).unwrap();
    if is_err {
        Err(string)
    } else {
        Ok(string)
    }
}

/// Convert any successful response to Success. If there is an error, show it as JSON
/// unless `jsonify_error` is false.
pub async fn to_common_success_result<T>(
    command: &str,
    start_time: Instant,
    result: CliTypedResult<T>,
    jsonify_error: bool,
) -> CliResult {
    to_common_result(
        command,
        start_time,
        result.map(|_| "Success"),
        jsonify_error,
    )
    .await
}

/// Checks if a file exists, being overridden by `PromptOptions`
pub fn check_if_file_exists(file: &Path, prompt_options: PromptOptions) -> CliTypedResult<()> {
    if file.exists() {
        prompt_yes_with_override(
            &format!(
                "{:?} already exists, are you sure you want to overwrite it?",
                file.as_os_str(),
            ),
            prompt_options,
        )?
    }

    Ok(())
}

pub fn prompt_yes_with_override(prompt: &str, prompt_options: PromptOptions) -> CliTypedResult<()> {
    if prompt_options.assume_no {
        return Err(CliError::AbortedError);
    } else if prompt_options.assume_yes {
        return Ok(());
    }

    let is_yes = if let Some(response) = GlobalConfig::load()?.get_default_prompt_response() {
        response
    } else {
        prompt_yes(prompt)
    };

    if is_yes {
        Ok(())
    } else {
        Err(CliError::AbortedError)
    }
}

/// Write a `&[u8]` to a file
pub fn write_to_file(path: &Path, name: &str, bytes: &[u8]) -> CliTypedResult<()> {
    write_to_file_with_opts(path, name, bytes, &mut OpenOptions::new())
}

/// Write a `&[u8]` to a file with the given options
pub fn write_to_file_with_opts(
    path: &Path,
    name: &str,
    bytes: &[u8],
    opts: &mut OpenOptions,
) -> CliTypedResult<()> {
    let mut file = opts
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(|e| CliError::IO(name.to_string(), e))?;
    file.write_all(bytes)
        .map_err(|e| CliError::IO(name.to_string(), e))
}

/// Write a User only read / write file
pub fn write_to_user_only_file(path: &Path, name: &str, bytes: &[u8]) -> CliTypedResult<()> {
    let mut opts = OpenOptions::new();
    #[cfg(unix)]
    opts.mode(0o600);
    write_to_file_with_opts(path, name, bytes, &mut opts)
}

pub fn read_from_file(path: &Path) -> CliTypedResult<Vec<u8>> {
    std::fs::read(path)
        .map_err(|e| CliError::UnableToReadFile(format!("{}", path.display()), e.to_string()))
}

/// Lists the content of a directory
pub fn read_dir_files(
    path: &Path,
    predicate: impl Fn(&Path) -> bool,
) -> CliTypedResult<Vec<PathBuf>> {
    let to_cli_err = |err| CliError::IO(path.display().to_string(), err);
    let mut result = vec![];
    for entry in std::fs::read_dir(path).map_err(to_cli_err)? {
        let path = entry.map_err(to_cli_err)?.path();
        if predicate(path.as_path()) {
            result.push(path)
        }
    }
    Ok(result)
}

pub fn create_dir_if_not_exist(dir: &Path) -> CliTypedResult<()> {
    // Check if the directory exists, if it's not a dir, it will also fail here
    if !dir.exists() || !dir.is_dir() {
        std::fs::create_dir_all(dir).map_err(|e| CliError::IO(dir.display().to_string(), e))?;
        debug!("Created {} folder", dir.display());
    } else {
        debug!("{} folder already exists", dir.display());
    }
    Ok(())
}

pub fn current_dir() -> CliTypedResult<PathBuf> {
    env::current_dir().map_err(|err| {
        CliError::UnexpectedError(format!("Failed to get current directory {}", err))
    })
}

pub fn dir_default_to_current(maybe_dir: Option<PathBuf>) -> CliTypedResult<PathBuf> {
    if let Some(dir) = maybe_dir {
        Ok(dir)
    } else {
        current_dir()
    }
}

/// Appends a file extension to a `Path` without overwriting the original extension.
pub fn append_file_extension(
    file: &Path,
    appended_extension: &'static str,
) -> CliTypedResult<PathBuf> {
    let extension = file
        .extension()
        .map(|extension| extension.to_str().unwrap_or_default());
    if let Some(extension) = extension {
        Ok(file.with_extension(extension.to_owned() + "." + appended_extension))
    } else {
        Ok(file.with_extension(appended_extension))
    }
}

pub fn start_logger(level: Level) {
    let mut logger = aptos_logger::Logger::new();
    logger.channel_size(1000).is_async(false).level(level);
    logger.build();
}

/// Error message for parsing a map
const PARSE_MAP_SYNTAX_MSG: &str = "Invalid syntax for map. Example: Name=Value,Name2=Value";

/// Parses an inline map of values
///
/// Example: Name=Value,Name2=Value
pub fn parse_map<K: FromStr + Ord, V: FromStr>(str: &str) -> anyhow::Result<BTreeMap<K, V>>
where
    K::Err: 'static + std::error::Error + Send + Sync,
    V::Err: 'static + std::error::Error + Send + Sync,
{
    let mut map = BTreeMap::new();

    // Split pairs by commas
    for pair in str.split_terminator(',') {
        // Split pairs by = then trim off any spacing
        let (first, second): (&str, &str) = pair
            .split_terminator('=')
            .collect_tuple()
            .ok_or_else(|| anyhow::Error::msg(PARSE_MAP_SYNTAX_MSG))?;
        let first = first.trim();
        let second = second.trim();
        if first.is_empty() || second.is_empty() {
            return Err(anyhow::Error::msg(PARSE_MAP_SYNTAX_MSG));
        }

        // At this point, we just give error messages appropriate to parsing
        let key: K = K::from_str(first)?;
        let value: V = V::from_str(second)?;
        map.insert(key, value);
    }
    Ok(map)
}

/// Retrieves account resource from the rest client
pub async fn get_account(client: &Client, address: AccountAddress) -> CliTypedResult<Account> {
    let account_response = client
        .get_account(address)
        .await
        .map_err(|err| CliError::ApiError(err.to_string()))?;
    Ok(account_response.into_inner())
}

/// Retrieves account resource from the rest client
pub async fn get_account_with_state(
    client: &Client,
    address: AccountAddress,
) -> CliTypedResult<(Account, State)> {
    let account_response = client
        .get_account(address)
        .await
        .map_err(|err| CliError::ApiError(err.to_string()))?;
    Ok(account_response.into_parts())
}

/// Retrieves sequence number from the rest client
pub async fn get_sequence_number(client: &Client, address: AccountAddress) -> CliTypedResult<u64> {
    Ok(get_account(client, address).await?.sequence_number)
}

/// Retrieves the auth key from the rest client
pub async fn get_auth_key(
    client: &Client,
    address: AccountAddress,
) -> CliTypedResult<aptos_types::transaction::authenticator::AuthenticationKey> {
    Ok(get_account(client, address).await?.authentication_key)
}

/// Fund account (and possibly create it) from a faucet. This function waits for the
/// transaction on behalf of the caller.
pub async fn fund_account(
    rest_client: Client,
    faucet_url: Url,
    faucet_auth_token: Option<&str>,
    address: AccountAddress,
    num_octas: u64,
) -> CliTypedResult<()> {
    let mut client = FaucetClient::new_from_rest_client(faucet_url, rest_client);
    if let Some(token) = faucet_auth_token {
        client = client.with_auth_token(token.to_string());
    }
    client
        .fund(address, num_octas)
        .await
        .map_err(|err| CliError::ApiError(format!("Faucet issue: {:#}", err)))
}

/// Wait for transactions, returning an error if any of them fail.
pub async fn wait_for_transactions(client: &Client, hashes: Vec<HashValue>) -> CliTypedResult<()> {
    let sys_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| CliError::UnexpectedError(e.to_string()))?
        .as_secs()
        + 30;
    for hash in hashes {
        client
            .wait_for_transaction_by_hash(
                hash.into(),
                sys_time,
                Some(Duration::from_secs(60)),
                None,
            )
            .await?;
    }
    Ok(())
}

pub fn explorer_account_link(hash: AccountAddress, network: Option<Network>) -> String {
    // For now, default to what the browser is already on, though the link could be wrong
    if let Some(network) = network {
        format!(
            "https://explorer.aptoslabs.com/account/{}?network={}",
            hash, network
        )
    } else {
        format!("https://explorer.aptoslabs.com/account/{}", hash)
    }
}

pub fn explorer_transaction_link(
    hash: aptos_crypto::HashValue,
    network: Option<Network>,
) -> String {
    // For now, default to what the browser is already on, though the link could be wrong
    if let Some(network) = network {
        format!(
            "https://explorer.aptoslabs.com/txn/{}?network={}",
            hash.to_hex_literal(),
            network
        )
    } else {
        format!(
            "https://explorer.aptoslabs.com/txn/{}",
            hash.to_hex_literal()
        )
    }
}

/// Strips the private key prefix for a given key string if it is AIP-80 compliant.
///
/// [Read about AIP-80](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-80.md)
pub fn strip_private_key_prefix(key: &str) -> CliTypedResult<&str> {
    let disabled_prefixes = ["secp256k1-priv-"];
    let enabled_prefixes = ["ed25519-priv-"];

    // Check for disabled prefixes first
    for prefix in disabled_prefixes {
        if key.starts_with(prefix) {
            return Err(CliError::UnexpectedError(format!(
                "Private key not supported. Cannot parse private key with '{}' prefix.",
                prefix
            )));
        }
    }

    // Try to strip enabled prefixes
    for prefix in enabled_prefixes {
        if let Some(stripped) = key.strip_prefix(prefix) {
            return Ok(stripped);
        }
    }

    // If no prefix is found, return the original key
    Ok(key)
}

pub fn serialize_address_str<S: Serializer>(
    addr: &Option<AccountAddress>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    if let Some(addr) = addr {
        serializer.serialize_some(&addr.to_standard_string())
    } else {
        serializer.serialize_none()
    }
}

pub fn deserialize_address_str<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<AccountAddress>, D::Error> {
    use serde::de::Error;

    // Deserialize the field as an Option<String>
    let opt: Option<String> = Option::deserialize(deserializer)?;

    // Transform Option<String> into Option<T>
    opt.map_or(Ok(None), |s| {
        AccountAddress::from_str(&s)
            .map(Some)
            .map_err(D::Error::custom)
    })
}

/// Serializes an [`ValidCryptoMaterial`] with a prefix AIP-80 prefix if present.
///
/// [Read about AIP-80](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-80.md)
pub fn serialize_material_with_prefix<S: Serializer, T: ValidCryptoMaterial>(
    material: &Option<T>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    use serde::ser::Error;

    if let Some(material) = material {
        serializer.serialize_some(
            &material
                .to_aip_80_string()
                .map_err(|err| S::Error::custom(err.to_string()))?,
        )
    } else {
        serializer.serialize_none()
    }
}

/// Deserializes an [`ValidCryptoMaterial`] with a prefix AIP-80 prefix if present.
///
/// [Read about AIP-80](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-80.md)
pub fn deserialize_material_with_prefix<'de, D: Deserializer<'de>, T: ValidCryptoMaterial>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    use serde::de::Error;

    // Deserialize the field as an Option<String>
    let opt: Option<String> = Option::deserialize(deserializer)?;

    // Transform Option<String> into Option<T>
    opt.map_or(Ok(None), |s| {
        T::from_encoded_string(&s)
            .map(Some)
            .map_err(D::Error::custom)
    })
}

/// Try parsing JSON in file at path into a specified type.
pub fn parse_json_file<T: for<'a> Deserialize<'a>>(path_ref: &Path) -> CliTypedResult<T> {
    serde_json::from_slice::<T>(&read_from_file(path_ref)?).map_err(|err| {
        CliError::UnableToReadFile(format!("{}", path_ref.display()), err.to_string())
    })
}

/// Convert a view function JSON field into a string option.
///
/// A view function JSON return represents an option via an inner JSON array titled `vec`.
pub fn view_json_option_str(option_ref: &serde_json::Value) -> CliTypedResult<Option<String>> {
    if let Some(vec_field) = option_ref.get("vec") {
        if let Some(vec_array) = vec_field.as_array() {
            if vec_array.is_empty() {
                Ok(None)
            } else if vec_array.len() > 1 {
                Err(CliError::UnexpectedError(format!(
                    "JSON `vec` array has more than one element: {:?}",
                    vec_array
                )))
            } else {
                let option_val_ref = &vec_array[0];
                if let Some(inner_str) = option_val_ref.as_str() {
                    Ok(Some(inner_str.to_string()))
                } else {
                    Err(CliError::UnexpectedError(format!(
                        "JSON option is not a string: {}",
                        option_val_ref
                    )))
                }
            }
        } else {
            Err(CliError::UnexpectedError(format!(
                "JSON `vec` field is not an array: {}",
                vec_field
            )))
        }
    } else {
        Err(CliError::UnexpectedError(format!(
            "JSON field does not have an inner `vec` field: {}",
            option_ref
        )))
    }
}

/// Returns build information for the CLI.
///
/// This is a simplified version that returns only the package version,
/// without depending on `aptos-build-info`.
pub fn cli_build_information() -> BTreeMap<String, String> {
    let mut info = BTreeMap::new();
    info.insert(
        "package_version".to_string(),
        env!("CARGO_PKG_VERSION").to_string(),
    );
    info
}
