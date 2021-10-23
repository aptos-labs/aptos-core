// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_api_types::{Address, Error, EventKey, MoveStructTag, TransactionId};
use move_core_types::identifier::Identifier;
use percent_encoding::percent_decode_str;
use serde::{Deserialize, Deserializer};

use std::{convert::Infallible, str::FromStr};

pub type AddressParam = Param<Address>;
pub type TransactionIdParam = Param<TransactionId>;
pub type TransactionVersionParam = Param<u64>;
pub type LedgerVersionParam = Param<u64>;
pub type EventKeyParam = Param<EventKey>;
pub type MoveStructTagParam = Param<MoveStructTag>;
pub type MoveIdentifierParam = Param<Identifier>;

/// `Param` is designed for parsing `warp` path parameter or query string
/// into a type specified by the generic type parameter of `Param`.
#[derive(Clone, Debug)]
pub struct Param<T: FromStr> {
    data: String,
    _value: Option<T>,
}

/// `FromStr` is required for parsing `warp` path parameter into `Param` type.
impl<T: FromStr> FromStr for Param<T> {
    type Err = Infallible;

    fn from_str(data: &str) -> Result<Self, Infallible> {
        Ok(Self {
            data: data.to_owned(),
            _value: None,
        })
    }
}

impl<T: FromStr> Param<T> {
    pub fn parse(self, name: &str) -> Result<T, Error> {
        let decoded = percent_decode_str(&self.data)
            .decode_utf8()
            .map_err(|_| Error::invalid_param(name, &self.data))?;

        decoded
            .parse()
            .map_err(|_| Error::invalid_param(name, &decoded))
    }
}

/// `Deserialize` is required for parsing `warp` query string parameter into `Param` type.
impl<'de, T: FromStr> Deserialize<'de> for Param<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = <String>::deserialize(deserializer)?;
        Ok(Self { data, _value: None })
    }
}

#[cfg(test)]
mod tests {
    use super::MoveIdentifierParam;
    use std::str::FromStr;

    #[test]
    fn test_parse_percent_encoded_path_parameter() {
        let param = MoveIdentifierParam::from_str("abcd%5F").unwrap();
        assert!(param.parse("param_name").is_ok())
    }

    #[test]
    fn test_parse_percent_encoded_path_parameter_failed() {
        let param = MoveIdentifierParam::from_str("%3Aabcd").unwrap();
        let ret = param.parse("param_name");
        assert!(ret.is_err());
        assert_eq!(
            "400 Bad Request: invalid parameter param_name: :abcd",
            ret.err().unwrap().to_string()
        );
    }
}
