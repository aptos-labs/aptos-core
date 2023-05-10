// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use ledger_apdu::APDUCommand;
use ledger_transport_hid::hidapi::HidApi;
use ledger_transport_hid::TransportNativeHID;

const LEDGER_VID: u16 = 0x2c97; // Ledger Vendor ID
const CLA_APTOS: u8 = 0x5b; // Aptos CLA Instruction class
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
        p1: 0, // Instruction parameter 1 (offset)
        p2: 0,
        data: vec![],
    }) {
        Ok(response) => {
            // Ok means we successfully exchanged with the Ledger
            // but doesn't mean our request succeeded
            // we need to check it based on `response.retcode`
            if response.retcode() == APDU_ANSWER_CODE {
                let major = response.data()[0];
                let minor = response.data()[1];
                let patch = response.data()[2];
                let version = format!("{}.{}.{}", major, minor, patch);
                return Ok(version);
            } else {
                let error_string = response
                    .error_code()
                    .map(|error_code| error_code.to_string())
                    .unwrap_or_else(|retcode| format!("Error with retcode: {}", retcode));
                return Err(AptosLedgerError::UnexpectedError(error_string));
            }
        },
        Err(err) => return Err(AptosLedgerError::UnexpectedError(err.to_string())),
    };
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_app_version() {
        let version = get_app_version();
        println!("Version: {:?}", version);
    }
}
