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
use aptos_gas::{
    AbstractValueSizeGasParameters, AptosGasParameters, ChangeSetConfigs, InitialGasSchedule,
    NativeGasParameters, ToOnChainGasSchedule, LATEST_GAS_FEATURE_VERSION,
};
use aptos_types::{
    account_config::{self, aptos_test_root_address, events::NewEpochEvent, CORE_CODE_ADDRESS},
    chain_id::ChainId,
    contract_event::ContractEvent,
    on_chain_config::{
        FeatureFlag, Features, GasScheduleV2, OnChainConsensusConfig, OnChainExecutionConfig,
        TimedFeatures, APTOS_MAX_KNOWN_VERSION,
    },
    transaction::{authenticator::AuthenticationKey, ChangeSet, Transaction, WriteSetPayload},
};
use aptos_vm::{
    data_cache::AsMoveResolver,
    move_vm_ext::{MoveVmExt, SessionExt, SessionId},
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    value::{serialize_values, MoveValue},
};
use move_vm_types::gas::UnmeteredGasMeter;
use once_cell::sync::Lazy;
use rand::prelude::*;
use serde::{Deserialize, Serialize};

// The seed is arbitrarily picked to produce a consistent key. XXX make this more formal?
const GENESIS_SEED: [u8; 32] = [42; 32];

