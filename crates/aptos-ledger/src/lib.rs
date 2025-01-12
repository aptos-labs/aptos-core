// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! # aptos-ledger
//!
//! `aptos-ledger` provides convenience methods to communicate with the Aptos app on ledger

#![deny(missing_docs)]

use aptos_crypto::ed25519::Ed25519Signature;
pub use aptos_crypto::{ed25519::Ed25519PublicKey, ValidCryptoMaterialStringExt};
pub use aptos_types::{
    account_address::AccountAddress, transaction::authenticator::AuthenticationKey,
};
use hex::encode;
use ledger_apdu::APDUCommand;
use ledger_transport_hid::{hidapi::HidApi, LedgerHIDError, TransportNativeHID};
use std::{
    collections::BTreeMap,
    fmt,
    fmt::{Debug, Display},
    ops::Range,
    str,
    string::ToString,
};
use thiserror::Error;

/// Derivation path template
/// A piece of data which tells a wallet how to derive a specific key within a tree of keys
/// 637 is the key for Aptos
pub const DERIVATION_PATH: &str = "m/44'/637'/{index}'/0'/0'";

const CLA_APTOS: u8 = 0x5B; // Aptos CLA Instruction class
const INS_GET_VERSION: u8 = 0x03; // Get version instruction code
const INS_GET_APP_NAME: u8 = 0x04; // Get app name instruction code
const INS_GET_PUB_KEY: u8 = 0x05; // Get public key instruction code
const INS_SIGN_TXN: u8 = 0x06; // Sign the transaction
const APDU_CODE_SUCCESS: u16 = 36864; // Success code for transport.exchange

const MAX_APDU_LEN: usize = 255;
const P1_NON_CONFIRM: u8 = 0x00;
const P1_CONFIRM: u8 = 0x01;
const P1_START: u8 = 0x00;
const P2_MORE: u8 = 0x80;
const P2_LAST: u8 = 0x00;

#[derive(Debug, Error)]
/// Aptos Ledger Error
pub enum AptosLedgerError {
    /// Error when trying to open a connection to the Ledger device
    #[error("Device not found")]
    DeviceNotFound,

    /// Error when communicating with Aptos app on Ledger
    #[error("Error - {0}")]
    AptosError(AptosLedgerStatusCode),

    /// Unexpected error, the `Option<u16>` is the retcode received from ledger transport
    #[error("Unexpected Error: {0} (StatusCode {1:?})")]
    UnexpectedError(String, Option<u16>),
}

impl From<LedgerHIDError> for AptosLedgerError {
    fn from(e: LedgerHIDError) -> Self {
        AptosLedgerError::UnexpectedError(e.to_string(), None)
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(u16)]
/// Status code returned when communicating with ledger
/// Most Aptos ones defined here <https://github.com/aptos-labs/ledger-app-aptos/blob/main/doc/COMMANDS.md#status-words>
/// Some of the ledger status code defined here - <https://www.eftlab.com/knowledge-base/complete-list-of-apdu-responses>
pub enum AptosLedgerStatusCode {
    // Aptos ledger app related status code
    /// Rejected by user
    Deny = 0x6985,

    /// Either P1 or P2 is incorrect
    WrongPip2 = 0x6A86,

    /// Lc or minimum APDU length is incorrect
    WrongDataLength = 0x6A87,

    /// No command exists with INS
    InsNotSupported = 0x6D00,

    /// Bad CLA used for this application
    ClaNotSupported = 0x6E00,

    /// Wrong response length (buffer size problem)
    WrongResponseLength = 0xB000,

    /// BIP32 path conversion to string failed
    DisplayBip32PathFail = 0xB001,

    /// Address conversion to string failed
    DisplayAddressFail = 0xB002,

    /// Amount conversion to string failed
    DisplayAmountFail = 0xB003,

    /// Wrong raw transaction length
    WrongTxnLength = 0xB004,

    /// Failed to parse raw transaction
    TxnParsingFail = 0xB005,

    /// Failed to compute hash digest of raw transaction
    TxnHashFail = 0xB006,

    /// Security issue with bad state
    BadState = 0xB007,

    /// Signature of raw transaction failed
    SignatureFail = 0xB008,

    /// Success
    Success = 0x9000,

    // Ledger device related general status code
    /// There are more status code, but we only list the most common ones
    /// Ledger device is locked
    LedgerLocked = 0x5515,

    /// Aptos ledger app is not opened
    AppNotOpen = 0x6E01,

