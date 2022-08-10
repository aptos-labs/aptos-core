// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

mod genesis_context;

use crate::genesis_context::GenesisStateView;
use aptos_crypto::{
    bls12381,
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    HashValue, PrivateKey, Uniform,
};
use aptos_gas::{
    AptosGasParameters, InitialGasSchedule, NativeGasParameters, ToOnChainGasSchedule,
};
use aptos_types::{
    account_config::{self, events::NewEpochEvent, CORE_CODE_ADDRESS},
    chain_id::ChainId,
    contract_event::ContractEvent,
    on_chain_config::{ConsensusConfigV1, OnChainConsensusConfig, APTOS_MAX_KNOWN_VERSION},
    transaction::{authenticator::AuthenticationKey, ChangeSet, Transaction, WriteSetPayload},
};
use aptos_vm::{
    data_cache::{IntoMoveResolver, StateViewCache},
    move_vm_ext::{MoveVmExt, SessionExt, SessionId},
};
use move_deps::move_binary_format::access::ModuleAccess;
use move_deps::{
    move_binary_format::CompiledModule,
    move_bytecode_utils::Modules,
    move_core_types::{
        account_address::AccountAddress,
        identifier::Identifier,
        language_storage::{ModuleId, TypeTag},
        resolver::MoveResolver,
        value::{serialize_values, MoveValue},
    },
    move_vm_types::gas::UnmeteredGasMeter,
};
use once_cell::sync::Lazy;
use rand::prelude::*;
use std::collections::{HashMap, HashSet};

// The seed is arbitrarily picked to produce a consistent key. XXX make this more formal?
const GENESIS_SEED: [u8; 32] = [42; 32];

const GENESIS_MODULE_NAME: &str = "genesis";
const GOVERNANCE_MODULE_NAME: &str = "aptos_governance";

const NUM_SECONDS_PER_YEAR: u64 = 365 * 24 * 60 * 60;
const MICRO_SECONDS_PER_SECOND: u64 = 1_000_000;

pub struct GenesisConfiguration {
    pub allow_new_validators: bool,
    pub epoch_duration_secs: u64,
    pub max_stake: u64,
    pub min_stake: u64,
    pub min_voting_threshold: u128,
    pub recurring_lockup_duration_secs: u64,
    pub required_proposer_stake: u64,
    pub rewards_apy_percentage: u64,
    pub voting_duration_secs: u64,
}

pub static GENESIS_KEYPAIR: Lazy<(Ed25519PrivateKey, Ed25519PublicKey)> = Lazy::new(|| {
    let mut rng = StdRng::from_seed(GENESIS_SEED);
    let private_key = Ed25519PrivateKey::generate(&mut rng);
    let public_key = private_key.public_key();
    (private_key, public_key)
});

pub fn encode_genesis_transaction(
    aptos_root_key: Ed25519PublicKey,
    validators: &[Validator],
    stdlib_module_bytes: &[Vec<u8>],
    chain_id: ChainId,
    genesis_config: GenesisConfiguration,
) -> Transaction {
    let consensus_config = OnChainConsensusConfig::V1(ConsensusConfigV1::default());

    Transaction::GenesisTransaction(WriteSetPayload::Direct(encode_genesis_change_set(
        &aptos_root_key,
        validators,
        stdlib_module_bytes,
        consensus_config,
        chain_id,
        &genesis_config,
    )))
}

