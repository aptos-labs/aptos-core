// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::CliTypedResult;
use serde::{de::DeserializeOwned, Serialize};

pub fn to_yaml<T: Serialize + ?Sized>(input: &T) -> CliTypedResult<String> {
    Ok(serde_yaml::to_string(input)?)
}

pub fn from_yaml<T: DeserializeOwned>(input: &str) -> CliTypedResult<T> {
    Ok(serde_yaml::from_str(input)?)
}

pub fn to_base64_encoded_yaml<T: Serialize + ?Sized>(input: &T) -> CliTypedResult<String> {
    Ok(base64::encode(to_yaml(input)?))
}

pub fn from_base64_encoded_yaml<T: DeserializeOwned>(input: &str) -> CliTypedResult<T> {
    from_yaml(&String::from_utf8(base64::decode(input)?)?)
}
