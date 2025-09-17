// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

mod genesis_context;

use crate::genesis_context::GenesisStateView;
use aptos_crypto::{
    bls12381,
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    HashValue, PrivateKey, Uniform,
};
use aptos_framework::{ReleaseBundle, ReleasePackage};
use aptos_gas_schedule::{
    AptosGasParameters, InitialGasSchedule, ToOnChainGasSchedule, LATEST_GAS_FEATURE_VERSION,
};
use aptos_types::{
    account_config::{
        self, aptos_test_root_address, events::NewEpochEvent, CORE_CODE_ADDRESS,
        EXPERIMENTAL_CODE_ADDRESS,
    },
    chain_id::ChainId,
    contract_event::{ContractEvent, ContractEventV1},
    executable::ModulePath,
    jwks::{
        jwk::{JWKMoveStruct, JWK},
        patch::{IssuerJWK, PatchJWKMoveStruct, PatchUpsertJWK},
        secure_test_rsa_jwk,
    },
    keyless::{
        self, test_utils::get_sample_iss, Groth16VerificationKey, KEYLESS_ACCOUNT_MODULE_NAME,
    },
    move_utils::as_move_value::AsMoveValue,
    on_chain_config::{
        randomness_api_v0_config::{AllowCustomMaxGasFlag, RequiredGasDeposit},
        FeatureFlag, Features, GasScheduleV2, OnChainConsensusConfig, OnChainExecutionConfig,
        OnChainJWKConsensusConfig, OnChainRandomnessConfig, RandomnessConfigMoveStruct,
        APTOS_MAX_KNOWN_VERSION,
    },
    state_store::state_key::StateKey,
    transaction::{authenticator::AuthenticationKey, ChangeSet, Transaction, WriteSetPayload},
    write_set::{TransactionWrite, WriteOp, WriteSet},
};
use aptos_vm::{
    data_cache::AsMoveResolver,
    move_vm_ext::{
        convert_modules_into_write_ops, AptosMoveResolver, GenesisMoveVm, GenesisRuntimeBuilder,
        SessionExt,
    },
};
use aptos_vm_types::{
    change_set::VMChangeSet,
    module_and_script_storage::{module_storage::AptosModuleStorage, AsAptosCodeStorage},
    module_write_set::{ModuleWrite, ModuleWriteSet},
};
use bytes::Bytes;
use claims::assert_ok;
use move_binary_format::errors::{Location, VMResult};
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    value::{serialize_values, MoveValue},
};
use move_vm_runtime::{
    module_traversal::{TraversalContext, TraversalStorage},
    ModuleStorage, RuntimeEnvironment, StagingModuleStorage,
};
use move_vm_types::gas::UnmeteredGasMeter;
use once_cell::sync::Lazy;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// The seed is arbitrarily picked to produce a consistent key. XXX make this more formal?
const GENESIS_SEED: [u8; 32] = [42; 32];

const GENESIS_MODULE_NAME: &str = "genesis";
const GOVERNANCE_MODULE_NAME: &str = "aptos_governance";
const CODE_MODULE_NAME: &str = "code";
const VERSION_MODULE_NAME: &str = "version";
const JWK_CONSENSUS_CONFIG_MODULE_NAME: &str = "jwk_consensus_config";
const JWKS_MODULE_NAME: &str = "jwks";
const CONFIG_BUFFER_MODULE_NAME: &str = "config_buffer";
const DKG_MODULE_NAME: &str = "dkg";
const RANDOMNESS_API_V0_CONFIG_MODULE_NAME: &str = "randomness_api_v0_config";
const RANDOMNESS_CONFIG_SEQNUM_MODULE_NAME: &str = "randomness_config_seqnum";
const RANDOMNESS_CONFIG_MODULE_NAME: &str = "randomness_config";
const RANDOMNESS_MODULE_NAME: &str = "randomness";
const ACCOUNT_ABSTRACTION_MODULE_NAME: &str = "account_abstraction";
const RECONFIGURATION_STATE_MODULE_NAME: &str = "reconfiguration_state";

const NUM_SECONDS_PER_YEAR: u64 = 365 * 24 * 60 * 60;
const MICRO_SECONDS_PER_SECOND: u64 = 1_000_000;
const APTOS_COINS_BASE_WITH_DECIMALS: u64 = u64::pow(10, 8);

pub struct GenesisConfiguration {
    pub allow_new_validators: bool,
    pub epoch_duration_secs: u64,
    // If true, genesis will create a special core resources account that can mint coins.
    pub is_test: bool,
    pub max_stake: u64,
    pub min_stake: u64,
    pub min_voting_threshold: u128,
    pub recurring_lockup_duration_secs: u64,
    pub required_proposer_stake: u64,
    pub rewards_apy_percentage: u64,
    pub voting_duration_secs: u64,
    pub voting_power_increase_limit: u64,
    pub employee_vesting_start: u64,
    pub employee_vesting_period_duration: u64,
    pub initial_features_override: Option<Features>,
    pub randomness_config_override: Option<OnChainRandomnessConfig>,
    pub jwk_consensus_config_override: Option<OnChainJWKConsensusConfig>,
    pub initial_jwks: Vec<IssuerJWK>,
    pub keyless_groth16_vk: Option<Groth16VerificationKey>,
}

pub static GENESIS_KEYPAIR: Lazy<(Ed25519PrivateKey, Ed25519PublicKey)> = Lazy::new(|| {
    let mut rng = StdRng::from_seed(GENESIS_SEED);
    let private_key = Ed25519PrivateKey::generate(&mut rng);
    let public_key = private_key.public_key();
    (private_key, public_key)
});

// Cannot be impl Default in GasScheduleV2, due to circular dependencies.
pub fn default_gas_schedule() -> GasScheduleV2 {
    GasScheduleV2 {
        feature_version: LATEST_GAS_FEATURE_VERSION,
        entries: AptosGasParameters::initial().to_on_chain_gas_schedule(LATEST_GAS_FEATURE_VERSION),
    }
}