    /// Self Defined status code for Unknown code
    /// Unknown status code
    Unknown = 0x0000,
}

impl AptosLedgerStatusCode {
    fn description(&self) -> &str {
        match self {
            AptosLedgerStatusCode::Deny => "Request rejected by user",
            AptosLedgerStatusCode::WrongPip2 => "Wrong P1 or P2",
            AptosLedgerStatusCode::WrongDataLength => "Wrong data length",
            AptosLedgerStatusCode::InsNotSupported => "Ins(Instruction) not supported",
            AptosLedgerStatusCode::ClaNotSupported => "Cla not supported",
            AptosLedgerStatusCode::WrongResponseLength => "Wrong response length",
            AptosLedgerStatusCode::DisplayBip32PathFail => "BIP32 path conversion to string failed",
            AptosLedgerStatusCode::DisplayAddressFail => "Address conversion to string failed",
            AptosLedgerStatusCode::DisplayAmountFail => "Amount conversion to string failed",
            AptosLedgerStatusCode::WrongTxnLength => "Wrong raw transaction length",
            AptosLedgerStatusCode::TxnParsingFail => "Failed to parse raw transaction",
            AptosLedgerStatusCode::TxnHashFail => {
                "Failed to compute hash digest of raw transaction"
            },
            AptosLedgerStatusCode::BadState => "Security issue with bad state",
            AptosLedgerStatusCode::SignatureFail => "Signature of raw transaction failed",
            AptosLedgerStatusCode::Success => "Success",
            AptosLedgerStatusCode::LedgerLocked => "Ledger device is locked",
            AptosLedgerStatusCode::AppNotOpen => "Aptos ledger app is not opened",
            AptosLedgerStatusCode::Unknown => "Unknown status code",
        }
    }

    fn map_status_code(status_code: u16) -> AptosLedgerStatusCode {
        match status_code {
            0x6985 => AptosLedgerStatusCode::Deny,
            0x6A86 => AptosLedgerStatusCode::WrongPip2,
            0x6A87 => AptosLedgerStatusCode::WrongDataLength,
            0x6D00 => AptosLedgerStatusCode::InsNotSupported,
            0x6E00 => AptosLedgerStatusCode::ClaNotSupported,
            0xB000 => AptosLedgerStatusCode::WrongResponseLength,
            0xB001 => AptosLedgerStatusCode::DisplayBip32PathFail,
            0xB002 => AptosLedgerStatusCode::DisplayAddressFail,
            0xB003 => AptosLedgerStatusCode::DisplayAmountFail,
            0xB004 => AptosLedgerStatusCode::WrongTxnLength,
            0xB005 => AptosLedgerStatusCode::TxnParsingFail,
            0xB006 => AptosLedgerStatusCode::TxnHashFail,
            0xB007 => AptosLedgerStatusCode::BadState,
            0xB008 => AptosLedgerStatusCode::SignatureFail,
            0x9000 => AptosLedgerStatusCode::Success,
            0x5515 => AptosLedgerStatusCode::LedgerLocked,
            0x6E01 => AptosLedgerStatusCode::AppNotOpen,
            _ => AptosLedgerStatusCode::Unknown,
        }
    }
}

impl Display for AptosLedgerStatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (0x{:x})", self.description(), *self as u16)
    }
}

/// Aptos version in format major.minor.patch
#[derive(Debug)]
pub struct Version {
    major: u8,
    minor: u8,
    patch: u8,
}

impl Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Validate the input derivation path to check if it's valid
pub fn validate_derivation_path(input: &str) -> bool {
    let prefix = "m/44'/637'/";
    let suffix = "'";

    if input.starts_with(prefix) && input.ends_with(suffix) {
        let inner_input = &input[prefix.len()..input.len()];

        // Sample: 0'/0'/0'
        let sections: Vec<&str> = inner_input.split('/').collect();
        if sections.len() != 3 {
            return false;
        }

        for section in sections {
            if !section.ends_with(suffix) {
                return false;
            }

            let section_value = &section.trim_end_matches('\'');
            if section_value.parse::<u32>().is_ok() {
                continue;
            }
            return false;
        }

        return true;
    }
    false
}

/// Returns the current version of the Aptos app on Ledger
pub fn get_app_version() -> Result<Version, AptosLedgerError> {
    // Open connection to ledger
    let transport = open_ledger_transport()?;

    match transport.exchange(&APDUCommand {
        cla: CLA_APTOS,
        ins: INS_GET_VERSION,
        p1: P1_NON_CONFIRM,
        p2: P2_LAST,
        data: vec![],
    }) {
        Ok(response) => {
            // Received response from Ledger
            if response.retcode() == APDU_CODE_SUCCESS {
                let major = response.data()[0];
                let minor = response.data()[1];
                let patch = response.data()[2];
                Ok(Version {
                    major,
                    minor,
                    patch,
                })
            } else {
                let error_code = AptosLedgerStatusCode::map_status_code(response.retcode());
                Err(AptosLedgerError::AptosError(error_code))
            }
        },
        Err(err) => Err(AptosLedgerError::from(err)),
    }
}

