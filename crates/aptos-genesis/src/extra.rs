// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_types::account_address::AccountAddress;
use aptos_types::chain_id::ChainId;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::str::FromStr;
use vm_genesis::APTOS_COINS_BASE_WITH_DECIMALS;

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

/// A fixed point representation for APT
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct AptosCoin(pub u64);

impl From<AptosCoin> for u64 {
    fn from(inner: AptosCoin) -> Self {
        inner.0
    }
}

impl Serialize for AptosCoin {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        S::serialize_str(serializer, self.0.to_string().as_str())
    }
}

impl<'de> Deserialize<'de> for AptosCoin {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let str = <String>::deserialize(deserializer)?;
        AptosCoin::from_str(&str)
            .map_err(|err| D::Error::custom(format!("Failed to parse AptosCoin {}", err)))
    }
}

impl FromStr for AptosCoin {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pieces: Vec<&str> = s.trim().split('.').collect();

        let amount = match (pieces.len(), pieces.first(), pieces.get(1)) {
            // If there is no decimal, it's a full APT
            (1, Some(apt), None) => {
                if let Some(amount) =
                    u64::from_str(apt)?.checked_mul(APTOS_COINS_BASE_WITH_DECIMALS)
                {
                    amount
                } else {
                    bail!("Number is too large to handle 8 decimal points {}", s);
                }
            }
            // If there's a decimal, then there are subunits
            (2, Some(apt), Some(subunit)) => {
                let apt = if !apt.is_empty() {
                    u64::from_str(apt)? * APTOS_COINS_BASE_WITH_DECIMALS
                } else {
                    0
                };

                let subunit = if subunit.len() > 8 {
                    bail!(
                        "Too many decimal points, expected 8 or less, but got {}: {}",
                        subunit.len(),
                        s
                    )
                } else if !subunit.is_empty() {
                    // Fill in the missing zeros to the right of the subunit
                    let offset: u64 = 10u64.pow(8 - subunit.len() as u32);

                    if let Some(amount) = u64::from_str(subunit)?.checked_mul(offset) {
                        amount
                    } else {
                        bail!("Failed to parse subunit decimal {}", s)
                    }
                } else {
                    0
                };

                apt + subunit
            }
            _ => bail!("More than one decimal point in the input {}", s),
        };

        Ok(AptosCoin(amount))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_fixed_point() {
        let tests = [
            ("1", APTOS_COINS_BASE_WITH_DECIMALS),
            ("0.00000001", 1),
            ("0.1", APTOS_COINS_BASE_WITH_DECIMALS / 10),
            ("10000", 10000 * APTOS_COINS_BASE_WITH_DECIMALS),
            (
                "10000.01",
                10000 * APTOS_COINS_BASE_WITH_DECIMALS + APTOS_COINS_BASE_WITH_DECIMALS / 100,
            ),
            (".1", APTOS_COINS_BASE_WITH_DECIMALS / 10),
            ("1.0", APTOS_COINS_BASE_WITH_DECIMALS),
            ("1.", APTOS_COINS_BASE_WITH_DECIMALS),
        ];

        for (str, expected) in tests {
            let result = AptosCoin::from_str(str).unwrap().0;
            assert_eq!(
                result, expected,
                "Testcase: {} expected {} got {}",
                str, expected, result
            );
        }

        let bad_tests = ["1.1.", "10000000000000000", "0.000000001", "not_a_number"];
        for str in bad_tests {
            AptosCoin::from_str(str).expect_err(str);
        }

        let yaml = "1000.00000001";
        assert_eq!(
            1000_0000_0001,
            serde_yaml::from_str::<AptosCoin>(yaml).unwrap().0
        );
    }

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