pub fn encode_aptos_mainnet_genesis_transaction(
    accounts: &[AccountBalance],
    employees: &[EmployeePool],
    validators: &[ValidatorWithCommissionRate],
    framework: &ReleaseBundle,
    chain_id: ChainId,
    genesis_config: &GenesisConfiguration,
) -> Transaction {
    assert!(!genesis_config.is_test, "This is mainnet!");
    validate_genesis_config(genesis_config);

    let mut state_view = GenesisStateView::new();
    for (module_bytes, module) in framework.code_and_compiled_modules() {
        state_view.add_module(&module.self_id(), module_bytes);
    }

    let genesis_runtime_builder = GenesisRuntimeBuilder::new(chain_id);
    let genesis_runtime_environment = genesis_runtime_builder.build_genesis_runtime_environment();

    let module_storage = state_view.as_aptos_code_storage(&genesis_runtime_environment);
    let resolver = state_view.as_move_resolver();

    let genesis_vm = genesis_runtime_builder.build_genesis_vm();
    let genesis_change_set_configs = genesis_vm.genesis_change_set_configs();
    let mut session = genesis_vm.new_genesis_session(&resolver, HashValue::zero());

    let traversal_storage = TraversalStorage::new();
    let mut traversal_context = TraversalContext::new(&traversal_storage);

    // On-chain genesis process.
    let consensus_config = OnChainConsensusConfig::default_for_genesis();
    let execution_config = OnChainExecutionConfig::default_for_genesis();
    let gas_schedule = default_gas_schedule();
    initialize(
        &mut session,
        &module_storage,
        &mut traversal_context,
        chain_id,
        genesis_config,
        &consensus_config,
        &execution_config,
        &gas_schedule,
    );
    initialize_features(
        &mut session,
        &module_storage,
        &mut traversal_context,
        genesis_config
            .initial_features_override
            .clone()
            .map(Features::into_flag_vec),
    );
    initialize_aptos_coin(&mut session, &module_storage, &mut traversal_context);
    initialize_on_chain_governance(
        &mut session,
        &module_storage,
        &mut traversal_context,
        genesis_config,
    );
    create_accounts(
        &mut session,
        &module_storage,
        &mut traversal_context,
        accounts,
    );
    create_employee_validators(
        &mut session,
        &module_storage,
        &mut traversal_context,
        employees,
        genesis_config,
    );
    create_and_initialize_validators_with_commission(
        &mut session,
        &module_storage,
        &mut traversal_context,
        validators,
    );
    set_genesis_end(&mut session, &module_storage, &mut traversal_context);

    // Reconfiguration should happen after all on-chain invocations.
    emit_new_block_and_epoch_event(&mut session, &module_storage, &mut traversal_context);

    // Create a change set with all initialized resources.
    let mut change_set = assert_ok!(session.finish(&genesis_change_set_configs, &module_storage,));

    // Publish the framework, using a different session id, in case both sessions create tables.
    let mut new_id = [0u8; 32];
    new_id[31] = 1;

    let (additional_change_set, module_write_set) = publish_framework(
        &genesis_vm,
        &genesis_runtime_environment,
        HashValue::new(new_id),
        framework,
    );
    assert_ok!(change_set.squash_additional_change_set(additional_change_set));

    let change_set = assert_ok!(change_set.try_combine_into_storage_change_set(module_write_set));
    verify_genesis_module_write_set(change_set.write_set());
    verify_genesis_events(change_set.events());

    Transaction::GenesisTransaction(WriteSetPayload::Direct(change_set))
}

pub fn encode_genesis_transaction(
    aptos_root_key: Ed25519PublicKey,
    validators: &[Validator],
    framework: &ReleaseBundle,
    chain_id: ChainId,
    genesis_config: &GenesisConfiguration,
    consensus_config: &OnChainConsensusConfig,
    execution_config: &OnChainExecutionConfig,
    gas_schedule: &GasScheduleV2,
) -> Transaction {
    Transaction::GenesisTransaction(WriteSetPayload::Direct(encode_genesis_change_set(
        &aptos_root_key,
        validators,
        framework,
        chain_id,
        genesis_config,
        consensus_config,
        execution_config,
        gas_schedule,
    )))
}

pub fn encode_genesis_change_set(
    core_resources_key: &Ed25519PublicKey,
    validators: &[Validator],
    framework: &ReleaseBundle,
    chain_id: ChainId,
    genesis_config: &GenesisConfiguration,
    consensus_config: &OnChainConsensusConfig,
    execution_config: &OnChainExecutionConfig,
    gas_schedule: &GasScheduleV2,
) -> ChangeSet {
    validate_genesis_config(genesis_config);

    let mut state_view = GenesisStateView::new();
    for (module_bytes, module) in framework.code_and_compiled_modules() {
        state_view.add_module(&module.self_id(), module_bytes);
    }

    let genesis_runtime_builder = GenesisRuntimeBuilder::new(chain_id);
    let genesis_runtime_environment = genesis_runtime_builder.build_genesis_runtime_environment();

    let module_storage = state_view.as_aptos_code_storage(&genesis_runtime_environment);
    let resolver = state_view.as_move_resolver();

    let genesis_vm = genesis_runtime_builder.build_genesis_vm();
    let genesis_change_set_configs = genesis_vm.genesis_change_set_configs();
    let mut session = genesis_vm.new_genesis_session(&resolver, HashValue::zero());

    let traversal_storage = TraversalStorage::new();
    let mut traversal_context = TraversalContext::new(&traversal_storage);

    // On-chain genesis process.
    initialize(
        &mut session,
        &module_storage,
        &mut traversal_context,
        chain_id,
        genesis_config,
        consensus_config,
        execution_config,
        gas_schedule,
    );
    initialize_features(
        &mut session,
        &module_storage,
        &mut traversal_context,
        genesis_config
            .initial_features_override
            .clone()
            .map(Features::into_flag_vec),
    );
    if genesis_config.is_test {
        initialize_core_resources_and_aptos_coin(
            &mut session,
            &module_storage,
            &mut traversal_context,
            core_resources_key,
        );
    } else {
        initialize_aptos_coin(&mut session, &module_storage, &mut traversal_context);
    }
    initialize_config_buffer(&mut session, &module_storage, &mut traversal_context);
    initialize_dkg(&mut session, &module_storage, &mut traversal_context);
    initialize_reconfiguration_state(&mut session, &module_storage, &mut traversal_context);
    let randomness_config = genesis_config
        .randomness_config_override
        .clone()
        .unwrap_or_else(OnChainRandomnessConfig::default_for_genesis);
    initialize_randomness_api_v0_config(&mut session, &module_storage, &mut traversal_context);
    initialize_randomness_config_seqnum(&mut session, &module_storage, &mut traversal_context);
    initialize_randomness_config(
        &mut session,
        &module_storage,
        &mut traversal_context,
        randomness_config,
    );
    initialize_randomness_resources(&mut session, &module_storage, &mut traversal_context);
    initialize_on_chain_governance(
        &mut session,
        &module_storage,
        &mut traversal_context,
        genesis_config,
    );
    initialize_account_abstraction(&mut session, &module_storage, &mut traversal_context);
    create_and_initialize_validators(
        &mut session,
        &module_storage,
        &mut traversal_context,
        validators,
    );
    if genesis_config.is_test {
        allow_core_resources_to_set_version(&mut session, &module_storage, &mut traversal_context);
    }
    let jwk_consensus_config = genesis_config
        .jwk_consensus_config_override
        .clone()
        .unwrap_or_else(OnChainJWKConsensusConfig::default_for_genesis);
    initialize_jwk_consensus_config(
        &mut session,
        &module_storage,
        &mut traversal_context,
        &jwk_consensus_config,
    );
    initialize_jwks_resources(&mut session, &module_storage, &mut traversal_context);
    initialize_keyless_accounts(
        &mut session,
        &module_storage,
        &mut traversal_context,
        chain_id,
        genesis_config.initial_jwks.clone(),
        genesis_config.keyless_groth16_vk.clone(),
    );
    initialize_confidential_asset(
        &mut session,
        &module_storage,
        chain_id,
        &mut traversal_context,
    );
    set_genesis_end(&mut session, &module_storage, &mut traversal_context);

    // Reconfiguration should happen after all on-chain invocations.
    emit_new_block_and_epoch_event(&mut session, &module_storage, &mut traversal_context);

    let mut change_set = assert_ok!(session.finish(&genesis_change_set_configs, &module_storage,));

    // Publish the framework, using a different id, in case both sessions create tables.
    let mut new_id = [0u8; 32];
    new_id[31] = 1;

    let (additional_change_set, module_write_set) = publish_framework(
        &genesis_vm,
        &genesis_runtime_environment,
        HashValue::new(new_id),
        framework,
    );
    assert_ok!(change_set.squash_additional_change_set(additional_change_set));

    let change_set = assert_ok!(change_set.try_combine_into_storage_change_set(module_write_set));
    verify_genesis_module_write_set(change_set.write_set());
    verify_genesis_events(change_set.events());

    change_set
}