const GENESIS_MODULE_NAME: &str = "genesis";
const GOVERNANCE_MODULE_NAME: &str = "aptos_governance";
const CODE_MODULE_NAME: &str = "code";
const VERSION_MODULE_NAME: &str = "version";

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
        feature_version: aptos_gas::LATEST_GAS_FEATURE_VERSION,
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

    // Create a Move VM session so we can invoke on-chain genesis intializations.
    let mut state_view = GenesisStateView::new();
    for (module_bytes, module) in framework.code_and_compiled_modules() {
        state_view.add_module(&module.self_id(), module_bytes);
    }
    let data_cache = state_view.as_move_resolver();
    let move_vm = MoveVmExt::new(
        NativeGasParameters::zeros(),
        AbstractValueSizeGasParameters::zeros(),
        LATEST_GAS_FEATURE_VERSION,
        ChainId::test().id(),
        Features::default(),
        TimedFeatures::enable_all(),
    )
    .unwrap();
    let id1 = HashValue::zero();
    let mut session = move_vm.new_session(&data_cache, SessionId::genesis(id1), true);

    // On-chain genesis process.
    let consensus_config = OnChainConsensusConfig::default();
    let execution_config = OnChainExecutionConfig::default();
    let gas_schedule = default_gas_schedule();
    initialize(
        &mut session,
        chain_id,
        genesis_config,
        &consensus_config,
        &execution_config,
        &gas_schedule,
    );
    initialize_features(&mut session);
    initialize_aptos_coin(&mut session);
    initialize_on_chain_governance(&mut session, genesis_config);
    create_accounts(&mut session, accounts);
    create_employee_validators(&mut session, employees, genesis_config);
    create_and_initialize_validators_with_commission(&mut session, validators);
    set_genesis_end(&mut session);

    // Reconfiguration should happen after all on-chain invocations.
    emit_new_block_and_epoch_event(&mut session);

    let configs = ChangeSetConfigs::unlimited_at_gas_feature_version(LATEST_GAS_FEATURE_VERSION);
    let cs1 = session.finish(&mut (), &configs).unwrap();

    // Publish the framework, using a different session id, in case both scripts creates tables
    let state_view = GenesisStateView::new();
    let data_cache = state_view.as_move_resolver();

    let mut id2_arr = [0u8; 32];
    id2_arr[31] = 1;
    let id2 = HashValue::new(id2_arr);
    let mut session = move_vm.new_session(&data_cache, SessionId::genesis(id2), true);
    publish_framework(&mut session, framework);
    let cs2 = session.finish(&mut (), &configs).unwrap();
    let change_set = cs1.squash(cs2, &configs).unwrap();

    let (write_set, delta_change_set, events) = change_set.unpack();

    // Publishing stdlib should not produce any deltas around aggregators and map to write ops and
    // not deltas. The second session only publishes the framework module bundle, which should not
    // produce deltas either.
    assert!(
        delta_change_set.is_empty(),
        "non-empty delta change set in genesis"
    );

    assert!(!write_set.iter().any(|(_, op)| op.is_deletion()));
    verify_genesis_write_set(&events);
    let change_set = ChangeSet::new(write_set, events);
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

    // Create a Move VM session so we can invoke on-chain genesis intializations.
    let mut state_view = GenesisStateView::new();
    for (module_bytes, module) in framework.code_and_compiled_modules() {
        state_view.add_module(&module.self_id(), module_bytes);
    }
    let data_cache = state_view.as_move_resolver();
    let move_vm = MoveVmExt::new(
        NativeGasParameters::zeros(),
        AbstractValueSizeGasParameters::zeros(),
        LATEST_GAS_FEATURE_VERSION,
        ChainId::test().id(),
        Features::default(),
        TimedFeatures::enable_all(),
    )
    .unwrap();
    let id1 = HashValue::zero();
    let mut session = move_vm.new_session(&data_cache, SessionId::genesis(id1), true);

    // On-chain genesis process.
    initialize(
        &mut session,
        chain_id,
        genesis_config,
        consensus_config,
        execution_config,
        gas_schedule,
    );
    initialize_features(&mut session);
    if genesis_config.is_test {
        initialize_core_resources_and_aptos_coin(&mut session, core_resources_key);
    } else {
        initialize_aptos_coin(&mut session);
    }
    initialize_on_chain_governance(&mut session, genesis_config);
    create_and_initialize_validators(&mut session, validators);
    if genesis_config.is_test {
        allow_core_resources_to_set_version(&mut session);
    }
    set_genesis_end(&mut session);

    // Reconfiguration should happen after all on-chain invocations.
    emit_new_block_and_epoch_event(&mut session);

    let configs = ChangeSetConfigs::unlimited_at_gas_feature_version(LATEST_GAS_FEATURE_VERSION);
    let cs1 = session.finish(&mut (), &configs).unwrap();

    let state_view = GenesisStateView::new();
    let data_cache = state_view.as_move_resolver();

    // Publish the framework, using a different session id, in case both scripts creates tables
    let mut id2_arr = [0u8; 32];
    id2_arr[31] = 1;
    let id2 = HashValue::new(id2_arr);
    let mut session = move_vm.new_session(&data_cache, SessionId::genesis(id2), true);
    publish_framework(&mut session, framework);
    let cs2 = session.finish(&mut (), &configs).unwrap();
    let change_set = cs1.squash(cs2, &configs).unwrap();

    let (write_set, delta_change_set, events) = change_set.unpack();

    // Publishing stdlib should not produce any deltas around aggregators and map to write ops and
    // not deltas. The second session only publishes the framework module bundle, which should not
    // produce deltas either.
    assert!(
        delta_change_set.is_empty(),
        "non-empty delta change set in genesis"
    );

    assert!(!write_set.iter().any(|(_, op)| op.is_deletion()));
    verify_genesis_write_set(&events);
    ChangeSet::new(write_set, events)
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

fn exec_function(
    session: &mut SessionExt,
    module_name: &str,
    function_name: &str,
    ty_args: Vec<TypeTag>,
    args: Vec<Vec<u8>>,
) {
    session
        .execute_function_bypass_visibility(
            &ModuleId::new(
                account_config::CORE_CODE_ADDRESS,
                Identifier::new(module_name).unwrap(),
            ),
            &Identifier::new(function_name).unwrap(),
            ty_args,
            args,
            &mut UnmeteredGasMeter,
        )
        .unwrap_or_else(|e| {
            panic!(
                "Error calling {}.{}: ({:#x}) {}",
                module_name,
                function_name,
                e.sub_status().unwrap_or_default(),
                e,
            )
        });
}

