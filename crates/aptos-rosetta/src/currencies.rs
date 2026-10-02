// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Currency configuration loading and validation for Rosetta.

use crate::{common::native_coin, types::Currency};
use aptos_logger::warn;
use aptos_types::account_address::AccountAddress;
use move_core_types::language_storage::StructTag;
use std::{collections::HashSet, str::FromStr};

/// Loads the default native coin plus any valid currencies from a config list.
///
/// A configured currency is accepted when it has a non-empty symbol and either:
/// - no metadata,
/// - a valid Move coin type (`move_type`), or
/// - a valid fungible-asset metadata address (`fa_address`).
pub fn load_supported_currencies(configured: Vec<Currency>) -> HashSet<Currency> {
    let mut supported_currencies = HashSet::new();
    supported_currencies.insert(native_coin());

    for item in configured {
        if item.symbol.as_str().is_empty() {
            warn!(
                "Currency {:?} has an empty symbol, and is being skipped",
                item
            );
            continue;
        }

        let Some(metadata) = item.metadata.as_ref() else {
            supported_currencies.insert(item);
            continue;
        };

        let move_type_valid = match metadata.move_type.as_ref() {
            Some(move_type) => StructTag::from_str(move_type).is_ok(),
            None => false,
        };
        let fa_address_valid = match metadata.fa_address.as_ref() {
            Some(fa_address) => AccountAddress::from_str(fa_address).is_ok(),
            None => false,
        };

        // Reject when move_type is present but invalid.
        if metadata.move_type.is_some() && !move_type_valid {
            warn!(
                "Currency {:?} has an invalid metadata coin type, and is being skipped",
                item
            );
            continue;
        }

        // Reject when fa_address is present but invalid.
        if metadata.fa_address.is_some() && !fa_address_valid {
            warn!(
                "Currency {:?} has an invalid fungible asset address, and is being skipped",
                item
            );
            continue;
        }

        // Accept coin-type, FA-only, or combined metadata.
        if move_type_valid || fa_address_valid {
            supported_currencies.insert(item);
        } else {
            warn!(
                "Currency {:?} has neither a valid move type nor fungible asset address, and is being skipped",
                item
            );
        }
    }

    supported_currencies
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CurrencyMetadata;

    #[test]
    fn accepts_fa_only_currency() {
        let currencies = load_supported_currencies(vec![Currency {
            symbol: "TFA".to_string(),
            decimals: 4,
            metadata: Some(CurrencyMetadata {
                move_type: None,
                fa_address: Some(
                    "0x7e51ad6e79cd113f5abe08f53ed6a3c2bfbf88561a24ae10b9e1e822e0623dfd"
                        .to_string(),
                ),
            }),
        }]);

        assert!(currencies.iter().any(|c| c.symbol == "TFA"));
    }

    #[test]
    fn accepts_coin_type_currency() {
        let currencies = load_supported_currencies(vec![Currency {
            symbol: "TC".to_string(),
            decimals: 4,
            metadata: Some(CurrencyMetadata {
                move_type: Some(
                    "0xf5a9b6ccc95f8ad3c671ddf1e227416e71f7bcd3c971efe83c0ae8e5e028350f::test_faucet::TestFaucetCoin"
                        .to_string(),
                ),
                fa_address: Some(
                    "0xb528ad40e472f8fcf0f21aa78aecd09fe68f6208036a5845e6d16b7d561c83b8"
                        .to_string(),
                ),
            }),
        }]);

        assert!(currencies.iter().any(|c| c.symbol == "TC"));
    }

    #[test]
    fn rejects_invalid_fa_address() {
        let currencies = load_supported_currencies(vec![Currency {
            symbol: "BAD".to_string(),
            decimals: 4,
            metadata: Some(CurrencyMetadata {
                move_type: None,
                fa_address: Some("not-an-address".to_string()),
            }),
        }]);

        assert!(!currencies.iter().any(|c| c.symbol == "BAD"));
    }

    #[test]
    fn rejects_empty_symbol() {
        let currencies = load_supported_currencies(vec![Currency {
            symbol: "".to_string(),
            decimals: 4,
            metadata: Some(CurrencyMetadata {
                move_type: None,
                fa_address: Some(
                    "0x7e51ad6e79cd113f5abe08f53ed6a3c2bfbf88561a24ae10b9e1e822e0623dfd"
                        .to_string(),
                ),
            }),
        }]);

        assert_eq!(currencies.len(), 1);
        assert!(currencies.contains(&native_coin()));
    }
}
