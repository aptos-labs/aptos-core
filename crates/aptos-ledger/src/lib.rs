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
const APDU_ANSWER_CODE: u16 = 36864; // success code for transport.exchange

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_app_version() {
        let version = get_app_version();
        println!("Version: {:?}", version);
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
}