fn initialize(
    session: &mut SessionExt,
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

pub fn default_features() -> Vec<FeatureFlag> {
    vec![
        FeatureFlag::CODE_DEPENDENCY_CHECK,
        FeatureFlag::TREAT_FRIEND_AS_PRIVATE,
        FeatureFlag::SHA_512_AND_RIPEMD_160_NATIVES,
        FeatureFlag::APTOS_STD_CHAIN_ID_NATIVES,
        FeatureFlag::VM_BINARY_FORMAT_V6,
        FeatureFlag::MULTI_ED25519_PK_VALIDATE_V2_NATIVES,
        FeatureFlag::BLAKE2B_256_NATIVE,
        FeatureFlag::RESOURCE_GROUPS,
        FeatureFlag::MULTISIG_ACCOUNTS,
        FeatureFlag::DELEGATION_POOLS,
        FeatureFlag::ED25519_PUBKEY_VALIDATE_RETURN_FALSE_WRONG_LENGTH,
        FeatureFlag::STRUCT_CONSTRUCTORS,
        FeatureFlag::CRYPTOGRAPHY_ALGEBRA_NATIVES,
        FeatureFlag::BLS12_381_STRUCTURES,
        FeatureFlag::CHARGE_INVARIANT_VIOLATION,
        FeatureFlag::APTOS_UNIQUE_IDENTIFIERS,
        FeatureFlag::GAS_PAYER_ENABLED,
    ]
}

fn initialize_features(session: &mut SessionExt) {
    let features: Vec<u64> = default_features()
        .into_iter()
        .map(|feature| feature as u64)
        .collect();

    let mut serialized_values = serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS)]);
    serialized_values.push(bcs::to_bytes(&features).unwrap());
    serialized_values.push(bcs::to_bytes(&Vec::<u64>::new()).unwrap());

    exec_function(
        session,
        "features",
        "change_feature_flags",
        vec![],
        serialized_values,
    );
}

fn initialize_aptos_coin(session: &mut SessionExt) {
    exec_function(
        session,
        GENESIS_MODULE_NAME,
        "initialize_aptos_coin",
        vec![],
        serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS)]),
    );
}

fn set_genesis_end(session: &mut SessionExt) {
    exec_function(
        session,
        GENESIS_MODULE_NAME,
        "set_genesis_end",
        vec![],
        serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS)]),
    );
}

fn initialize_core_resources_and_aptos_coin(
    session: &mut SessionExt,
    core_resources_key: &Ed25519PublicKey,
) {
    let core_resources_auth_key = AuthenticationKey::ed25519(core_resources_key);
    exec_function(
        session,
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
fn initialize_on_chain_governance(session: &mut SessionExt, genesis_config: &GenesisConfiguration) {
    exec_function(
        session,
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

fn create_accounts(session: &mut SessionExt, accounts: &[AccountBalance]) {
    let accounts_bytes = bcs::to_bytes(accounts).expect("AccountMaps can be serialized");
    let mut serialized_values = serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS)]);
    serialized_values.push(accounts_bytes);
    exec_function(
        session,
        GENESIS_MODULE_NAME,
        "create_accounts",
        vec![],
        serialized_values,
    );
}

fn create_employee_validators(
    session: &mut SessionExt,
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
        GENESIS_MODULE_NAME,
        "create_employee_validators",
        vec![],
        serialized_values,
    );
}

/// Creates and initializes each validator owner and validator operator. This method creates all
/// the required accounts, sets the validator operators for each validator owner, and sets the
/// validator config on-chain.
fn create_and_initialize_validators(session: &mut SessionExt, validators: &[Validator]) {
    let validators_bytes = bcs::to_bytes(validators).expect("Validators can be serialized");
    let mut serialized_values = serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS)]);
    serialized_values.push(validators_bytes);
    exec_function(
        session,
        GENESIS_MODULE_NAME,
        "create_initialize_validators",
        vec![],
        serialized_values,
    );
}

fn create_and_initialize_validators_with_commission(
    session: &mut SessionExt,
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
        GENESIS_MODULE_NAME,
        "create_initialize_validators_with_commission",
        vec![],
        serialized_values,
    );
}

fn allow_core_resources_to_set_version(session: &mut SessionExt) {
    exec_function(
        session,
        VERSION_MODULE_NAME,
        "initialize_for_test",
        vec![],
        serialize_values(&vec![MoveValue::Signer(aptos_test_root_address())]),
    );
}

