// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use hex::encode;
use ledger_apdu::APDUCommand;
use ledger_transport_hid::{hidapi::HidApi, TransportNativeHID};
use std::str;

const DERIVATIVE_PATH: &str = "m/44'/637'/0'/0'/0'"; // TODO: Add support for multiple index

const CLA_APTOS: u8 = 0x5B; // Aptos CLA Instruction class
const INS_GET_VERSION: u8 = 0x03; // Get version instruction code
const INS_GET_APP_NAME: u8 = 0x04; // Get app name instruction code
const INS_GET_PUB_KEY: u8 = 0x05; // Get public key instruction code
const INS_SIGN_TXN: u8 = 0x06; // Sign the transaction
const APDU_CODE_SUCCESS: u16 = 36864; // success code for transport.exchange

const MAX_APDU_LEN: usize = 255;
const P1_NON_CONFIRM: u8 = 0x00;
const P1_CONFIRM: u8 = 0x01;
const P1_START: u8 = 0x00;
const P2_MORE: u8 = 0x80;
const P2_LAST: u8 = 0x00;

#[derive(Debug)]
pub enum AptosLedgerError {
    /// Error when trying to open a connection to the Ledger device
    DeviceNotFound,
    /// Unexpected error
    UnexpectedError(String),
}

/// Returns the current version of the Aptos app on Ledger
pub fn get_app_version() -> Result<String, AptosLedgerError> {
    // open connection to ledger
    let transport = match open_ledger_transport() {
        Ok(transport) => transport,
        Err(err) => return Err(err),
    };

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
                let version = format!("{}.{}.{}", major, minor, patch);
                Ok(version)
            } else {
                let error_string = response
                    .error_code()
                    .map(|error_code| error_code.to_string())
                    .unwrap_or_else(|retcode| format!("Error with retcode: {}", retcode));
                Err(AptosLedgerError::UnexpectedError(error_string))
            }
        },
        Err(err) => Err(AptosLedgerError::UnexpectedError(err.to_string())),
    }
}

/// Returns the official app name register in Ledger
pub fn get_app_name() -> Result<String, AptosLedgerError> {
    // open connection to ledger
    let transport = match open_ledger_transport() {
        Ok(transport) => transport,
        Err(err) => return Err(err),
    };

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
                    Err(e) => return Err(AptosLedgerError::UnexpectedError(e.to_string())),
                };
                Ok(app_name.to_string())
            } else {
                let error_string = response
                    .error_code()
                    .map(|error_code| error_code.to_string())
                    .unwrap_or_else(|retcode| format!("Error with retcode: {}", retcode));
                Err(AptosLedgerError::UnexpectedError(error_string))
            }
        },
        Err(err) => Err(AptosLedgerError::UnexpectedError(err.to_string())),
    }
}

/// Returns the public key of your Aptos account in Ledger device
///
/// # Arguments
///
/// * `display` - If true, the public key will be displayed on the Ledger device, and confirmation is needed
pub fn get_public_key(display: bool) -> Result<String, AptosLedgerError> {
    // open connection to ledger
    let transport = match open_ledger_transport() {
        Ok(transport) => transport,
        Err(err) => return Err(err),
    };

    // serialize the derivative path
    let cdata = serialize_bip32(DERIVATIVE_PATH);

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
                // extract the Public key from the response data
                let mut offset = 0;
                let response_buffer = response.data();
                let pub_key_len: usize = (response_buffer[offset] - 1).into();
                offset += 1;

                // Skipping weird 0x04
                offset += 1;

                let pub_key_buffer = response_buffer[offset..offset + pub_key_len].to_vec();
                let hex_string = encode(pub_key_buffer);
                Ok(hex_string)
            } else {
                let error_string = response
                    .error_code()
                    .map(|error_code| error_code.to_string())
                    .unwrap_or_else(|retcode| format!("Error with retcode: {}", retcode));
                Err(AptosLedgerError::UnexpectedError(error_string))
            }
        },
        Err(err) => Err(AptosLedgerError::UnexpectedError(err.to_string())),
    }
}

pub fn sign_txn(raw_txn: Vec<u8>) -> Result<Vec<u8>, AptosLedgerError> {
    // open connection to ledger
    let transport = match open_ledger_transport() {
        Ok(transport) => transport,
        Err(err) => return Err(err),
    };

    // serialize the derivative path
    let derivative_path_bytes = serialize_bip32(DERIVATIVE_PATH);

    // send the derivative path over as first message
    let sign_start = transport.exchange(&APDUCommand {
        cla: CLA_APTOS,
        ins: INS_SIGN_TXN,
        p1: P1_START,
        p2: P2_MORE,
        data: derivative_path_bytes,
    });

    if let Err(err) = sign_start {
        return Err(AptosLedgerError::UnexpectedError(err.to_string()));
    }

    let chunks = raw_txn.chunks(MAX_APDU_LEN);
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
                        return Ok(signature_buffer.to_vec());
                    }
                } else {
                    let error_string = response
                        .error_code()
                        .map(|error_code| error_code.to_string())
                        .unwrap_or_else(|retcode| {
                            format!("Unknown Ledger APDU retcode: {}", retcode)
                        });
                    return Err(AptosLedgerError::UnexpectedError(error_string));
                }
            },
            Err(err) => return Err(AptosLedgerError::UnexpectedError(err.to_string())),
        };
    }
    Err(AptosLedgerError::UnexpectedError(
        "Unable to process request".to_owned(),
    ))
}

/// This is the Rust version of the serialization of BIP32 from Petra Wallet
/// https://github.com/aptos-labs/wallet/blob/main/apps/extension/src/core/ledger/index.ts#L47
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
