// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub mod json_transactions;

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_protos::transaction::v1::Transaction;
    use json_transactions::generated_transactions::IMPORTED_TESTNET_TXNS_5979639459_COIN_REGISTER;
    #[test]
    fn test_generate_transactions() {
        let json_bytes = IMPORTED_TESTNET_TXNS_5979639459_COIN_REGISTER;
        // Check that the transaction is valid JSON
        let transaction = serde_json::from_slice::<Transaction>(json_bytes).unwrap();

        assert_eq!(transaction.version, 5979639459);
    }
}
