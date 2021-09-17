// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_types::account_address::AccountAddress;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

#[derive(Clone, Debug, PartialEq)]
pub struct Address(AccountAddress);

impl Address {
    pub fn new(address: AccountAddress) -> Self {
        Self(address)
    }

    pub fn into_inner(&self) -> AccountAddress {
        self.0
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.to_hex_literal())
    }
}

impl FromStr for Address {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self, anyhow::Error> {
        let mut ret = AccountAddress::from_hex_literal(s);
        if ret.is_err() {
            ret = AccountAddress::from_hex(s)
        }
        Ok(Self(ret.map_err(|_| {
            anyhow::format_err!("invalid account address: {}", s)
        })?))
    }
}

impl Serialize for Address {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let address = <String>::deserialize(deserializer)?;
        Address::from_str(&address).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use crate::address::Address;
    use serde_json::{json, Value};
    use std::str::FromStr;

    #[test]
    fn test_from_and_to_string() {
        let valid_addresses = vec!["0x1", "0x001", "00000000000000000000000000000001"];
        for address in valid_addresses {
            assert_eq!(Address::from_str(address).unwrap().to_string(), "0x1");
        }

        let invalid_addresses = vec!["invalid", "00x1", "x1", "01", "1"];
        for address in invalid_addresses {
            assert_eq!(
                format!("invalid account address: {}", address),
                Address::from_str(address).unwrap_err().to_string()
            );
        }
    }

    #[test]
    fn test_from_and_to_json() {
        let address: Address = serde_json::from_value(json!("0x1")).unwrap();
        assert_eq!(address, Address::from_str("0x1").unwrap());

        let val: Value = serde_json::to_value(address).unwrap();
        assert_eq!(val, json!("0x1"));
    }
}