pub fn encode_genesis_change_set(
    aptos_root_key: &Ed25519PublicKey,
    validators: &[Validator],
    stdlib_module_bytes: &[Vec<u8>],
    consensus_config: OnChainConsensusConfig,
    chain_id: ChainId,
    genesis_config: &GenesisConfiguration,
) -> ChangeSet {
    validate_genesis_config(genesis_config);

    let mut stdlib_modules = Vec::new();
    // create a data view for move_vm
    let mut state_view = GenesisStateView::new();
    for module_bytes in stdlib_module_bytes {
        let module = CompiledModule::deserialize(module_bytes).unwrap();
        state_view.add_module(&module.self_id(), module_bytes);
        stdlib_modules.push(module)
    }
    let data_cache = StateViewCache::new(&state_view).into_move_resolver();

    let move_vm = MoveVmExt::new(NativeGasParameters::zeros()).unwrap();
    let id1 = HashValue::zero();
    let mut session = move_vm.new_session(&data_cache, SessionId::genesis(id1));

    create_and_initialize_main_accounts(
        &mut session,
        aptos_root_key,
        consensus_config,
        chain_id,
        genesis_config,
    );
    // generate the genesis WriteSet
    create_and_initialize_validators(&mut session, validators);

    // Initialize on-chain governance.
    initialize_on_chain_governance(&mut session, genesis_config);

    // Reconfiguration should happen after all on-chain invocations.
    emit_new_block_and_epoch_event(&mut session);

    let mut session1_out = session.finish().unwrap();

    let state_view = GenesisStateView::new();
    let data_cache = StateViewCache::new(&state_view).into_move_resolver();

    // use a different session id, in case both scripts creates tables
    let mut id2_arr = [0u8; 32];
    id2_arr[31] = 1;
    let id2 = HashValue::new(id2_arr);
    let mut session = move_vm.new_session(&data_cache, SessionId::genesis(id2));
    publish_stdlib(&mut session, stdlib_modules);
    let session2_out = session.finish().unwrap();

    session1_out.squash(session2_out).unwrap();
    let change_set = session1_out.into_change_set(&mut ()).unwrap();

    assert!(!change_set
        .write_set()
        .iter()
        .any(|(_, op)| op.is_deletion()));
    verify_genesis_write_set(change_set.events());
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
}

fn exec_function(
    session: &mut SessionExt<impl MoveResolver>,
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
                "Error calling {}.{}: {}",
                module_name,
                function_name,
                e.into_vm_status()
            )
        });
}

/// Create and initialize Association and Core Code accounts.
fn create_and_initialize_main_accounts(
    session: &mut SessionExt<impl MoveResolver>,
    aptos_root_key: &Ed25519PublicKey,
    consensus_config: OnChainConsensusConfig,
    chain_id: ChainId,
    genesis_config: &GenesisConfiguration,
) {
    let aptos_root_auth_key = AuthenticationKey::ed25519(aptos_root_key);

    let genesis_gas_params = AptosGasParameters::initial();
    let gas_schedule_blob = bcs::to_bytes(&genesis_gas_params.to_on_chain_gas_schedule())
        .expect("Failure serializing genesis gas schedule");

    let consensus_config_bytes =
        bcs::to_bytes(&consensus_config).expect("Failure serializing genesis consensus config");

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
            MoveValue::Signer(account_config::aptos_root_address()),
            MoveValue::vector_u8(aptos_root_auth_key.to_vec()),
            MoveValue::vector_u8(gas_schedule_blob),
            MoveValue::U8(chain_id.id()),
            MoveValue::U64(APTOS_MAX_KNOWN_VERSION.major),
            MoveValue::vector_u8(consensus_config_bytes),
            MoveValue::U64(epoch_interval_usecs),
            MoveValue::U64(genesis_config.min_stake),
            MoveValue::U64(genesis_config.max_stake),
            MoveValue::U64(genesis_config.recurring_lockup_duration_secs),
            MoveValue::Bool(genesis_config.allow_new_validators),
            MoveValue::U64(rewards_rate_numerator),
            MoveValue::U64(rewards_rate_denominator),
        ]),
    );
}