fn validate_genesis_config(genesis_config: &GenesisConfiguration) {
    assert!(
        genesis_config.min_stake <= genesis_config.max_stake,
        "Min stake must be smaller than or equal to max stake"
    );
    assert!(
        genesis_config.epoch_duration_secs > 0,
        "Epoch duration must be > 0"
    );
    assert!(
        genesis_config.recurring_lockup_duration_secs > 0,
        "Recurring lockup duration must be > 0"
    );
    assert!(
        genesis_config.recurring_lockup_duration_secs >= genesis_config.epoch_duration_secs,
        "Recurring lockup duration must be at least as long as epoch duration"
    );
    assert!(
        genesis_config.rewards_apy_percentage > 0 && genesis_config.rewards_apy_percentage < 100,
        "Rewards APY must be > 0% and < 100%"
    );
    assert!(
        genesis_config.voting_duration_secs > 0,
        "On-chain voting duration must be > 0"
    );
    assert!(
        genesis_config.voting_duration_secs < genesis_config.recurring_lockup_duration_secs,
        "Voting duration must be strictly smaller than recurring lockup"
    );
    assert!(
        genesis_config.voting_power_increase_limit > 0
            && genesis_config.voting_power_increase_limit <= 50,
        "voting_power_increase_limit must be > 0 and <= 50"
    );
}

fn exec_function_internal(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl ModuleStorage,
    traversal_context: &mut TraversalContext,
    module_name: &str,
    function_name: &str,
    ty_args: Vec<TypeTag>,
    args: Vec<Vec<u8>>,
    address: AccountAddress,
) {
    session
        .execute_function_bypass_visibility(
            &ModuleId::new(address, Identifier::new(module_name).unwrap()),
            &Identifier::new(function_name).unwrap(),
            ty_args,
            args,
            &mut UnmeteredGasMeter,
            traversal_context,
            module_storage,
        )
        .unwrap_or_else(|e| {
            panic!(
                "Error calling {}.{}.{}: ({:#x}) {}",
                address,
                module_name,
                function_name,
                e.sub_status().unwrap_or_default(),
                e,
            )
        });
}

fn exec_function(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl ModuleStorage,
    traversal_context: &mut TraversalContext,
    module_name: &str,
    function_name: &str,
    ty_args: Vec<TypeTag>,
    args: Vec<Vec<u8>>,
) {
    exec_function_internal(
        session,
        module_storage,
        traversal_context,
        module_name,
        function_name,
        ty_args,
        args,
        CORE_CODE_ADDRESS,
    );
}

fn exec_experimental_function(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl ModuleStorage,
    traversal_context: &mut TraversalContext,
    module_name: &str,
    function_name: &str,
    ty_args: Vec<TypeTag>,
    args: Vec<Vec<u8>>,
) {
    exec_function_internal(
        session,
        module_storage,
        traversal_context,
        module_name,
        function_name,
        ty_args,
        args,
        EXPERIMENTAL_CODE_ADDRESS,
    );
}

fn initialize(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
    chain_id: ChainId,
    genesis_config: &GenesisConfiguration,
    consensus_config: &OnChainConsensusConfig,
    execution_config: &OnChainExecutionConfig,
    gas_schedule: &GasScheduleV2,
) {
    let gas_schedule_blob =
        bcs::to_bytes(gas_schedule).expect("Failure serializing genesis gas schedule");

    let consensus_config_bytes =
        bcs::to_bytes(consensus_config).expect("Failure serializing genesis consensus config");

    let execution_config_bytes =
        bcs::to_bytes(execution_config).expect("Failure serializing genesis consensus config");

    // Calculate the per-epoch rewards rate, represented as 2 separate ints (numerator and
    // denominator).
    let rewards_rate_denominator = 1_000_000_000;
    let num_epochs_in_a_year = NUM_SECONDS_PER_YEAR / genesis_config.epoch_duration_secs;
    // Multiplication before division to minimize rounding errors due to integer division.
    let rewards_rate_numerator = (genesis_config.rewards_apy_percentage * rewards_rate_denominator
        / 100)
        / num_epochs_in_a_year;

    // Block timestamps are in microseconds and epoch_interval is used to check if a block timestamp
    // has crossed into a new epoch. So epoch_interval also needs to be in micro seconds.
    let epoch_interval_usecs = genesis_config.epoch_duration_secs * MICRO_SECONDS_PER_SECOND;
    exec_function(
        session,
        module_storage,
        traversal_context,
        GENESIS_MODULE_NAME,
        "initialize",
        vec![],
        serialize_values(&vec![
            MoveValue::vector_u8(gas_schedule_blob),
            MoveValue::U8(chain_id.id()),
            MoveValue::U64(APTOS_MAX_KNOWN_VERSION.major),
            MoveValue::vector_u8(consensus_config_bytes),
            MoveValue::vector_u8(execution_config_bytes),
            MoveValue::U64(epoch_interval_usecs),
            MoveValue::U64(genesis_config.min_stake),
            MoveValue::U64(genesis_config.max_stake),
            MoveValue::U64(genesis_config.recurring_lockup_duration_secs),
            MoveValue::Bool(genesis_config.allow_new_validators),
            MoveValue::U64(rewards_rate_numerator),
            MoveValue::U64(rewards_rate_denominator),
            MoveValue::U64(genesis_config.voting_power_increase_limit),
        ]),
    );
}

fn initialize_features(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
    features_override: Option<Vec<FeatureFlag>>,
) {
    let features: Vec<u64> = features_override
        .unwrap_or_else(FeatureFlag::default_features)
        .into_iter()
        .map(|feature| feature as u64)
        .collect();

    let mut serialized_values = serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS)]);
    serialized_values.push(bcs::to_bytes(&features).unwrap());
    serialized_values.push(bcs::to_bytes(&Vec::<u64>::new()).unwrap());

    exec_function(
        session,
        module_storage,
        traversal_context,
        "features",
        "change_feature_flags_internal",
        vec![],
        serialized_values,
    );
}