/// Returns the official app name register in Ledger
pub fn get_app_name() -> Result<String, AptosLedgerError> {
    // Open connection to ledger
    let transport = open_ledger_transport()?;

    match transport.exchange(&APDUCommand {
        cla: CLA_APTOS,
        ins: INS_GET_APP_NAME,
        p1: P1_NON_CONFIRM,
        p2: P2_LAST,
        data: vec![],
    }) {
        Ok(response) => {
            if response.retcode() == APDU_CODE_SUCCESS {
                let app_name = match str::from_utf8(response.data()) {
                    Ok(v) => v,
                    Err(e) => return Err(AptosLedgerError::UnexpectedError(e.to_string(), None)),
                };
                Ok(app_name.to_string())
            } else {
                let error_code = AptosLedgerStatusCode::map_status_code(response.retcode());
                Err(AptosLedgerError::AptosError(error_code))
            }
        },
        Err(err) => Err(AptosLedgerError::from(err)),
    }
}

/// Returns the batch/HashMap of the accounts for the account index in index_range
/// Note: We only allow a range of 10 for performance purpose
///
/// # Arguments
///
/// * `index_range` - start(inclusive) - end(exclusive) acounts, that you want to fetch, if None default to 0-10
pub fn fetch_batch_accounts(
    index_range: Option<Range<u32>>,
) -> Result<BTreeMap<String, AccountAddress>, AptosLedgerError> {
    let range = if let Some(range) = index_range {
        range
    } else {
        0..10
    };

    // Make sure the range is within 10 counts
    if range.end - range.start > 10 {
        return Err(AptosLedgerError::UnexpectedError(
            "Unexpected Error: Make sure the range is less than or equal to 10".to_string(),
            None,
        ));
    }

    // Open connection to ledger
    let transport = open_ledger_transport()?;

    let mut accounts = BTreeMap::new();
    for i in range {
        let path = DERIVATION_PATH.replace("{index}", &i.to_string());
        let cdata = serialize_bip32(&path);

        match transport.exchange(&APDUCommand {
            cla: CLA_APTOS,
            ins: INS_GET_PUB_KEY,
            p1: P1_NON_CONFIRM,
            p2: P2_LAST,
            data: cdata,
        }) {
            Ok(response) => {
                // Got the response from ledger after user has confirmed on the ledger wallet
                if response.retcode() == APDU_CODE_SUCCESS {
                    // Extract the Public key from the response data
                    let mut offset = 0;
                    let response_buffer = response.data();
                    let pub_key_len: usize = (response_buffer[offset] - 1).into();
                    offset += 1;

                    // Skipping weird 0x04 - because of how the Aptos Ledger parse works when return pub key
                    offset += 1;

                    let pub_key_buffer = response_buffer[offset..offset + pub_key_len].to_vec();
                    let hex_string = encode(pub_key_buffer);
                    let public_key = match Ed25519PublicKey::from_encoded_string(&hex_string) {
                        Ok(pk) => Ok(pk),
                        Err(err) => Err(AptosLedgerError::UnexpectedError(
                            err.to_string(),
                            Some(response.retcode()),
                        )),
                    };
                    let account = account_address_from_public_key(&public_key?);
                    accounts.insert(path, account);
                } else {
                    let error_code = AptosLedgerStatusCode::map_status_code(response.retcode());
                    return Err(AptosLedgerError::AptosError(error_code));
                }
            },
            Err(err) => return Err(AptosLedgerError::from(err)),
        }
    }

    Ok(accounts)
}

/// Returns the public key of your Aptos account in Ledger device at index 0
///
/// # Arguments
///
/// * `display` - If true, the public key will be displayed on the Ledger device, and confirmation is needed
pub fn get_public_key(path: &str, display: bool) -> Result<Ed25519PublicKey, AptosLedgerError> {
    // Open connection to ledger
    let transport = open_ledger_transport()?;

    // Serialize the derivation path
    let cdata = serialize_bip32(path);

    // APDU command's instruction parameter 1 or p1
    let p1: u8 = match display {
        true => P1_CONFIRM,
        false => P1_NON_CONFIRM,
    };

    match transport.exchange(&APDUCommand {
        cla: CLA_APTOS,
        ins: INS_GET_PUB_KEY,
        p1,
        p2: 0,
        data: cdata,
    }) {
        Ok(response) => {
            // Got the response from ledger after user has confirmed on the ledger wallet
            if response.retcode() == APDU_CODE_SUCCESS {
                // Extract the Public key from the response data
                let mut offset = 0;
                let response_buffer = response.data();
                let pub_key_len: usize = (response_buffer[offset] - 1).into();
                offset += 1;

                // Skipping weird 0x04 - because of how the Aptos Ledger parse works when return pub key
                offset += 1;

                let pub_key_buffer = response_buffer[offset..offset + pub_key_len].to_vec();
                let hex_string = encode(pub_key_buffer);
                match Ed25519PublicKey::from_encoded_string(&hex_string) {
                    Ok(pk) => Ok(pk),
                    Err(err) => Err(AptosLedgerError::UnexpectedError(err.to_string(), None)),
                }
            } else {
                let error_code = AptosLedgerStatusCode::map_status_code(response.retcode());
                Err(AptosLedgerError::AptosError(error_code))
            }
        },
        Err(err) => Err(AptosLedgerError::from(err)),
    }
}