/// Create and initialize Association and Core Code accounts.
fn initialize_on_chain_governance(
    session: &mut SessionExt<impl MoveResolver>,
    genesis_config: &GenesisConfiguration,
) {
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

/// Creates and initializes each validator owner and validator operator. This method creates all
/// the required accounts, sets the validator operators for each validator owner, and sets the
/// validator config on-chain.
fn create_and_initialize_validators(
    session: &mut SessionExt<impl MoveResolver>,
    validators: &[Validator],
) {
    let mut owners = vec![];
    let mut consensus_pubkeys = vec![];
    let mut proof_of_possession = vec![];
    let mut validator_network_addresses = vec![];
    let mut full_node_network_addresses = vec![];
    let mut staking_distribution = vec![];

    for v in validators {
        owners.push(MoveValue::Address(v.address));
        consensus_pubkeys.push(MoveValue::vector_u8(v.consensus_pubkey.clone()));
        proof_of_possession.push(MoveValue::vector_u8(v.proof_of_possession.clone()));
        validator_network_addresses.push(MoveValue::vector_u8(v.network_addresses.clone()));
        full_node_network_addresses
            .push(MoveValue::vector_u8(v.full_node_network_addresses.clone()));
        staking_distribution.push(MoveValue::U64(v.stake_amount));
    }
    exec_function(
        session,
        GENESIS_MODULE_NAME,
        "create_initialize_validators",
        vec![],
        serialize_values(&vec![
            MoveValue::Signer(CORE_CODE_ADDRESS),
            MoveValue::Vector(owners),
            MoveValue::Vector(consensus_pubkeys),
            MoveValue::Vector(proof_of_possession),
            MoveValue::Vector(validator_network_addresses),
            MoveValue::Vector(full_node_network_addresses),
            MoveValue::Vector(staking_distribution),
        ]),
    );
}

/// Collect compiledModule based on account address, dedup modules for each address
fn construct_module_map(
    modules: Vec<CompiledModule>,
) -> HashMap<AccountAddress, Vec<CompiledModule>> {
    let mut module_ids = HashSet::new();
    let mut map = HashMap::new();
    for m in modules {
        if module_ids.insert(m.self_id()) {
            map.entry(*m.address())
                .or_insert_with(Vec::new)
                .push(m.clone());
        }
    }
    map
}

/// Publish all modules that should be available after genesis.
fn publish_stdlib(session: &mut SessionExt<impl MoveResolver>, stdlib: Vec<CompiledModule>) {
    let map = construct_module_map(stdlib);
    let root_address = AccountAddress::from_hex_literal("0x1").unwrap();
    let token_address = AccountAddress::from_hex_literal("0x3").unwrap();

    let framework_modules = map.get(&root_address).unwrap();
    let token_modules = map.get(&token_address).unwrap();

    // publish core-framework
    publish_module_bundle(session, Modules::new(framework_modules));
    // publish non-core-framework modules
    publish_token_modules(session, token_modules.clone());
}

/// publish modules that are not core-framework. assuming core-framework published
/// the modules has to be sorted by topological order PropertyMap -> TokenV1 -> TokenCoinSwap
fn publish_token_modules(
    session: &mut SessionExt<impl MoveResolver>,
    mut lib: Vec<CompiledModule>,
) {
    // module topological order
    let x: HashMap<&str, u32> = HashMap::from([
        ("property_map", 0u32),
        ("token", 1u32),
        ("token_coin_swap", 2u32),
        ("token_transfers", 3u32),
    ])
    .into_iter()
    .collect();

    lib.sort_by_key(|m| x.get(m.name().as_str()).unwrap());

    for m in lib {
        let module_id = m.self_id();
        if module_id.name().as_str() == GENESIS_MODULE_NAME {
            // Do not publish the Genesis module
            continue;
        }
        let mut bytes = vec![];
        m.serialize(&mut bytes).unwrap();
        session
            .publish_module(bytes, *module_id.address(), &mut UnmeteredGasMeter)
            .unwrap_or_else(|e| panic!("Failure publishing module {:?}, {:?}", module_id, e));
    }
}

/// publish the core-framework with stdlib
fn publish_module_bundle(session: &mut SessionExt<impl MoveResolver>, lib: Modules) {
    let dep_graph = lib.compute_dependency_graph();
    let mut addr_opt: Option<AccountAddress> = None;
    let modules = dep_graph
        .compute_topological_order()
        .unwrap()
        .map(|m| {
            let addr = *m.self_id().address();
            if let Some(a) = addr_opt {
                assert_eq!(
                    a,
                    addr,
                    "All genesis modules must be published under the same address, but found modules under both {} and {}",
                    a.short_str_lossless(),
                    addr.short_str_lossless(),
                );
            } else {
                addr_opt = Some(addr)
            }
            let mut bytes = vec![];
            m.serialize(&mut bytes).unwrap();
            bytes
        })
        .collect::<Vec<Vec<u8>>>();
    // TODO: allow genesis modules published under different addresses. supporting this while
    // maintaining the topological order is challenging.
    session
        .publish_module_bundle(modules, addr_opt.unwrap(), &mut UnmeteredGasMeter)
        .unwrap_or_else(|e| panic!("Failure publishing modules {:?}", e));
}

/// Trigger a reconfiguration. This emits an event that will be passed along to the storage layer.
fn emit_new_block_and_epoch_event(session: &mut SessionExt<impl MoveResolver>) {
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
    Compiled,
    Fresh,
}

/// Generate an artificial genesis `ChangeSet` for testing
pub fn generate_genesis_change_set_for_testing(genesis_options: GenesisOptions) -> ChangeSet {
    let modules = match genesis_options {
        GenesisOptions::Compiled => cached_framework_packages::module_blobs().to_vec(),
        GenesisOptions::Fresh => framework::aptos::module_blobs(),
    };

    generate_test_genesis(&modules, None).0
}

pub fn test_genesis_transaction() -> Transaction {
    let changeset = test_genesis_change_set_and_validators(None).0;
    Transaction::GenesisTransaction(WriteSetPayload::Direct(changeset))
}

pub fn test_genesis_change_set_and_validators(
    count: Option<usize>,
) -> (ChangeSet, Vec<TestValidator>) {
    generate_test_genesis(cached_framework_packages::module_blobs(), count)
}

#[derive(Debug, Clone)]
pub struct Validator {
    /// The Aptos account address of the validator
    pub address: AccountAddress,
    /// bls12381 public key used to sign consensus messages
    pub consensus_pubkey: Vec<u8>,
    /// Proof of Possession of the consensus pubkey
    pub proof_of_possession: Vec<u8>,
    /// The Aptos account address of the validator's operator (same as `address` if the validator is
    /// its own operator)
    pub operator_address: AccountAddress,
    /// `NetworkAddress` for the validator
    pub network_addresses: Vec<u8>,
    /// `NetworkAddress` for the validator's full node
    pub full_node_network_addresses: Vec<u8>,
    /// Amount to stake for consensus
    pub stake_amount: u64,
}

pub struct TestValidator {
    pub key: Ed25519PrivateKey,
    pub consensus_key: bls12381::PrivateKey,
    pub data: Validator,
}

impl TestValidator {
    pub fn new_test_set(count: Option<usize>) -> Vec<TestValidator> {
        let mut rng: rand::rngs::StdRng = rand::SeedableRng::from_seed([1u8; 32]);
        (0..count.unwrap_or(10))
            .map(|_| TestValidator::gen(&mut rng))
            .collect()
    }

    fn gen(rng: &mut rand::rngs::StdRng) -> TestValidator {
        let key = Ed25519PrivateKey::generate(rng);
        let auth_key = AuthenticationKey::ed25519(&key.public_key());
        let address = auth_key.derived_address();
        let consensus_key = bls12381::PrivateKey::generate(rng);
        let consensus_pubkey = consensus_key.public_key().to_bytes().to_vec();
        let proof_of_possession = bls12381::ProofOfPossession::create(&consensus_key)
            .to_bytes()
            .to_vec();
        let network_address = [0u8; 0].to_vec();
        let full_node_network_address = [0u8; 0].to_vec();

        let data = Validator {
            address,
            consensus_pubkey,
            proof_of_possession,
            operator_address: address,
            network_addresses: network_address,
            full_node_network_addresses: full_node_network_address,
            stake_amount: 1,
        };
        Self {
            key,
            consensus_key,
            data,
        }
    }
}

pub fn generate_test_genesis(
    stdlib_modules: &[Vec<u8>],
    count: Option<usize>,
) -> (ChangeSet, Vec<TestValidator>) {
    let test_validators = TestValidator::new_test_set(count);
    let validators_: Vec<Validator> = test_validators.iter().map(|t| t.data.clone()).collect();
    let validators = &validators_;

    let genesis = encode_genesis_change_set(
        &GENESIS_KEYPAIR.1,
        validators,
        stdlib_modules,
        OnChainConsensusConfig::default(),
        ChainId::test(),
        &GenesisConfiguration {
            allow_new_validators: true,
            epoch_duration_secs: 3600,
            min_stake: 0,
            min_voting_threshold: 0,
            // 1M APTOS coins (with 8 decimals).
            max_stake: 100_000_000_000_000,
            recurring_lockup_duration_secs: 7200,
            required_proposer_stake: 0,
            rewards_apy_percentage: 10,
            voting_duration_secs: 3600,
        },
    );
    (genesis, test_validators)
}

#[test]
pub fn test_genesis_module_publishing() {
    let mut stdlib_modules = Vec::new();
    // create a data view for move_vm
    let mut state_view = GenesisStateView::new();
    for module_bytes in cached_framework_packages::module_blobs() {
        let module = CompiledModule::deserialize(module_bytes).unwrap();
        state_view.add_module(&module.self_id(), module_bytes);
        stdlib_modules.push(module)
    }
    let data_cache = StateViewCache::new(&state_view).into_move_resolver();

    let move_vm = MoveVmExt::new(NativeGasParameters::zeros()).unwrap();
    let id1 = HashValue::zero();
    let mut session = move_vm.new_session(&data_cache, SessionId::genesis(id1));
    publish_stdlib(&mut session, stdlib_modules);
}
