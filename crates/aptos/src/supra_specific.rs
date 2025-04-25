// Copyright © Entropy Foundation

use crate::common::types::{GasOptions, ProfileOptions, RestOptions};

impl From<ProfileOptions> for supra_aptos::ProfileOptions {
    fn from(value: ProfileOptions) -> Self {
        Self { profile: value.profile }
    }
}

impl From<RestOptions> for supra_aptos::RestOptions {
    fn from(value: RestOptions) -> Self {
        Self {
            rpc_url: value.rpc_url,
            connection_timeout_secs: value.connection_timeout_secs,
            node_api_key: value.node_api_key,
            api_version: value.api_version,
        }
    }
}

impl From<GasOptions> for supra_aptos::GasOptions {
    fn from(value: GasOptions) -> Self {
        Self {
            gas_unit_price: value.gas_unit_price,
            max_gas: value.max_gas,
            expiration_secs: value.expiration_secs,
        }
    }
}
