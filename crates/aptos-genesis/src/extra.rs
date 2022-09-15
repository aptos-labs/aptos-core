// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_types::account_address::AccountAddress;
use aptos_types::chain_id::ChainId;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Network {
    /// TODO: In the future, we won't need a root key
    pub root_key: Option<Ed25519PublicKey>,
    /// List of usernames or identifiers
    pub users: Vec<String>,
    /// ChainId for the target network
    pub chain_id: ChainId,
    /// Whether to allow new validators to join the set after genesis
    #[serde(default)]
    pub allow_new_validators: bool,
    /// Duration of an epoch
    pub epoch_duration_secs: u64,
    /// Minimum stake to be in the validator set
    pub min_stake: u64,
    /// Minimum number of votes to consider a proposal valid.
    pub min_voting_threshold: u128,
    /// Maximum stake to be in the validator set
    pub max_stake: u64,
    /// Minimum number of seconds to lockup staked coins
    pub recurring_lockup_duration_secs: u64,
    /// Required amount of stake to create proposals.
    pub required_proposer_stake: u64,
    /// Percentage of stake given out as rewards a year (0-100%).
    pub rewards_apy_percentage: u64,
    /// Voting duration for a proposal in seconds.
    pub voting_duration_secs: u64,
    /// % of current epoch's total voting power that can be added in this epoch.
    pub voting_power_increase_limit: u64,
    pub total_initial_supply: u128,
    pub genesis_validators: Vec<String>,
    pub future_validators: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Validator {
    owner_account_address: AccountAddress,
    operator_account_address: AccountAddress,
    voter_account_address: AccountAddress,
    stake_amount: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FoundationOperator {
    operator: AccountAddress,
    amount: u64,
    commission: u64,
}

impl Serialize for FoundationOperator {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let vec = vec![
            self.operator.to_string(),
            self.amount.to_string(),
            self.commission.to_string(),
        ];
        <Vec<String>>::serialize(&vec, serializer)
    }
}

impl<'de> Deserialize<'de> for FoundationOperator {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec = <Vec<String>>::deserialize(deserializer)?;

        if vec.len() != 3 {
            return Err(D::Error::custom(format!(
                "Invalid number of fields, expected {} got {}",
                3,
                vec.len()
            )));
        }

        let operator = AccountAddress::from_str(vec.get(0).unwrap())
            .map_err(|err| D::Error::custom(format!("Invalid operator field {}", err)))?;
        let amount = u64::from_str(vec.get(1).unwrap())
            .map_err(|err| D::Error::custom(format!("Invalid amount field {}", err)))?;
        let commission = u64::from_str(vec.get(2).unwrap())
            .map_err(|err| D::Error::custom(format!("Invalid commission field {}", err)))?;

        Ok(FoundationOperator {
            operator,
            amount,
            commission,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Foundation {
    admin: AccountAddress,
    #[serde(default)]
    operators: Vec<FoundationOperator>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_foundation_deserialize() {
        let yaml = "
admin: '0x123'
operators:
- [0x234, 1000, 10]
- [0x345, 2000, 7]
- [0x456, 3000, 5]";

        let output: Foundation = serde_yaml::from_str(yaml).expect("Should deserialize");
        let foundation = Foundation {
            admin: AccountAddress::from_str("0x123").unwrap(),
            operators: vec![
                FoundationOperator {
                    operator: AccountAddress::from_str("0x234").unwrap(),
                    amount: 1000,
                    commission: 10,
                },
                FoundationOperator {
                    operator: AccountAddress::from_str("0x345").unwrap(),
                    amount: 2000,
                    commission: 7,
                },
                FoundationOperator {
                    operator: AccountAddress::from_str("0x456").unwrap(),
                    amount: 3000,
                    commission: 5,
                },
            ],
        };
        // Ensure that the example can be deserialized
        assert_eq!(foundation, output);

        // Serialization and deserialization should work fine (so we can automate this)
        assert_eq!(
            serde_yaml::from_str::<Foundation>(&serde_yaml::to_string(&foundation).unwrap())
                .unwrap(),
            foundation
        );
    }
}