/// Publish the framework release bundle.
fn publish_framework(session: &mut SessionExt, framework: &ReleaseBundle) {
    for pack in &framework.packages {
        publish_package(session, pack)
    }
}

/// Publish the given package.
fn publish_package(session: &mut SessionExt, pack: &ReleasePackage) {
    let modules = pack.sorted_code_and_modules();
    let addr = *modules.first().unwrap().1.self_id().address();
    let code = modules
        .into_iter()
        .map(|(c, _)| c.to_vec())
        .collect::<Vec<_>>();
    session
        .publish_module_bundle(code, addr, &mut UnmeteredGasMeter)
        .unwrap_or_else(|e| {
            panic!(
                "Failure publishing package `{}`: {:?}",
                pack.package_metadata().name,
                e
            )
        });

    // Call the initialize function with the metadata.
    exec_function(session, CODE_MODULE_NAME, "initialize", vec![], vec![
        MoveValue::Signer(CORE_CODE_ADDRESS)
            .simple_serialize()
            .unwrap(),
        MoveValue::Signer(addr).simple_serialize().unwrap(),
        bcs::to_bytes(pack.package_metadata()).unwrap(),
    ]);
}

/// Trigger a reconfiguration. This emits an event that will be passed along to the storage layer.
fn emit_new_block_and_epoch_event(session: &mut SessionExt) {
    exec_function(
        session,
        "block",
        "emit_genesis_block_event",
        vec![],
        serialize_values(&vec![MoveValue::Signer(
            account_config::reserved_vm_address(),
        )]),
    );
    exec_function(
        session,
        "reconfiguration",
        "emit_genesis_reconfiguration_event",
        vec![],
        vec![],
    );
}

/// Verify the consistency of the genesis `WriteSet`
fn verify_genesis_write_set(events: &[ContractEvent]) {
    let new_epoch_events: Vec<&ContractEvent> = events
        .iter()
        .filter(|e| e.key() == &NewEpochEvent::event_key())
        .collect();
    assert_eq!(
        new_epoch_events.len(),
        1,
        "There should only be exactly one NewEpochEvent"
    );
    assert_eq!(new_epoch_events[0].sequence_number(), 0,);
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
        let owner_address = auth_key.derived_address();
        let consensus_key = bls12381::PrivateKey::generate(rng);
        let consensus_pubkey = consensus_key.public_key().to_bytes().to_vec();
        let proof_of_possession = bls12381::ProofOfPossession::create(&consensus_key)
            .to_bytes()
            .to_vec();
        let network_address = [0u8; 0].to_vec();
        let full_node_network_address = [0u8; 0].to_vec();

        let stake_amount = if let Some(amount) = initial_stake {
            amount
        } else {
            1
        };
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
        },
        &OnChainConsensusConfig::default(),
        &OnChainExecutionConfig::default(),
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
        &OnChainConsensusConfig::default(),
        &OnChainExecutionConfig::default(),
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
    // create a state view for move_vm
    let mut state_view = GenesisStateView::new();
    for (module_bytes, module) in
        aptos_cached_packages::head_release_bundle().code_and_compiled_modules()
    {
        state_view.add_module(&module.self_id(), module_bytes);
    }
    let data_cache = state_view.as_move_resolver();

    let move_vm = MoveVmExt::new(
        NativeGasParameters::zeros(),
        AbstractValueSizeGasParameters::zeros(),
        LATEST_GAS_FEATURE_VERSION,
        ChainId::test().id(),
        Features::default(),
        TimedFeatures::enable_all(),
    )
    .unwrap();
    let id1 = HashValue::zero();
    let mut session = move_vm.new_session(&data_cache, SessionId::genesis(id1), true);
    publish_framework(&mut session, aptos_cached_packages::head_release_bundle());
}

#[test]
pub fn test_mainnet_end_to_end() {
    use aptos_types::{
        account_address,
        on_chain_config::{OnChainConfig, ValidatorSet},
        state_store::state_key::StateKey,
        write_set::{TransactionWrite, WriteSet},
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

    let WriteSet::V0(writeset) = changeset.write_set();

    let state_key =
        StateKey::access_path(ValidatorSet::access_path().expect("access path in test"));
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
