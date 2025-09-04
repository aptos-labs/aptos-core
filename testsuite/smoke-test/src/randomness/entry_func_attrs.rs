// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    randomness::{
        e2e_basic_consumption::publish_on_chain_dice_module,
        entry_func_attrs::TxnResult::{
            CommittedWithBiasableAbort, CommittedWithNoSeedAbort, DiscardedWithMaxGasCheck,
        },
    },
    smoke_test_environment::SwarmBuilder,
};
use velor::{
    common::types::{CliError, CliTypedResult, GasOptions, TransactionSummary},
    move_tool::MemberId,
};
use velor_forge::{Swarm, SwarmExt};
use velor_logger::info;
use velor_types::on_chain_config::OnChainRandomnessConfig;
use std::{str::FromStr, sync::Arc, time::Duration};

#[derive(Clone, Copy, Debug)]
enum RollFunc {
    /// Represents `dice::roll_v0()` which does not have any randomness attribute.
    NoAttr,
    /// Represents `dice::roll()` which has attribute `#[randomness]`.
    AttrOnly,
    /// Represents `dice::roll_v2()` which has attribute `#[randomness(max_gas=56789)]`.
    AttrWithMaxGasProp,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
enum TxnResult {
    CommittedSucceeded,
    CommittedWithBiasableAbort,
    CommittedWithNoSeedAbort,
    DiscardedWithMaxGasCheck,
    DiscardedWithInsufficientBalanceForDeposit,
}

impl From<CliTypedResult<TransactionSummary>> for TxnResult {
    fn from(value: CliTypedResult<TransactionSummary>) -> Self {
        match value {
            Ok(TransactionSummary { success, .. }) => {
                if success.unwrap() {
                    TxnResult::CommittedSucceeded
                } else {
                    unreachable!()
                }
            },
            Err(e) => match e {
                CliError::ApiError(msg) => {
                    if msg.starts_with("Unknown error Transaction committed on chain, but failed execution: Move abort in 0x1::randomness: E_API_USE_IS_BIASIBLE(0x1)") {
                            return CommittedWithBiasableAbort;
                        }
                    if msg.starts_with("Unknown error Transaction committed on chain, but failed execution: Move abort in 0x1::option: EOPTION_NOT_SET(0x40001)") {
                            return CommittedWithNoSeedAbort;
                        }
                    if msg.contains("REQUIRED_DEPOSIT_INCONSISTENT_WITH_TXN_MAX_GAS") {
                        return DiscardedWithMaxGasCheck;
                    }
                    unreachable!()
                },
                _ => unreachable!(),
            },
        }
    }
}

impl RollFunc {
    fn function_id(&self, account: String) -> MemberId {
        match self {
            RollFunc::NoAttr => MemberId::from_str(&format!("{}::dice::roll_v0", account)).unwrap(),
            RollFunc::AttrOnly => MemberId::from_str(&format!("{}::dice::roll", account)).unwrap(),
            RollFunc::AttrWithMaxGasProp => {
                MemberId::from_str(&format!("{}::dice::roll_v2", account)).unwrap()
            },
        }
    }
}

struct RollParams {
    func: RollFunc,
    max_gas: u64,
    expected_txn_result: TxnResult,
}

impl RollParams {
    fn new(func: RollFunc, max_gas: u64, expected_txn_result: TxnResult) -> Self {
        Self {
            func,
            max_gas,
            expected_txn_result,
        }
    }
}

struct TestParams {
    chain_generates_randomness_seed: bool,
    chain_requires_deposit_for_randtxn: bool,
    chain_allows_custom_max_gas_for_randtxn: bool,
    rolls: Vec<RollParams>,
}

#[tokio::test]
async fn randomness_attr_000() {
    common(TestParams {
        chain_generates_randomness_seed: false,
        chain_requires_deposit_for_randtxn: false,
        chain_allows_custom_max_gas_for_randtxn: false,
        rolls: vec![
            RollParams::new(RollFunc::NoAttr, 10000, CommittedWithBiasableAbort),
            RollParams::new(RollFunc::NoAttr, 45678, CommittedWithBiasableAbort),
            RollParams::new(RollFunc::NoAttr, 56789, CommittedWithBiasableAbort),
            RollParams::new(RollFunc::AttrOnly, 10000, CommittedWithNoSeedAbort),
            RollParams::new(RollFunc::AttrOnly, 45678, CommittedWithNoSeedAbort),
            RollParams::new(RollFunc::AttrOnly, 56789, CommittedWithNoSeedAbort),
            RollParams::new(
                RollFunc::AttrWithMaxGasProp,
                10000,
                CommittedWithNoSeedAbort,
            ),
            RollParams::new(
                RollFunc::AttrWithMaxGasProp,
                45678,
                CommittedWithNoSeedAbort,
            ),
            RollParams::new(
                RollFunc::AttrWithMaxGasProp,
                56789,
                CommittedWithNoSeedAbort,
            ),
        ],
    })
    .await;
}

async fn common(params: TestParams) {
    let TestParams {
        chain_generates_randomness_seed,
        chain_requires_deposit_for_randtxn,
        chain_allows_custom_max_gas_for_randtxn,
        rolls,
    } = params;
    let epoch_duration_secs = 20;
    let estimated_dkg_latency_secs = 30;

    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(1)
        .with_velor()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;

            conf.consensus_config.enable_validator_txns();
            let randomness_config = if chain_generates_randomness_seed {
                OnChainRandomnessConfig::default_enabled()
            } else {
                OnChainRandomnessConfig::default_disabled()
            };
            conf.randomness_config_override = Some(randomness_config);
        }))
        .build_with_cli(0)
        .await;

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Waited too long for epoch 4.");

    let root_address = swarm.chain_info().root_account().address();
    let root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_address);

    info!("Publishing OnChainDice module.");
    publish_on_chain_dice_module(&mut cli, 0).await;

    info!("Update API configs.");
    let script = format!(
        r#"
script {{
    use velor_framework::velor_governance;
    use velor_framework::randomness_api_v0_config;
    use std::option;

    fun main(core_resources: &signer) {{
        let framework_signer = velor_governance::get_signer_testnet_only(core_resources, @0x1);
        let required_gas = if ({}) {{ option::some(10000) }} else {{ option::none() }};
        randomness_api_v0_config::set_for_next_epoch(&framework_signer, required_gas);
        let allow_custom_max_gas = {};
        randomness_api_v0_config::set_allow_max_gas_flag_for_next_epoch(&framework_signer, allow_custom_max_gas);
        velor_governance::reconfigure(&framework_signer);
    }}
}}
"#,
        chain_requires_deposit_for_randtxn, chain_allows_custom_max_gas_for_randtxn
    );

    let gas_options = GasOptions {
        gas_unit_price: Some(100),
        max_gas: Some(2000000),
        expiration_secs: 60,
    };
    let txn_summary = cli
        .run_script_with_gas_options(root_idx, script.as_str(), Some(gas_options))
        .await
        .expect("Txn execution error.");
    println!("txn_summary={:?}", txn_summary);

    tokio::time::sleep(Duration::from_secs(
        epoch_duration_secs + estimated_dkg_latency_secs,
    ))
    .await;

    let account = cli.account_id(root_idx).to_hex_literal();
    for RollParams {
        func,
        max_gas,
        expected_txn_result,
    } in rolls
    {
        let gas_options = GasOptions {
            gas_unit_price: Some(100),
            max_gas: Some(max_gas),
            expiration_secs: 60,
        };
        let txn_summary = cli
            .run_function(
                root_idx,
                Some(gas_options),
                func.function_id(account.clone()),
                vec![],
                vec![],
            )
            .await;
        println!("block_rand_seed={}, requires_deposit={}, allows_custom_max_gas={}, roll_func={:?}, max_gas={:?}, txn_summary={:?}",
                 chain_generates_randomness_seed,
                 chain_requires_deposit_for_randtxn,
                 chain_allows_custom_max_gas_for_randtxn,
                 func, max_gas, txn_summary
        );
        assert_eq!(expected_txn_result, TxnResult::from(txn_summary));
    }
}
