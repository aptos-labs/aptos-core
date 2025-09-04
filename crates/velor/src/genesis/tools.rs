// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        types::PromptOptions,
        utils::{dir_default_to_current, write_to_file},
    },
    genesis::{
        get_validator_configs,
        git::{GitOptions, EMPLOYEE_VESTING_ACCOUNTS_FILE, LAYOUT_FILE},
        parse_error,
    },
    CliCommand, CliTypedResult,
};
use velor_genesis::config::{EmployeePoolMap, Layout};
use velor_sdk::move_types::account_address::AccountAddress;
use velor_types::account_address::{create_vesting_pool_address, default_stake_pool_address};
use async_trait::async_trait;
use clap::Parser;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

const POOL_ADDRESSES: &str = "pool-addresses.yaml";
const EMPLOYEE_POOL_ADDRESSES: &str = "employee-pool-addresses.yaml";

/// Get pool addresses from a mainnet genesis setup
///
/// Outputs all pool addresses to a file from the genesis files
#[derive(Parser)]
pub struct PoolAddresses {
    /// Output directory for pool addresses
    #[clap(long, value_parser)]
    output_dir: Option<PathBuf>,

    #[clap(flatten)]
    prompt_options: PromptOptions,
    #[clap(flatten)]
    git_options: GitOptions,
}

#[async_trait]
impl CliCommand<Vec<PathBuf>> for PoolAddresses {
    fn command_name(&self) -> &'static str {
        "GetPoolAddresses"
    }

    async fn execute(self) -> CliTypedResult<Vec<PathBuf>> {
        let output_dir = dir_default_to_current(self.output_dir.clone())?;
        let client = self.git_options.get_client()?;
        let layout: Layout = client.get(Path::new(LAYOUT_FILE))?;
        let employee_vesting_accounts: EmployeePoolMap =
            client.get(Path::new(EMPLOYEE_VESTING_ACCOUNTS_FILE))?;
        let validators = get_validator_configs(&client, &layout, true).map_err(parse_error)?;

        let mut address_to_pool = BTreeMap::<AccountAddress, AccountAddress>::new();

        for validator in validators {
            let stake_pool_address = default_stake_pool_address(
                validator.owner_account_address.into(),
                validator.operator_account_address.into(),
            );
            address_to_pool.insert(validator.owner_account_address.into(), stake_pool_address);
        }

        let mut employee_address_to_pool = BTreeMap::<AccountAddress, AccountAddress>::new();

        for employee_pool in employee_vesting_accounts.inner.iter() {
            let stake_pool_address = create_vesting_pool_address(
                employee_pool.validator.owner_account_address.into(),
                employee_pool.validator.operator_account_address.into(),
                0,
                &[],
            );

            employee_address_to_pool.insert(
                employee_pool.validator.owner_account_address.into(),
                stake_pool_address,
            );
        }

        let pool_addresses_file = output_dir.join(POOL_ADDRESSES);
        let employee_pool_addresses_file = output_dir.join(EMPLOYEE_POOL_ADDRESSES);

        write_to_file(
            pool_addresses_file.as_path(),
            POOL_ADDRESSES,
            serde_yaml::to_string(&address_to_pool)?.as_bytes(),
        )?;
        write_to_file(
            employee_pool_addresses_file.as_path(),
            EMPLOYEE_POOL_ADDRESSES,
            serde_yaml::to_string(&employee_address_to_pool)?.as_bytes(),
        )?;

        Ok(vec![pool_addresses_file, employee_pool_addresses_file])
    }
}
