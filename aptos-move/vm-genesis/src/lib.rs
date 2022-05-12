// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

mod genesis_context;

use crate::genesis_context::GenesisStateView;
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    HashValue, PrivateKey, Uniform,
};
use aptos_types::{
    account_config::{self, events::NewEpochEvent},
    chain_id::ChainId,
    contract_event::ContractEvent,
    on_chain_config::{
        ConsensusConfigV1, ConsensusConfigV2, OnChainConsensusConfig, VMPublishingOption,
        APTOS_MAX_KNOWN_VERSION,
    },
    transaction::{authenticator::AuthenticationKey, ChangeSet, Transaction, WriteSetPayload},
};
use aptos_vm::{
    data_cache::{IntoMoveResolver, StateViewCache},
    move_vm_ext::{MoveVmExt, SessionExt, SessionId},
};
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
    move_vm_types::gas_schedule::{GasStatus, INITIAL_COST_SCHEDULE},
};
use once_cell::sync::Lazy;
use rand::prelude::*;

// The seed is arbitrarily picked to produce a consistent key. XXX make this more formal?
const GENESIS_SEED: [u8; 32] = [42; 32];

const GENESIS_MODULE_NAME: &str = "Genesis";

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
    min_price_per_gas_unit: u64,
) -> Transaction {
    let consensus_config = OnChainConsensusConfig::V2(ConsensusConfigV2 {
        two_chain: true,
        decoupled_execution: true,
        back_pressure_limit: 10,
        exclude_round: 20,
    });

    Transaction::GenesisTransaction(WriteSetPayload::Direct(encode_genesis_change_set(
        &aptos_root_key,
        validators,
        stdlib_module_bytes,
        VMPublishingOption::open(),
        consensus_config,
        chain_id,
        min_price_per_gas_unit,
    )))
}

