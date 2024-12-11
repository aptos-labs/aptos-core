// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

include!(concat!(env!("OUT_DIR"), "/generate_transactions.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_protos::transaction::v1::Transaction;

    #[test]
    fn test_generate_transactions() {
        let json_bytes = IMPORTED_TESTNET_TXNS_5979639459_COIN_REGISTER;
        // Check that the transaction is valid JSON
        let transaction = serde_json::from_slice::<Transaction>(json_bytes).unwrap();

        assert_eq!(transaction.version, 5979639459);
    }
}
