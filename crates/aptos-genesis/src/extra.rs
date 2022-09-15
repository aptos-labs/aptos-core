// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_types::account_address::AccountAddress;
use aptos_types::chain_id::ChainId;
use aptos_types::coin::AptosCoin;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::str::FromStr;

type Username = String;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Network {
    pub root_key: Option<Ed25519PublicKey>,
    pub users: Vec<Username>,
    pub chain_id: ChainId,
    pub allow_new_validators: bool,
    pub epoch_duration_secs: u64,
    pub min_stake: u64,
    pub min_voting_threshold: u128,
    pub max_stake: u64,
    pub recurring_lockup_duration_secs: u64,
    pub required_proposer_stake: u64,
    pub rewards_apy_percentage: u64,
    pub voting_duration_secs: u64,
    pub voting_power_increase_limit: u64,
    pub total_initial_supply: u128,
    pub genesis_validators: Vec<Username>,
    pub future_validators: Vec<Username>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Validator {
    owner_account_address: AccountAddress,
    operator_account_address: AccountAddress,
    voter_account_address: AccountAddress,
    stake_amount: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Operator {
    operator: AccountAddress,
    amount: u64,
    commission: u64,
}

impl Serialize for Operator {
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

impl<'de> Deserialize<'de> for Operator {
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

        Ok(Operator {
            operator,
            amount,
            commission,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Organization {
    admin: AccountAddress,
    #[serde(default)]
    operators: Vec<Operator>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct AccountGroup {
    admin: AccountAddress,
    operator: AccountAddress,
    #[serde(default)]
    accounts: Vec<AccountAddress>,
    #[serde(default)]
    beneficiaries: Vec<Option<AccountAddress>>,
    stake_amount: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Recovery {
    #[serde(default)]
    accounts: Vec<AccountAddress>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AccountAmounts {
    amounts: Vec<BTreeMap<AccountAddress, AptosCoin>>,
}

impl Serialize for AccountAmounts {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.amounts.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AccountAmounts {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let amounts = <Vec<BTreeMap<AccountAddress, AptosCoin>>>::deserialize(deserializer)?;
        Ok(AccountAmounts { amounts })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_initial_split() {
        let yaml = "---
- 0x123: 500.1234
- 0x234: 10000.2345";

        let split: AccountAmounts = serde_yaml::from_str(yaml).unwrap();
        let amounts = vec![
            vec![(
                AccountAddress::from_str("0x123").unwrap(),
                AptosCoin::from_str("500.1234").unwrap(),
            )]
            .into_iter()
            .collect(),
            vec![(
                AccountAddress::from_str("0x234").unwrap(),
                AptosCoin::from_str("10000.2345").unwrap(),
            )]
            .into_iter()
            .collect(),
        ];
        let expected = AccountAmounts { amounts };

        assert_eq!(split, expected);
    }

    #[test]
    fn test_foundation_deserialize() {
        let yaml = "
admin: 0x123
operators:
- [0x234, 1000, 10]
- [0x345, 2000, 7]
- [0x456, 3000, 5]";

        let output: Organization = serde_yaml::from_str(yaml).expect("Should deserialize");
        let organization = Organization {
            admin: AccountAddress::from_str("0x123").unwrap(),
            operators: vec![
                Operator {
                    operator: AccountAddress::from_str("0x234").unwrap(),
                    amount: 1000,
                    commission: 10,
                },
                Operator {
                    operator: AccountAddress::from_str("0x345").unwrap(),
                    amount: 2000,
                    commission: 7,
                },
                Operator {
                    operator: AccountAddress::from_str("0x456").unwrap(),
                    amount: 3000,
                    commission: 5,
                },
            ],
        };
        // Ensure that the example can be deserialized
        assert_eq!(organization, output);

        // Serialization and deserialization should work fine (so we can automate this)
        assert_eq!(
            serde_yaml::from_str::<Organization>(&serde_yaml::to_string(&organization).unwrap())
                .unwrap(),
            organization
        );
    }
}