fn initialize_aptos_coin(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
) {
    exec_function(
        session,
        module_storage,
        traversal_context,
        GENESIS_MODULE_NAME,
        "initialize_aptos_coin",
        vec![],
        serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS)]),
    );
}

fn initialize_config_buffer(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
) {
    exec_function(
        session,
        module_storage,
        traversal_context,
        CONFIG_BUFFER_MODULE_NAME,
        "initialize",
        vec![],
        serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS)]),
    );
}

fn initialize_dkg(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
) {
    exec_function(
        session,
        module_storage,
        traversal_context,
        DKG_MODULE_NAME,
        "initialize",
        vec![],
        serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS)]),
    );
}

fn initialize_randomness_config_seqnum(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
) {
    exec_function(
        session,
        module_storage,
        traversal_context,
        RANDOMNESS_CONFIG_SEQNUM_MODULE_NAME,
        "initialize",
        vec![],
        serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS)]),
    );
}

fn initialize_randomness_api_v0_config(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
) {
    exec_function(
        session,
        module_storage,
        traversal_context,
        RANDOMNESS_API_V0_CONFIG_MODULE_NAME,
        "initialize",
        vec![],
        serialize_values(&vec![
            MoveValue::Signer(CORE_CODE_ADDRESS),
            RequiredGasDeposit::default_for_genesis().as_move_value(),
            AllowCustomMaxGasFlag::default_for_genesis().as_move_value(),
        ]),
    );
}

fn initialize_randomness_config(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
    randomness_config: OnChainRandomnessConfig,
) {
    exec_function(
        session,
        module_storage,
        traversal_context,
        RANDOMNESS_CONFIG_MODULE_NAME,
        "initialize",
        vec![],
        serialize_values(&vec![
            MoveValue::Signer(CORE_CODE_ADDRESS),
            RandomnessConfigMoveStruct::from(randomness_config).as_move_value(),
        ]),
    );
}

fn initialize_randomness_resources(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
) {
    exec_function(
        session,
        module_storage,
        traversal_context,
        RANDOMNESS_MODULE_NAME,
        "initialize",
        vec![],
        serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS)]),
    );
}

fn initialize_account_abstraction(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
) {
    exec_function(
        session,
        module_storage,
        traversal_context,
        ACCOUNT_ABSTRACTION_MODULE_NAME,
        "initialize",
        vec![],
        serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS)]),
    );

    exec_function(
        session,
        module_storage,
        traversal_context,
        ACCOUNT_ABSTRACTION_MODULE_NAME,
        "register_derivable_authentication_function",
        vec![],
        serialize_values(&vec![
            MoveValue::Signer(CORE_CODE_ADDRESS),
            MoveValue::Address(AccountAddress::SEVEN),
            "test_derivable_account_abstraction_ed25519_hex"
                .to_string()
                .as_move_value(),
            "authenticate".to_string().as_move_value(),
        ]),
    );

    exec_function(
        session,
        module_storage,
        traversal_context,
        ACCOUNT_ABSTRACTION_MODULE_NAME,
        "register_derivable_authentication_function",
        vec![],
        serialize_values(&vec![
            MoveValue::Signer(CORE_CODE_ADDRESS),
            MoveValue::Address(AccountAddress::ONE),
            "solana_derivable_account".to_string().as_move_value(),
            "authenticate".to_string().as_move_value(),
        ]),
    );

    exec_function(
        session,
        module_storage,
        traversal_context,
        ACCOUNT_ABSTRACTION_MODULE_NAME,
        "register_derivable_authentication_function",
        vec![],
        serialize_values(&vec![
            MoveValue::Signer(CORE_CODE_ADDRESS),
            MoveValue::Address(AccountAddress::ONE),
            "ethereum_derivable_account".to_string().as_move_value(),
            "authenticate".to_string().as_move_value(),
        ]),
    );
}

fn initialize_reconfiguration_state(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
) {
    exec_function(
        session,
        module_storage,
        traversal_context,
        RECONFIGURATION_STATE_MODULE_NAME,
        "initialize",
        vec![],
        serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS)]),
    );
}

fn initialize_jwk_consensus_config(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
    jwk_consensus_config: &OnChainJWKConsensusConfig,
) {
    exec_function(
        session,
        module_storage,
        traversal_context,
        JWK_CONSENSUS_CONFIG_MODULE_NAME,
        "initialize",
        vec![],
        serialize_values(&vec![
            MoveValue::Signer(CORE_CODE_ADDRESS),
            jwk_consensus_config.as_move_value(),
        ]),
    );
}

fn initialize_jwks_resources(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
) {
    exec_function(
        session,
        module_storage,
        traversal_context,
        JWKS_MODULE_NAME,
        "initialize",
        vec![],
        serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS)]),
    );
}

fn set_genesis_end(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
) {
    exec_function(
        session,
        module_storage,
        traversal_context,
        GENESIS_MODULE_NAME,
        "set_genesis_end",
        vec![],
        serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS)]),
    );
}

fn initialize_core_resources_and_aptos_coin(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
    core_resources_key: &Ed25519PublicKey,
) {
    let core_resources_auth_key = AuthenticationKey::ed25519(core_resources_key);
    exec_function(
        session,
        module_storage,
        traversal_context,
        GENESIS_MODULE_NAME,
        "initialize_core_resources_and_aptos_coin",
        vec![],
        serialize_values(&vec![
            MoveValue::Signer(CORE_CODE_ADDRESS),
            MoveValue::vector_u8(core_resources_auth_key.to_vec()),
        ]),
    );
}

/// Create and initialize Association and Core Code accounts.
fn initialize_on_chain_governance(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
    genesis_config: &GenesisConfiguration,
) {
    exec_function(
        session,
        module_storage,
        traversal_context,
        GOVERNANCE_MODULE_NAME,
        "initialize",
        vec![],
        serialize_values(&vec![
            MoveValue::Signer(CORE_CODE_ADDRESS),
            MoveValue::U128(genesis_config.min_voting_threshold),
            MoveValue::U64(genesis_config.required_proposer_stake),
            MoveValue::U64(genesis_config.voting_duration_secs),
        ]),
    );
}

