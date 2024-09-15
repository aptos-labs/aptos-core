// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

include!(concat!(env!("OUT_DIR"), "/generate_transactions.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_protos::transaction::v1::Transaction;

    #[test]
    fn test_generate_transactions() {
        let json_bytes = GENERATED_USER_SCRIPT_TRANSACTION;
        // Check that the transaction is valid JSON
        let transaction = serde_json::from_slice::<Transaction>(json_bytes).unwrap();

        assert_eq!(transaction.version, 1);
    }
}
