// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use hex::{encode, decode};
use ledger_apdu::APDUCommand;
use ledger_transport_hid::{hidapi::HidApi, TransportNativeHID};
use std::str;
use bcs::to_bytes;

const DERIVATIVE_PATH: &str = "m/44'/637'/0'/0'/0'"; // TODO: Add support for multiple index

const CLA_APTOS: u8 = 0x5B; // Aptos CLA Instruction class
const INS_GET_VERSION: u8 = 0x03; // Get version instruction code
const INS_GET_APP_NAME: u8 = 0x04; // Get app name instruction code
const INS_GET_PUB_KEY: u8 = 0x05; // Get public key instruction code
const INS_SIGN_TXN: u8 = 0x06; // Sign the transaction
const APDU_ANSWER_CODE: u16 = 36864; // success code for transport.exchange

const MAX_APDU_LEN: u16 = 255;
const CHUNK_SIZE: usize = 128; // Chunk size to be sent to Ledger
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
        p1: 0,
        p2: 0,
        data: vec![],
    }) {
        Ok(response) => {
            // Received response from Ledger
            if response.retcode() == APDU_ANSWER_CODE {
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
        p1: 0,
        p2: 0,
        data: vec![],
    }) {
        Ok(response) => {
            if response.retcode() == APDU_ANSWER_CODE {
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
        true => 0x01,
        false => 0x00,
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
            if response.retcode() == APDU_ANSWER_CODE {
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

    // await this.sendToDevice(INS.SIGN_TX, P1_START, P2_MORE, pathBuffer);
    let sign_start = transport.exchange(&APDUCommand {
        cla: CLA_APTOS,
        ins: INS_SIGN_TXN,
        p1: P1_START,
        p2: P2_MORE,
        data: derivative_path_bytes.clone(),
    });

    if let Err(err) = sign_start {
        return Err(AptosLedgerError::UnexpectedError(err.to_string()));
    }

    // build the cdata for ledger txn signing transport
    // let cdata: Vec<u8> = raw_txn;

    let chunks = raw_txn.chunks(CHUNK_SIZE);
    let chunks_count = chunks.len();

    for (i, chunk) in chunks.enumerate() {
        let is_last_chunk = chunks_count == i + 1;
        match transport.exchange(&APDUCommand {
            cla: CLA_APTOS,
            ins: INS_SIGN_TXN,
            p1: (i+1) as u8,
            p2: if is_last_chunk { 0x80 } else { 0x00 },
            data: chunk.to_vec(),
        }) {
            Ok(response) => {
                // success response
                if response.retcode() == APDU_ANSWER_CODE {
                    if is_last_chunk {
                        println!("response: {:?}", response.data());
                        let mut offset = 0;
                        let response_buffer = response.data();
                        let pub_key_len: usize = (response_buffer[offset] - 1).into();
                        offset += 1;
                        let sign_bytes = response_buffer[offset..offset + pub_key_len].to_vec();
                        return Ok(sign_bytes);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_app_version() {
        let version = get_app_version();
        println!("Version: {:?}", version);
        let app_name = get_app_name();
        println!("App Name: {:?}", app_name);
        let pub_key = get_public_key(false);
        println!("Public Key: {:?}", pub_key);
    }

    #[test]
    fn test_get_app_name() {
        let app_name = get_app_name();
        println!("App Name: {:?}", app_name);
    }

    #[test]
    fn test_get_public_key() {
        let pub_key = get_public_key(false);
        println!("Public Key: {:?}", pub_key);
    }

    #[test]
    fn test_sign_txn() {
        let txn_string = b"b5e97db07fa0bd0e5598aa3643a9bc6f6693bddc1a9fec9e674a461eaa00b193783135e8b00430253a22ba041d860c373d7a1501ccf7ac2d1ad37a8ed2775aee000000000000000002000000000000000000000000000000000000000000000000000000000000000104636f696e087472616e73666572010700000000000000000000000000000000000000000000000000000000000000010a6170746f735f636f696e094170746f73436f696e000220094c6fc0d3b382a599c37e1aaa7618eff2c96a3586876082c4594c50c50d7dde082a00000000000000204e0000000000006400000000000000565c51630000000022";
        let signed_txn = sign_txn(txn_string.to_vec());
        println!("Signed txn: {:?}", signed_txn);
    }
}