fn initialize_keyless_accounts(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
    chain_id: ChainId,
    mut initial_jwks: Vec<IssuerJWK>,
    vk: Option<Groth16VerificationKey>,
) {
    let config = keyless::Configuration::new_for_devnet();
    exec_function(
        session,
        module_storage,
        traversal_context,
        KEYLESS_ACCOUNT_MODULE_NAME,
        "update_configuration",
        vec![],
        serialize_values(&vec![
            MoveValue::Signer(CORE_CODE_ADDRESS),
            config.as_move_value(),
        ]),
    );

    if vk.is_some() {
        exec_function(
            session,
            module_storage,
            traversal_context,
            KEYLESS_ACCOUNT_MODULE_NAME,
            "update_groth16_verification_key",
            vec![],
            serialize_values(&vec![
                MoveValue::Signer(CORE_CODE_ADDRESS),
                vk.unwrap().as_move_value(),
            ]),
        );
    }
    if !chain_id.is_mainnet() {
        let additional_jwk_patch = IssuerJWK {
            issuer: get_sample_iss(),
            jwk: JWK::RSA(secure_test_rsa_jwk()),
        };
        initial_jwks.insert(0, additional_jwk_patch);

        let jwk_patches: Vec<PatchJWKMoveStruct> = initial_jwks
            .into_iter()
            .map(|issuer_jwk| {
                let IssuerJWK { issuer, jwk } = issuer_jwk;
                let upsert_patch = PatchUpsertJWK {
                    issuer,
                    jwk: JWKMoveStruct::from(jwk),
                };
                PatchJWKMoveStruct::from(upsert_patch)
            })
            .collect();

        exec_function(
            session,
            module_storage,
            traversal_context,
            JWKS_MODULE_NAME,
            "set_patches",
            vec![],
            serialize_values(&vec![
                MoveValue::Signer(CORE_CODE_ADDRESS),
                jwk_patches.as_move_value(),
            ]),
        );
    }
}

fn initialize_confidential_asset(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    chain_id: ChainId,
    traversal_context: &mut TraversalContext,
) {
    if !chain_id.is_mainnet() && !chain_id.is_testnet() {
        exec_experimental_function(
            session,
            module_storage,
            traversal_context,
            "confidential_asset",
            "init_module_for_genesis",
            vec![],
            serialize_values(&vec![MoveValue::Signer(EXPERIMENTAL_CODE_ADDRESS)]),
        );
    }
}

fn create_accounts(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
    accounts: &[AccountBalance],
) {
    let accounts_bytes = bcs::to_bytes(accounts).expect("AccountMaps can be serialized");
    let mut serialized_values = serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS)]);
    serialized_values.push(accounts_bytes);
    exec_function(
        session,
        module_storage,
        traversal_context,
        GENESIS_MODULE_NAME,
        "create_accounts",
        vec![],
        serialized_values,
    );
}

fn create_employee_validators(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
    employees: &[EmployeePool],
    genesis_config: &GenesisConfiguration,
) {
    let employees_bytes = bcs::to_bytes(employees).expect("AccountMaps can be serialized");
    let mut serialized_values = serialize_values(&vec![
        MoveValue::U64(genesis_config.employee_vesting_start),
        MoveValue::U64(genesis_config.employee_vesting_period_duration),
    ]);
    serialized_values.push(employees_bytes);

    exec_function(
        session,
        module_storage,
        traversal_context,
        GENESIS_MODULE_NAME,
        "create_employee_validators",
        vec![],
        serialized_values,
    );
}

/// Creates and initializes each validator owner and validator operator. This method creates all
/// the required accounts, sets the validator operators for each validator owner, and sets the
/// validator config on-chain.
fn create_and_initialize_validators(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
    validators: &[Validator],
) {
    let validators_bytes = bcs::to_bytes(validators).expect("Validators can be serialized");
    let mut serialized_values = serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS)]);
    serialized_values.push(validators_bytes);
    exec_function(
        session,
        module_storage,
        traversal_context,
        GENESIS_MODULE_NAME,
        "create_initialize_validators",
        vec![],
        serialized_values,
    );
}

fn create_and_initialize_validators_with_commission(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
    validators: &[ValidatorWithCommissionRate],
) {
    let validators_bytes = bcs::to_bytes(validators).expect("Validators can be serialized");
    let mut serialized_values = serialize_values(&vec![
        MoveValue::Signer(CORE_CODE_ADDRESS),
        MoveValue::Bool(true),
    ]);
    serialized_values.push(validators_bytes);
    exec_function(
        session,
        module_storage,
        traversal_context,
        GENESIS_MODULE_NAME,
        "create_initialize_validators_with_commission",
        vec![],
        serialized_values,
    );
}

fn allow_core_resources_to_set_version(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
) {
    exec_function(
        session,
        module_storage,
        traversal_context,
        VERSION_MODULE_NAME,
        "initialize_for_test",
        vec![],
        serialize_values(&vec![MoveValue::Signer(aptos_test_root_address())]),
    );
}

fn initialize_package(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl ModuleStorage,
    traversal_context: &mut TraversalContext,
    addr: AccountAddress,
    package: &ReleasePackage,
) {
    exec_function(
        session,
        module_storage,
        traversal_context,
        CODE_MODULE_NAME,
        "initialize",
        vec![],
        vec![
            MoveValue::Signer(CORE_CODE_ADDRESS)
                .simple_serialize()
                .unwrap(),
            MoveValue::Signer(addr).simple_serialize().unwrap(),
            bcs::to_bytes(package.package_metadata()).unwrap(),
        ],
    );
}

fn code_to_writes_for_publishing(
    genesis_runtime_environment: &RuntimeEnvironment,
    genesis_features: &Features,
    genesis_state_view: &GenesisStateView,
    addr: AccountAddress,
    code: Vec<Bytes>,
) -> VMResult<BTreeMap<StateKey, ModuleWrite<WriteOp>>> {
    let module_storage = genesis_state_view.as_aptos_code_storage(genesis_runtime_environment);
    let resolver = genesis_state_view.as_move_resolver();

    let module_storage_with_staged_modules =
        StagingModuleStorage::create(&addr, &module_storage, code)?;
    let verified_module_bundle =
        module_storage_with_staged_modules.release_verified_module_bundle();

    convert_modules_into_write_ops(
        &resolver,
        genesis_features,
        &module_storage,
        verified_module_bundle,
    )
    .map_err(|e| e.finish(Location::Undefined))
}

/// Produces the changes when a framework is published:
///  1. Resources containing package information.
///  2. Module write set with published code.
fn publish_framework(
    genesis_vm: &GenesisMoveVm,
    genesis_runtime_environment: &RuntimeEnvironment,
    hash_value: HashValue,
    framework: &ReleaseBundle,
) -> (VMChangeSet, ModuleWriteSet) {
    // Reset state view to be empty, to make sure all module write ops are creations.
    let mut state_view = GenesisStateView::new();

    // First, publish modules.
    let mut writes = BTreeMap::new();
    for pack in &framework.packages {
        let modules = pack.sorted_code_and_modules();

        let addr = *modules.first().unwrap().1.self_id().address();
        let code = modules
            .into_iter()
            .map(|(c, _)| c.to_vec().into())
            .collect::<Vec<_>>();

        let package_writes = code_to_writes_for_publishing(
            genesis_runtime_environment,
            genesis_vm.genesis_features(),
            &state_view,
            addr,
            code,
        )
        .unwrap_or_else(|e| {
            panic!(
                "Failure publishing package `{}`: {:?}",
                pack.package_metadata().name,
                e
            )
        });

        // Add write ops so that we can later create a module write set. Also add them to the state
        // view so that modules in subsequent packages can link to them.
        writes.extend(package_writes.clone());
        state_view.add_module_write_ops(package_writes);
    }
    let module_write_set = ModuleWriteSet::new(writes);

    // At this point we processed all packages, and the state view contains all the code. We can
    // run package initialization.

    let module_storage = state_view.as_aptos_code_storage(genesis_runtime_environment);
    let resolver = state_view.as_move_resolver();
    let mut session = genesis_vm.new_genesis_session(&resolver, hash_value);

    let traversal_storage = TraversalStorage::new();
    let mut traversal_context = TraversalContext::new(&traversal_storage);

    for pack in &framework.packages {
        // Unfortunately, package does not contain address information, so we have to access its
        // modules to extract the destination address.
        let addr = *pack
            .sorted_code_and_modules()
            .first()
            .unwrap()
            .1
            .self_id()
            .address();
        initialize_package(
            &mut session,
            &module_storage,
            &mut traversal_context,
            addr,
            pack,
        );
    }

    let change_set =
        assert_ok!(session.finish(&genesis_vm.genesis_change_set_configs(), &module_storage,));
    (change_set, module_write_set)
}