/// Returns the signed signature of the raw transaction user provided
///
/// # Arguments
///
/// * `path` - derivation path of the ledger account
/// * `raw_message` - the raw message that need to be signed
pub fn sign_message(path: &str, raw_message: &[u8]) -> Result<Ed25519Signature, AptosLedgerError> {
    // open connection to ledger
    let transport = open_ledger_transport()?;

    // Serialize the derivation path
    let derivation_path_bytes = serialize_bip32(path);

    // Send the derivation path over as first message
    let sign_start = transport.exchange(&APDUCommand {
        cla: CLA_APTOS,
        ins: INS_SIGN_TXN,
        p1: P1_START,
        p2: P2_MORE,
        data: derivation_path_bytes,
    });

    if let Err(err) = sign_start {
        return Err(AptosLedgerError::UnexpectedError(err.to_string(), None));
    }

    let chunks = raw_message.chunks(MAX_APDU_LEN);
    let chunks_count = chunks.len();

    for (i, chunk) in chunks.enumerate() {
        let is_last_chunk = chunks_count == i + 1;
        match transport.exchange(&APDUCommand {
            cla: CLA_APTOS,
            ins: INS_SIGN_TXN,
            p1: (i + 1) as u8,
            p2: if is_last_chunk { P2_LAST } else { P2_MORE },
            data: chunk.to_vec(),
        }) {
            Ok(response) => {
                // success response
                if response.retcode() == APDU_CODE_SUCCESS {
                    if is_last_chunk {
                        let response_buffer = response.data();

                        let signature_len: usize = response_buffer[0] as usize;
                        let signature_buffer = &response_buffer[1..1 + signature_len];
                        return Ed25519Signature::try_from(signature_buffer).map_err(|err| {
                            AptosLedgerError::UnexpectedError(err.to_string(), None)
                        });
                    }
                } else {
                    let error_code = AptosLedgerStatusCode::map_status_code(response.retcode());
                    return Err(AptosLedgerError::AptosError(error_code));
                }
            },
            Err(err) => return Err(AptosLedgerError::from(err)),
        };
    }
    Err(AptosLedgerError::UnexpectedError(
        "Unable to process request".to_string(),
        None,
    ))
}

/// This is the Rust version of the serialization of BIP32 from Petra Wallet
fn serialize_bip32(path: &str) -> Vec<u8> {
    let parts: Vec<u32> = path
        .split('/')
        .skip(1)
        .map(|part| {
            if let Some(part) = part.strip_suffix('\'') {
                part.parse::<u32>().unwrap() + 0x80000000
            } else {
                part.parse::<u32>().unwrap()
            }
        })
        .collect();

    let mut serialized = vec![0u8; 1 + parts.len() * 4];
    serialized[0] = parts.len() as u8;

    for (i, part) in parts.iter().enumerate() {
        serialized[(1 + i * 4)..(5 + i * 4)].copy_from_slice(&part.to_be_bytes());
    }

    serialized
}

fn open_ledger_transport() -> Result<TransportNativeHID, AptosLedgerError> {
    // open connection to ledger
    // NOTE: ledger has to be unlocked
    let hidapi = match HidApi::new() {
        Ok(hidapi) => hidapi,
        Err(_err) => return Err(AptosLedgerError::DeviceNotFound),
    };

    // Open transport to the first device
    let transport = match TransportNativeHID::new(&hidapi) {
        Ok(transport) => transport,
        Err(_err) => return Err(AptosLedgerError::DeviceNotFound),
    };

    Ok(transport)
}

fn account_address_from_public_key(public_key: &Ed25519PublicKey) -> AccountAddress {
    let auth_key = AuthenticationKey::ed25519(public_key);
    AccountAddress::new(*auth_key.account_address())
}