pub fn encode_genesis_change_set(
    aptos_root_key: &Ed25519PublicKey,
    validators: &[Validator],
    stdlib_module_bytes: &[Vec<u8>],
    vm_publishing_option: VMPublishingOption,
    consensus_config: OnChainConsensusConfig,
    chain_id: ChainId,
    min_price_per_gas_unit: u64,
) -> ChangeSet {
    let mut stdlib_modules = Vec::new();
    // create a data view for move_vm
    let mut state_view = GenesisStateView::new();
    for module_bytes in stdlib_module_bytes {
        let module = CompiledModule::deserialize(module_bytes).unwrap();
        state_view.add_module(&module.self_id(), module_bytes);
        stdlib_modules.push(module)
    }
    let data_cache = StateViewCache::new(&state_view).into_move_resolver();

    let move_vm = MoveVmExt::new().unwrap();
    let id1 = HashValue::zero();
    let mut session = move_vm.new_session(&data_cache, SessionId::genesis(id1));

    create_and_initialize_main_accounts(
        &mut session,
        aptos_root_key,
        vm_publishing_option,
        consensus_config,
        chain_id,
        min_price_per_gas_unit,
    );
    // generate the genesis WriteSet
    create_and_initialize_validators(&mut session, validators);
    reconfigure(&mut session);

    let mut session1_out = session.finish().unwrap();

    let state_view = GenesisStateView::new();
    let data_cache = StateViewCache::new(&state_view).into_move_resolver();

    // use a different session id, in case both scripts creates tables
    let mut id2_arr = [0u8; 32];
    id2_arr[31] = 1;
    let id2 = HashValue::new(id2_arr);
    let mut session = move_vm.new_session(&data_cache, SessionId::genesis(id2));

    publish_stdlib(&mut session, Modules::new(stdlib_modules.iter()));
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
            &mut GasStatus::new_unmetered(),
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
    publishing_option: VMPublishingOption,
    consensus_config: OnChainConsensusConfig,
    chain_id: ChainId,
    min_price_per_gas_unit: u64,
) {
    let aptos_root_auth_key = AuthenticationKey::ed25519(aptos_root_key);

    let root_aptos_root_address = account_config::aptos_root_address();

    let initial_allow_list = MoveValue::Vector(
        publishing_option
            .script_allow_list
            .into_iter()
            .map(|hash| MoveValue::vector_u8(hash.to_vec().into_iter().collect()))
            .collect(),
    );

    let genesis_gas_schedule = &INITIAL_COST_SCHEDULE;
    let instr_gas_costs = bcs::to_bytes(&genesis_gas_schedule.instruction_table)
        .expect("Failure serializing genesis instr gas costs");
    let native_gas_costs = bcs::to_bytes(&genesis_gas_schedule.native_table)
        .expect("Failure serializing genesis native gas costs");

    let consensus_config_bytes =
        bcs::to_bytes(&consensus_config).expect("Failure serializing genesis consensus config");

    // TODO: make these configurable
    let epoch_interval = 86400 * 1000000;
    let minimum_stake = 0;
    let maximum_stake = 1000000;

    exec_function(
        session,
        GENESIS_MODULE_NAME,
        "initialize",
        vec![],
        serialize_values(&vec![
            MoveValue::Signer(root_aptos_root_address),
            MoveValue::vector_u8(aptos_root_auth_key.to_vec()),
            initial_allow_list,
            MoveValue::Bool(publishing_option.is_open_module),
            MoveValue::vector_u8(instr_gas_costs),
            MoveValue::vector_u8(native_gas_costs),
            MoveValue::U8(chain_id.id()),
            MoveValue::U64(APTOS_MAX_KNOWN_VERSION.major),
            MoveValue::vector_u8(consensus_config_bytes),
            MoveValue::U64(min_price_per_gas_unit),
            MoveValue::U64(epoch_interval),
            MoveValue::U64(minimum_stake),
            MoveValue::U64(maximum_stake),
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
    let aptos_root_address = account_config::aptos_root_address();
    let mut owners = vec![];
    let mut owner_auth_keys = vec![];
    let mut consensus_pubkeys = vec![];
    let mut validator_network_addresses = vec![];
    let mut full_node_network_addresses = vec![];
    let mut staking_distribution = vec![];

    for v in validators {
        owners.push(MoveValue::Address(v.address));
        owner_auth_keys.push(MoveValue::vector_u8(v.auth_key.to_vec()));
        consensus_pubkeys.push(MoveValue::vector_u8(v.consensus_pubkey.clone()));
        validator_network_addresses.push(MoveValue::vector_u8(v.network_address.clone()));
        full_node_network_addresses.push(MoveValue::vector_u8(v.full_node_network_address.clone()));
        staking_distribution.push(MoveValue::U64(v.stake_amount));
    }
    exec_function(
        session,
        GENESIS_MODULE_NAME,
        "create_initialize_validators",
        vec![],
        serialize_values(&vec![
            MoveValue::Signer(aptos_root_address),
            MoveValue::Vector(owners),
            MoveValue::Vector(owner_auth_keys),
            MoveValue::Vector(consensus_pubkeys),
            MoveValue::Vector(validator_network_addresses),
            MoveValue::Vector(full_node_network_addresses),
            MoveValue::Vector(staking_distribution),
        ]),
    );
}

/// Publish the standard library.
fn publish_stdlib(session: &mut SessionExt<impl MoveResolver>, stdlib: Modules) {
    let dep_graph = stdlib.compute_dependency_graph();
    let mut addr_opt: Option<AccountAddress> = None;
    let modules = dep_graph
        .compute_topological_order()
        .unwrap()
        .map(|m| {
            let addr = *m.self_id().address();
            if let Some(a) = addr_opt {
              assert!(
                  a == addr,
                  "All genesis modules must be published under the same address, but found modules under both {} and {}",
                  a.short_str_lossless(),
                  addr.short_str_lossless()
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
        .publish_module_bundle(modules, addr_opt.unwrap(), &mut GasStatus::new_unmetered())
        .unwrap_or_else(|e| panic!("Failure publishing modules {:?}", e));
}

/// Trigger a reconfiguration. This emits an event that will be passed along to the storage layer.
fn reconfigure(session: &mut SessionExt<impl MoveResolver>) {
    exec_function(
        session,
        "Reconfiguration",
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
    assert!(
        new_epoch_events.len() == 1,
        "There should only be one NewEpochEvent"
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

    generate_test_genesis(&modules, VMPublishingOption::open(), None).0
}

pub fn test_genesis_transaction() -> Transaction {
    let changeset = test_genesis_change_set_and_validators(None).0;
    Transaction::GenesisTransaction(WriteSetPayload::Direct(changeset))
}

pub fn test_genesis_change_set_and_validators(
    count: Option<usize>,
) -> (ChangeSet, Vec<TestValidator>) {
    generate_test_genesis(
        cached_framework_packages::module_blobs(),
        VMPublishingOption::open(),
        count,
    )
}

#[derive(Debug, Clone)]
pub struct Validator {
    /// The Aptos account address of the validator
    pub address: AccountAddress,
    /// Authentication key for the validator
    pub auth_key: AuthenticationKey,
    /// Ed25519 public key used to sign consensus messages
    pub consensus_pubkey: Vec<u8>,
    /// The Aptos account address of the validator's operator (same as `address` if the validator is
    /// its own operator)
    pub operator_address: AccountAddress,
    /// Authentication key for the operator
    pub operator_auth_key: AuthenticationKey,
    /// `NetworkAddress` for the validator
    pub network_address: Vec<u8>,
    /// `NetworkAddress` for the validator's full node
    pub full_node_network_address: Vec<u8>,
    /// Amount to stake for consensus
    pub stake_amount: u64,
}

pub struct TestValidator {
    pub key: Ed25519PrivateKey,
    pub data: Validator,
}

impl TestValidator {
    pub fn new_test_set(count: Option<usize>) -> Vec<TestValidator> {
        let mut rng: rand::rngs::StdRng = rand::SeedableRng::from_seed([1u8; 32]);
        (0..count.unwrap_or(10))
            .map(|idx| TestValidator::gen(idx, &mut rng))
            .collect()
    }

    fn gen(index: usize, rng: &mut rand::rngs::StdRng) -> TestValidator {
        let name = index.to_string().as_bytes().to_vec();
        let address = aptos_config::utils::validator_owner_account_from_name(&name);
        let key = Ed25519PrivateKey::generate(rng);
        let auth_key = AuthenticationKey::ed25519(&key.public_key());
        let consensus_pubkey = key.public_key().to_bytes().to_vec();
        let operator_auth_key = auth_key;
        let operator_address = operator_auth_key.derived_address();
        let network_address = [0u8; 0].to_vec();
        let full_node_network_address = [0u8; 0].to_vec();

        let data = Validator {
            address,
            auth_key,
            consensus_pubkey,
            operator_address,
            operator_auth_key,
            network_address,
            full_node_network_address,
            stake_amount: 1,
        };
        Self { key, data }
    }
}

pub fn generate_test_genesis(
    stdlib_modules: &[Vec<u8>],
    vm_publishing_option: VMPublishingOption,
    count: Option<usize>,
) -> (ChangeSet, Vec<TestValidator>) {
    let test_validators = TestValidator::new_test_set(count);
    let validators_: Vec<Validator> = test_validators.iter().map(|t| t.data.clone()).collect();
    let validators = &validators_;

    let genesis = encode_genesis_change_set(
        &GENESIS_KEYPAIR.1,
        validators,
        stdlib_modules,
        vm_publishing_option,
        OnChainConsensusConfig::V1(ConsensusConfigV1 { two_chain: true }),
        ChainId::test(),
        0,
    );
    (genesis, test_validators)
}