/// Trigger a reconfiguration. This emits an event that will be passed along to the storage layer.
fn emit_new_block_and_epoch_event(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    traversal_context: &mut TraversalContext,
) {
    exec_function(
        session,
        module_storage,
        traversal_context,
        "block",
        "emit_genesis_block_event",
        vec![],
        serialize_values(&vec![MoveValue::Signer(
            account_config::reserved_vm_address(),
        )]),
    );
    exec_function(
        session,
        module_storage,
        traversal_context,
        "reconfiguration",
        "emit_genesis_reconfiguration_event",
        vec![],
        vec![],
    );
}

/// Verify the consistency of modules in the genesis write set.
fn verify_genesis_module_write_set(write_set: &WriteSet) {
    for (state_key, write_op) in write_set.expect_write_op_iter() {
        if state_key.is_module_path() {
            assert!(write_op.is_creation())
        }
    }
}

/// Verify the consistency of events emitted during genesis.
fn verify_genesis_events(events: &[ContractEvent]) {
    let new_epoch_events: Vec<&ContractEventV1> = events
        .iter()
        .filter_map(|e| {
            if e.event_key() == Some(&NewEpochEvent::event_key()) {
                Some(e.v1().unwrap())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        new_epoch_events.len(),
        1,
        "There should only be exactly one NewEpochEvent"
    );
    assert_eq!(new_epoch_events[0].sequence_number(), 0);
}

/// An enum specifying whether the compiled stdlib/scripts should be used or freshly built versions
/// should be used.
#[derive(Debug, Eq, PartialEq)]
pub enum GenesisOptions {
    /// Framework compiled from head
    Head,
    /// Framework as it was released or upgraded in testnet
    Testnet,
    /// Framework as it was released or upgraded in mainnet
    Mainnet,
}

/// Generate an artificial genesis `ChangeSet` for testing
pub fn generate_genesis_change_set_for_testing(genesis_options: GenesisOptions) -> ChangeSet {
    generate_genesis_change_set_for_testing_with_count(genesis_options, 1)
}

pub fn generate_genesis_change_set_for_testing_with_count(
    genesis_options: GenesisOptions,
    count: u64,
) -> ChangeSet {
    let framework = match genesis_options {
        GenesisOptions::Head => aptos_cached_packages::head_release_bundle(),
        GenesisOptions::Testnet => aptos_framework::testnet_release_bundle(),
        GenesisOptions::Mainnet => {
            // We don't yet have mainnet, so returning testnet here
            aptos_framework::testnet_release_bundle()
        },
    };

    generate_test_genesis(framework, Some(count as usize)).0
}

/// Generate a genesis `ChangeSet` for mainnet
pub fn generate_genesis_change_set_for_mainnet(genesis_options: GenesisOptions) -> ChangeSet {
    let framework = match genesis_options {
        GenesisOptions::Head => aptos_cached_packages::head_release_bundle(),
        GenesisOptions::Testnet => aptos_framework::testnet_release_bundle(),
        // We don't yet have mainnet, so returning testnet here
        GenesisOptions::Mainnet => aptos_framework::testnet_release_bundle(),
    };

    generate_mainnet_genesis(framework, Some(1)).0
}

pub fn test_genesis_transaction() -> Transaction {
    let changeset = test_genesis_change_set_and_validators(None).0;
    Transaction::GenesisTransaction(WriteSetPayload::Direct(changeset))
}

pub fn test_genesis_change_set_and_validators(
    count: Option<usize>,
) -> (ChangeSet, Vec<TestValidator>) {
    generate_test_genesis(aptos_cached_packages::head_release_bundle(), count)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validator {
    /// The Aptos account address of the validator or the admin in the case of a commissioned or
    /// vesting managed validator.
    pub owner_address: AccountAddress,
    /// The Aptos account address of the validator's operator (same as `address` if the validator is
    /// its own operator).
    pub operator_address: AccountAddress,
    pub voter_address: AccountAddress,
    /// Amount to stake for consensus. Also the intial amount minted to the owner account.
    pub stake_amount: u64,

    /// bls12381 public key used to sign consensus messages.
    pub consensus_pubkey: Vec<u8>,
    /// Proof of Possession of the consensus pubkey.
    pub proof_of_possession: Vec<u8>,
    /// `NetworkAddress` for the validator.
    pub network_addresses: Vec<u8>,
    /// `NetworkAddress` for the validator's full node.
    pub full_node_network_addresses: Vec<u8>,
}

pub struct TestValidator {
    pub key: Ed25519PrivateKey,
    pub consensus_key: bls12381::PrivateKey,
    pub data: Validator,
}

impl TestValidator {
    pub fn new_test_set(count: Option<usize>, initial_stake: Option<u64>) -> Vec<TestValidator> {
        let mut rng = rand::SeedableRng::from_seed([1u8; 32]);
        (0..count.unwrap_or(10))
            .map(|_| TestValidator::gen(&mut rng, initial_stake))
            .collect()
    }

    fn gen(rng: &mut StdRng, initial_stake: Option<u64>) -> TestValidator {
        let key = Ed25519PrivateKey::generate(rng);
        let auth_key = AuthenticationKey::ed25519(&key.public_key());
        let owner_address = auth_key.account_address();
        let consensus_key = bls12381::PrivateKey::generate(rng);
        let consensus_pubkey = consensus_key.public_key().to_bytes().to_vec();
        let proof_of_possession = bls12381::ProofOfPossession::create(&consensus_key)
            .to_bytes()
            .to_vec();
        let network_address = [0u8; 0].to_vec();
        let full_node_network_address = [0u8; 0].to_vec();

        let stake_amount = initial_stake.unwrap_or(1);
        let data = Validator {
            owner_address,
            consensus_pubkey,
            proof_of_possession,
            operator_address: owner_address,
            voter_address: owner_address,
            network_addresses: network_address,
            full_node_network_addresses: full_node_network_address,
            stake_amount,
        };
        Self {
            key,
            consensus_key,
            data,
        }
    }
}

pub fn generate_test_genesis(
    framework: &ReleaseBundle,
    count: Option<usize>,
) -> (ChangeSet, Vec<TestValidator>) {
    let test_validators = TestValidator::new_test_set(count, Some(100_000_000));
    let validators_: Vec<Validator> = test_validators.iter().map(|t| t.data.clone()).collect();
    let validators = &validators_;

    // Create override of features to ensure runtime checks are fully enabled
    let initial_features_override = Some(Features::default_for_tests());

    let genesis = encode_genesis_change_set(
        &GENESIS_KEYPAIR.1,
        validators,
        framework,
        ChainId::test(),
        &GenesisConfiguration {
            allow_new_validators: true,
            epoch_duration_secs: 3600,
            is_test: true,
            min_stake: 0,
            min_voting_threshold: 0,
            // 1M APTOS coins (with 8 decimals).
            max_stake: 100_000_000_000_000,
            recurring_lockup_duration_secs: 7200,
            required_proposer_stake: 0,
            rewards_apy_percentage: 10,
            voting_duration_secs: 3600,
            voting_power_increase_limit: 50,
            employee_vesting_start: 1663456089,
            employee_vesting_period_duration: 5 * 60, // 5 minutes
            initial_features_override,
            randomness_config_override: None,
            jwk_consensus_config_override: None,
            initial_jwks: vec![],
            keyless_groth16_vk: None,
        },
        &OnChainConsensusConfig::default_for_genesis(),
        &OnChainExecutionConfig::default_for_genesis(),
        &default_gas_schedule(),
    );
    (genesis, test_validators)
}

pub fn generate_mainnet_genesis(
    framework: &ReleaseBundle,
    count: Option<usize>,
) -> (ChangeSet, Vec<TestValidator>) {
    // TODO: Update to have custom validators/accounts with initial balances at genesis.
    let test_validators = TestValidator::new_test_set(count, Some(1_000_000_000_000_000));
    let validators_: Vec<Validator> = test_validators.iter().map(|t| t.data.clone()).collect();
    let validators = &validators_;

    let genesis = encode_genesis_change_set(
        &GENESIS_KEYPAIR.1,
        validators,
        framework,
        ChainId::test(),
        &mainnet_genesis_config(),
        &OnChainConsensusConfig::default_for_genesis(),
        &OnChainExecutionConfig::default_for_genesis(),
        &default_gas_schedule(),
    );
    (genesis, test_validators)
}

fn mainnet_genesis_config() -> GenesisConfiguration {
    // TODO: Update once mainnet numbers are decided. These numbers are just placeholders.
    GenesisConfiguration {
        allow_new_validators: true,
        epoch_duration_secs: 2 * 3600, // 2 hours
        is_test: false,
        min_stake: 1_000_000 * APTOS_COINS_BASE_WITH_DECIMALS, // 1M APT
        // 400M APT
        min_voting_threshold: (400_000_000 * APTOS_COINS_BASE_WITH_DECIMALS as u128),
        max_stake: 50_000_000 * APTOS_COINS_BASE_WITH_DECIMALS, // 50M APT.
        recurring_lockup_duration_secs: 30 * 24 * 3600,         // 1 month
        required_proposer_stake: 1_000_000 * APTOS_COINS_BASE_WITH_DECIMALS, // 1M APT
        rewards_apy_percentage: 10,
        voting_duration_secs: 7 * 24 * 3600, // 7 days
        voting_power_increase_limit: 30,
        employee_vesting_start: 1663456089,
        employee_vesting_period_duration: 5 * 60, // 5 minutes
        initial_features_override: None,
        randomness_config_override: None,
        jwk_consensus_config_override: None,
        initial_jwks: vec![],
        keyless_groth16_vk: None,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalance {
    pub account_address: AccountAddress,
    pub balance: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeePool {
    pub accounts: Vec<AccountAddress>,
    pub validator: ValidatorWithCommissionRate,
    pub vesting_schedule_numerators: Vec<u64>,
    pub vesting_schedule_denominator: u64,
    // Address that can reset the beneficiary for any shareholder.
    pub beneficiary_resetter: AccountAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorWithCommissionRate {
    pub validator: Validator,
    pub validator_commission_percentage: u64,
    /// Whether the validator should be joining the genesis validator set.
    pub join_during_genesis: bool,
}

#[test]
pub fn test_genesis_module_publishing() {
    let genesis_runtime_builder = GenesisRuntimeBuilder::new(ChainId::test());

    let genesis_vm = genesis_runtime_builder.build_genesis_vm();
    let genesis_runtime_environment = genesis_runtime_builder.build_genesis_runtime_environment();

    let (change_set, module_write_set) = publish_framework(
        &genesis_vm,
        &genesis_runtime_environment,
        HashValue::zero(),
        aptos_cached_packages::head_release_bundle(),
    );

    // All write ops must be a creation!
    let change_set = assert_ok!(change_set.try_combine_into_storage_change_set(module_write_set));
    verify_genesis_module_write_set(change_set.write_set());
}

#[test]
pub fn test_mainnet_end_to_end() {
    use aptos_types::{
        account_address, on_chain_config::ValidatorSet, state_store::state_key::StateKey,
        write_set::TransactionWrite,
    };

    let balance = 10_000_000 * APTOS_COINS_BASE_WITH_DECIMALS;
    let non_validator_balance = 10 * APTOS_COINS_BASE_WITH_DECIMALS;

    // currently just test that all functions have the right interface
    let account44 = AccountAddress::from_hex_literal("0x44").unwrap();
    let account45 = AccountAddress::from_hex_literal("0x45").unwrap();
    let account46 = AccountAddress::from_hex_literal("0x46").unwrap();
    let account47 = AccountAddress::from_hex_literal("0x47").unwrap();
    let account48 = AccountAddress::from_hex_literal("0x48").unwrap();
    let account49 = AccountAddress::from_hex_literal("0x49").unwrap();
    let operator0 = AccountAddress::from_hex_literal("0x100").unwrap();
    let operator1 = AccountAddress::from_hex_literal("0x101").unwrap();
    let operator2 = AccountAddress::from_hex_literal("0x102").unwrap();
    let operator3 = AccountAddress::from_hex_literal("0x103").unwrap();
    let operator4 = AccountAddress::from_hex_literal("0x104").unwrap();
    let operator5 = AccountAddress::from_hex_literal("0x105").unwrap();
    let voter0 = AccountAddress::from_hex_literal("0x200").unwrap();
    let voter1 = AccountAddress::from_hex_literal("0x201").unwrap();
    let voter2 = AccountAddress::from_hex_literal("0x202").unwrap();
    let voter3 = AccountAddress::from_hex_literal("0x203").unwrap();
    let admin0 = AccountAddress::from_hex_literal("0x300").unwrap();
    let admin1 = AccountAddress::from_hex_literal("0x301").unwrap();
    let admin2 = AccountAddress::from_hex_literal("0x302").unwrap();

    let accounts = vec![
        AccountBalance {
            account_address: account44,
            balance,
        },
        AccountBalance {
            account_address: account45,
            balance: balance * 3, // Three times the balance so it can host 2 operators.
        },
        AccountBalance {
            account_address: account46,
            balance,
        },
        AccountBalance {
            account_address: account47,
            balance,
        },
        AccountBalance {
            account_address: account48,
            balance,
        },
        AccountBalance {
            account_address: account49,
            balance,
        },
        AccountBalance {
            account_address: admin0,
            balance: non_validator_balance,
        },
        AccountBalance {
            account_address: admin1,
            balance: non_validator_balance,
        },
        AccountBalance {
            account_address: admin2,
            balance: non_validator_balance,
        },
        AccountBalance {
            account_address: operator0,
            balance: non_validator_balance,
        },
        AccountBalance {
            account_address: operator1,
            balance: non_validator_balance,
        },
        AccountBalance {
            account_address: operator2,
            balance: non_validator_balance,
        },
        AccountBalance {
            account_address: operator3,
            balance: non_validator_balance,
        },
        AccountBalance {
            account_address: operator4,
            balance: non_validator_balance,
        },
        AccountBalance {
            account_address: operator5,
            balance: non_validator_balance,
        },
        AccountBalance {
            account_address: voter0,
            balance: non_validator_balance,
        },
        AccountBalance {
            account_address: voter1,
            balance: non_validator_balance,
        },
        AccountBalance {
            account_address: voter2,
            balance: non_validator_balance,
        },
        AccountBalance {
            account_address: voter3,
            balance: non_validator_balance,
        },
    ];

    let test_validators = TestValidator::new_test_set(Some(6), Some(balance * 9 / 10));
    let mut employee_validator_1 = test_validators[0].data.clone();
    employee_validator_1.owner_address = admin0;
    employee_validator_1.operator_address = operator0;
    employee_validator_1.voter_address = voter0;
    let mut employee_validator_2 = test_validators[1].data.clone();
    employee_validator_2.owner_address = admin1;
    employee_validator_2.operator_address = operator1;
    employee_validator_2.voter_address = voter1;
    let mut zero_commission_validator = test_validators[2].data.clone();
    zero_commission_validator.owner_address = account44;
    zero_commission_validator.operator_address = operator2;
    zero_commission_validator.voter_address = voter2;
    let mut same_owner_validator_1 = test_validators[3].data.clone();
    same_owner_validator_1.owner_address = account45;
    same_owner_validator_1.operator_address = operator3;
    same_owner_validator_1.voter_address = voter3;
    let mut same_owner_validator_2 = test_validators[4].data.clone();
    same_owner_validator_2.owner_address = account45;
    same_owner_validator_2.operator_address = operator4;
    same_owner_validator_2.voter_address = voter3;
    let mut same_owner_validator_3 = test_validators[5].data.clone();
    same_owner_validator_3.owner_address = account45;
    same_owner_validator_3.operator_address = operator5;
    same_owner_validator_3.voter_address = voter3;

    let employees = vec![
        EmployeePool {
            accounts: vec![account46, account47],
            validator: ValidatorWithCommissionRate {
                validator: employee_validator_1,
                validator_commission_percentage: 10,
                join_during_genesis: true,
            },
            vesting_schedule_numerators: vec![3, 3, 3, 3, 1],
            vesting_schedule_denominator: 48,
            beneficiary_resetter: AccountAddress::ZERO,
        },
        EmployeePool {
            accounts: vec![account48, account49],
            validator: ValidatorWithCommissionRate {
                validator: employee_validator_2,
                validator_commission_percentage: 10,
                join_during_genesis: false,
            },
            vesting_schedule_numerators: vec![3, 3, 3, 3, 1],
            vesting_schedule_denominator: 48,
            beneficiary_resetter: account44,
        },
    ];

    let validators = vec![
        ValidatorWithCommissionRate {
            validator: same_owner_validator_1,
            validator_commission_percentage: 10,
            join_during_genesis: true,
        },
        ValidatorWithCommissionRate {
            validator: same_owner_validator_2,
            validator_commission_percentage: 15,
            join_during_genesis: true,
        },
        ValidatorWithCommissionRate {
            validator: same_owner_validator_3,
            validator_commission_percentage: 10,
            join_during_genesis: false,
        },
        ValidatorWithCommissionRate {
            validator: zero_commission_validator,
            validator_commission_percentage: 0,
            join_during_genesis: true,
        },
    ];

    let transaction = encode_aptos_mainnet_genesis_transaction(
        &accounts,
        &employees,
        &validators,
        aptos_cached_packages::head_release_bundle(),
        ChainId::mainnet(),
        &mainnet_genesis_config(),
    );

    let direct_writeset = if let Transaction::GenesisTransaction(direct_writeset) = transaction {
        direct_writeset
    } else {
        panic!("Invalid GenesisTransaction");
    };

    let changeset = if let WriteSetPayload::Direct(changeset) = direct_writeset {
        changeset
    } else {
        panic!("Invalid WriteSetPayload");
    };

    let writeset = changeset.write_set().as_v0();

    let state_key = StateKey::on_chain_config::<ValidatorSet>().unwrap();
    let bytes = writeset
        .get(&state_key)
        .unwrap()
        .extract_raw_bytes()
        .unwrap();
    let validator_set: ValidatorSet = bcs::from_bytes(&bytes).unwrap();
    let validator_set_addresses = validator_set
        .active_validators
        .iter()
        .map(|v| v.account_address)
        .collect::<Vec<_>>();

    let zero_commission_validator_pool_address =
        account_address::default_stake_pool_address(account44, operator2);
    let same_owner_validator_1_pool_address =
        account_address::default_stake_pool_address(account45, operator3);
    let same_owner_validator_2_pool_address =
        account_address::default_stake_pool_address(account45, operator4);
    let same_owner_validator_3_pool_address =
        account_address::default_stake_pool_address(account45, operator5);
    let employee_1_pool_address =
        account_address::create_vesting_pool_address(admin0, operator0, 0, &[]);
    let employee_2_pool_address =
        account_address::create_vesting_pool_address(admin1, operator1, 0, &[]);

    assert!(validator_set_addresses.contains(&zero_commission_validator_pool_address));
    assert!(validator_set_addresses.contains(&employee_1_pool_address));
    // This validator should not be in the genesis validator set as they specified
    // join_during_genesis = false.
    assert!(!validator_set_addresses.contains(&employee_2_pool_address));
    assert!(validator_set_addresses.contains(&same_owner_validator_1_pool_address));
    assert!(validator_set_addresses.contains(&same_owner_validator_2_pool_address));
    // This validator should not be in the genesis validator set as they specified
    // join_during_genesis = false.
    assert!(!validator_set_addresses.contains(&same_owner_validator_3_pool_address));
}
