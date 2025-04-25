// Copyright © Entropy Foundation

use anyhow::Result;
use aptos_types::account_address::AccountAddress;
use aptos_types::transaction::TransactionPayload;
use async_trait::async_trait;
use clap::ValueEnum;
use std::fmt::{Display, Formatter};

/// RPC api versions maintained by this module.
#[derive(PartialEq, Eq, Debug, Clone, Copy, Default, ValueEnum)]
pub enum ApiVersion {
    V1,
    V2,
    #[default]
    V3,
}

impl Display for ApiVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiVersion::V1 => write!(f, "v1"),
            ApiVersion::V2 => write!(f, "v2"),
            ApiVersion::V3 => write!(f, "v3"),
        }
    }
}

pub struct ProfileOptions {
    /// Profile to use from the CLI config
    ///
    /// This will be used to override associated settings such as
    /// the REST URL, the Faucet URL, and the private key arguments.
    ///
    /// Defaults to "default"
    pub profile: Option<String>,
}

pub struct RestOptions {
    /// URL to a fullnode on the network
    ///
    /// Defaults to the URL in the `default` profile
    pub rpc_url: Option<reqwest::Url>,

    /// Connection timeout in seconds, used for the REST endpoint of the fullnode
    pub connection_timeout_secs: u64,

    /// Key to use for ratelimiting purposes with the node API. This value will be used
    /// as `Authorization: Bearer <key>`. You may also set this with the NODE_API_KEY
    /// environment variable.
    pub node_api_key: Option<String>,
    pub api_version: ApiVersion,
}

pub struct GasOptions {
    /// Gas multiplier per unit of gas
    ///
    /// The amount of Quants (10^-8 SUPRA) used for a transaction is equal
    /// to (gas unit price * gas used).  The gas_unit_price can
    /// be used as a multiplier for the amount of Quants willing
    /// to be paid for a transaction.  This will prioritize the
    /// transaction with a higher gas unit price.
    ///
    /// Without a value, it will determine the price based on the current estimated price
    pub gas_unit_price: Option<u64>,
    /// Maximum amount of gas units to be used to send this transaction
    ///
    /// The maximum amount of gas units willing to pay for the transaction.
    /// This is the (max gas in Quants / gas unit price).
    ///
    /// For example if I wanted to pay a maximum of 100 Quants, I may have the
    /// max gas set to 100 if the gas unit price is 1.  If I want it to have a
    /// gas unit price of 2, the max gas would need to be 50 to still only have
    /// a maximum price of 100 Quants.
    ///
    /// Without a value, it will determine the price based on simulating the current transaction
    pub max_gas: Option<u64>,
    /// Number of seconds to expire the transaction
    ///
    /// This is the number of seconds from the current local computer time.
    pub expiration_secs: u64,
}

/// Arguments required by supra cli for its operation.
pub struct SupraCommandArguments {
    /// Transaction payload
    pub payload: TransactionPayload,
    /// Sender account address
    ///
    /// This allows you to override the account address from the derived account address
    /// in the event that the authentication key was rotated or for a resource account
    pub sender_account: Option<AccountAddress>,
    pub profile_options: ProfileOptions,
    pub rest_options: RestOptions,
    pub gas_options: GasOptions,
}

/// Trait required by supra cli for its operation.
#[async_trait]
pub trait SupraCommand {
    /// consume self and returns [SupraCommandArguments]
    async fn supra_command_arguments(self) -> Result<SupraCommandArguments>;
}
