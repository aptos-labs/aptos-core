// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, format_err, Result};
use aptos_api_types::AsConverter;
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    hash::HashValue,
    ValidCryptoMaterialStringExt,
};
use aptos_gas::{InitialGasSchedule, TransactionGasParameters};
use aptos_state_view::StateView;
use aptos_types::{
    access_path::AccessPath,
    account_config::{aptos_test_root_address, AccountResource, CoinStoreResource},
    block_metadata::BlockMetadata,
    chain_id::ChainId,
    contract_event::ContractEvent,
    state_store::{state_key::StateKey, table::TableHandle},
    transaction::{
        EntryFunction as TransactionEntryFunction, ExecutionStatus, Module as TransactionModule,
        RawTransaction, Script as TransactionScript, Transaction, TransactionOutput,
        TransactionStatus,
    },
};
use aptos_vm::{
    data_cache::{AsMoveResolver, IntoMoveResolver, StorageAdapterOwned},
    AptosVM,
};
use clap::StructOpt;
use language_e2e_tests::data_store::{FakeDataStore, GENESIS_CHANGE_SET_HEAD};
use move_deps::{
    move_binary_format::file_format::{CompiledModule, CompiledScript},
    move_command_line_common::{
        address::ParsedAddress, files::verify_and_create_named_address_mapping,
    },
    move_compiler::{self, shared::PackagePaths, FullyCompiledProgram},
    move_core_types::{
        account_address::AccountAddress,
        identifier::{IdentStr, Identifier},
        language_storage::{ModuleId, ResourceKey, TypeTag},
        move_resource::MoveStructType,
        parser::parse_type_tag,
        transaction_argument::{convert_txn_args, TransactionArgument},
        value::{MoveTypeLayout, MoveValue},
    },
    move_resource_viewer::{AnnotatedMoveValue, MoveValueAnnotator},
    move_transactional_test_runner::{
        framework::{run_test_impl, CompiledState, MoveTestAdapter},
        tasks::{InitCommand, SyntaxChoice, TaskInput},
        vm_test_harness::view_resource_in_move_storage,
    },
    move_vm_runtime::session::SerializedReturnValues,
};
use once_cell::sync::Lazy;
use std::{
    collections::{BTreeMap, HashMap},
    convert::TryFrom,
    fmt,
    path::Path,
    string::String,
    sync::Arc,
};
use vm_genesis::GENESIS_KEYPAIR;
/**
 * Definitions
 */

/// The Aptos transaction test adapter.
///
/// This differs from the SimpleVMTestAdapter in a few ways to ensure that our tests mimics
/// production settings:
///   - It uses a StateView as its storage backend
///   - It executes transactions through AptosVM, instead of MoveVM directly
struct AptosTestAdapter<'a> {
    compiled_state: CompiledState<'a>,
    storage: StorageAdapterOwned<FakeDataStore>,
    default_syntax: SyntaxChoice,
    private_key_mapping: BTreeMap<String, Ed25519PrivateKey>,
}

/// Parameters *required* to create a transaction.
struct TransactionParameters {
    pub sequence_number: u64,
    pub max_gas_amount: u64,
    pub gas_unit_price: u64,
    pub expiration_timestamp_secs: u64,
}

/// Aptos-specific arguments for the publish command.
#[derive(StructOpt, Debug)]
struct AptosPublishArgs {
    #[structopt(long = "private-key", parse(try_from_str = RawPrivateKey::parse))]
    private_key: Option<RawPrivateKey>,

    #[structopt(long = "expiration")]
    expiration_time: Option<u64>,

    #[structopt(long = "sequence-number")]
    sequence_number: Option<u64>,

    #[structopt(long = "gas-price")]
    gas_unit_price: Option<u64>,

    #[structopt(long = "override-signer", parse(try_from_str = ParsedAddress::parse))]
    override_signer: Option<ParsedAddress>,
}

#[derive(Debug)]
struct SignerAndKeyPair {
    address: ParsedAddress,
    private_key: Option<RawPrivateKey>,
}

/// Aptos-specifc arguments for the run command.
#[derive(StructOpt, Debug)]
struct AptosRunArgs {
    #[structopt(long = "private-key", parse(try_from_str = RawPrivateKey::parse))]
    private_key: Option<RawPrivateKey>,

    #[structopt(long = "script")]
    script: bool,

    #[structopt(long = "expiration")]
    expiration_time: Option<u64>,

    #[structopt(long = "sequence-number")]
    sequence_number: Option<u64>,

    #[structopt(long = "gas-price")]
    gas_unit_price: Option<u64>,

    #[structopt(long = "show-events")]
    show_events: bool,

    #[structopt(long = "secondary-signers", parse(try_from_str = SignerAndKeyPair::parse), multiple_values(true))]
    secondary_signers: Option<Vec<SignerAndKeyPair>>,
}

/// Aptos-specifc arguments for the init command.
#[derive(StructOpt, Debug)]
struct AptosInitArgs {
    #[structopt(long = "private-keys", parse(try_from_str = parse_named_private_key), multiple_values(true))]
    private_keys: Option<Vec<(Identifier, Ed25519PrivateKey)>>,
    #[structopt(long = "initial-coins")]
    initial_coins: Option<u64>,
}

/// A raw private key -- either a literal or an unresolved name.
#[derive(Debug)]
enum RawPrivateKey {
    Named(Identifier),
    Anonymous(Ed25519PrivateKey),
}

/// Command to initiate a block metadata transaction.
#[derive(StructOpt, Debug)]
struct BlockCommand {
    #[structopt(long = "proposer", parse(try_from_str = ParsedAddress::parse))]
    proposer: ParsedAddress,

    #[structopt(long = "time")]
    time: u64,
}

/// Command to view a table item.
#[derive(StructOpt, Debug)]
struct ViewTableCommand {
    #[structopt(long = "table_handle")]
    table_handle: AccountAddress,

    #[structopt(long = "key_type", parse(try_from_str = parse_type_tag))]
    key_type: TypeTag,

    #[structopt(long = "value_type", parse(try_from_str = parse_type_tag))]
    value_type: TypeTag,

    #[structopt(long = "key_value", parse(try_from_str = serde_json::from_str))]
    key_value: serde_json::Value,
}

/// Custom commands for the transactional test flow.
#[derive(StructOpt, Debug)]
enum AptosSubCommand {
    #[structopt(name = "block")]
    BlockCommand(BlockCommand),

    #[structopt(name = "view_table")]
    ViewTableCommand(ViewTableCommand),
}

/**
 * Parsing
 */

fn parse_ed25519_private_key(s: &str) -> Result<Ed25519PrivateKey> {
    Ok(Ed25519PrivateKey::from_encoded_string(s)?)
}

impl RawPrivateKey {
    fn parse(s: &str) -> Result<Self> {
        if let Ok(private_key) = parse_ed25519_private_key(s) {
            return Ok(Self::Anonymous(private_key));
        }
        let name = Identifier::new(s)
            .map_err(|_| format_err!("Failed to parse '{}' as private key.", s))?;
        Ok(Self::Named(name))
    }
}

fn parse_named_private_key(s: &str) -> Result<(Identifier, Ed25519PrivateKey)> {
    let before_after = s.split('=').collect::<Vec<_>>();

    if before_after.len() != 2 {
        bail!("Invalid named private key assignment. Must be of the form <private_key_name>=<private_key>, but found '{}'", s);
    }

    let name = Identifier::new(before_after[0])
        .map_err(|_| format_err!("Invalid private key name '{}'", s))?;
    let private_key = parse_ed25519_private_key(before_after[1])?;

    Ok((name, private_key))
}

impl SignerAndKeyPair {
    fn parse(s: &str) -> Result<Self> {
        if let Ok(address) = ParsedAddress::parse(s) {
            return Ok(Self {
                address,
                private_key: None,
            });
        };

        let before_after = s.split('=').collect::<Vec<_>>();

        if before_after.len() != 2 {
            bail!("Invalid signer and key pair. Must be of the form <signer addr>=<private_key> or <named signer addr>, but found '{}'", s);
        }

        let address = ParsedAddress::parse(before_after[0])?;
        let private_key = RawPrivateKey::parse(before_after[1])?;

        Ok(Self {
            address,
            private_key: Some(private_key),
        })
    }
}

pub struct FakeDbReader {}

impl storage_interface::DbReader for FakeDbReader {
    fn indexer_enabled(&self) -> bool {
        false
    }
}

/**
 * Helpers
 */

/// Default private key mappings for special Aptos accounts.
fn aptos_framework_private_key_mapping() -> Vec<(String, Ed25519PrivateKey)> {
    vec![("Root".to_string(), GENESIS_KEYPAIR.0.clone())]
}

fn panic_missing_private_key_named(cmd_name: &str, name: &str) -> ! {
    panic!(
        "Missing private key. Either add a `--private-key <priv_key>` argument \
            to the {} command, or associate an address to the \
            name '{}' in the init command.",
        cmd_name, name,
    )
}

fn panic_missing_private_key(cmd_name: &str) -> ! {
    panic!(
        "Missing private key. Try adding a `--private-key <priv_key>` \
            argument to the {} command.",
        cmd_name
    )
}

static PRECOMPILED_APTOS_FRAMEWORK: Lazy<FullyCompiledProgram> = Lazy::new(|| {
    let deps = vec![PackagePaths {
        name: None,
        paths: cached_packages::head_release_bundle().files().unwrap(),
        named_address_map: framework::named_addresses().clone(),
    }];
    let program_res = move_compiler::construct_pre_compiled_lib(
        deps,
        None,
        move_compiler::Flags::empty().set_sources_shadow_deps(false),
    )
    .unwrap();
    match program_res {
        Ok(af) => af,
        Err((files, errors)) => {
            eprintln!("!!!Aptos Framework failed to compile!!!");
            move_compiler::diagnostics::report_diagnostics(&files, errors)
        }
    }
});

/**
 * Test Adapter Implementation
 */

impl<'a> AptosTestAdapter<'a> {
    /// Look up the named private key in the mapping.
    fn resolve_named_private_key(&self, s: &IdentStr) -> Ed25519PrivateKey {
        if let Some(private_key) = self.private_key_mapping.get(s.as_str()) {
            return private_key.clone();
        }
        panic!("Failed to resolve private key '{}'", s)
    }

    /// Resolve a raw private key into a numeric one.
    fn resolve_private_key(&self, private_key: &RawPrivateKey) -> Ed25519PrivateKey {
        match private_key {
            RawPrivateKey::Anonymous(private_key) => private_key.clone(),
            RawPrivateKey::Named(name) => self.resolve_named_private_key(name),
        }
    }

    /// Resolve addresses and private keys for secondary signers.
    fn resolve_secondary_signers(
        &mut self,
        secondary_signers: &[SignerAndKeyPair],
    ) -> (Vec<AccountAddress>, Vec<Ed25519PrivateKey>) {
        let mut addresses = vec![];
        let mut private_keys = vec![];

        for SignerAndKeyPair {
            address,
            private_key,
        } in secondary_signers
        {
            addresses.push(self.compiled_state().resolve_address(address));

            let resolved_private_key = match (private_key, address) {
                (Some(private_key), _) => self.resolve_private_key(private_key),
                (None, ParsedAddress::Named(named_addr)) => {
                    match self.private_key_mapping.get(named_addr) {
                        Some(private_key) => private_key.clone(),
                        None => panic!(
                            "Failed to resolve private key for secondary signer {}.",
                            named_addr
                        ),
                    }
                }
                (None, ParsedAddress::Numerical(addr)) => {
                    panic!("No private key provided for secondary signer {}.", addr)
                }
            };

            private_keys.push(resolved_private_key);
        }

        (addresses, private_keys)
    }

    /// Obtain a Rust representation of the account resource from storage, which is used to derive
    /// a few default transaction parameters.
    fn fetch_account_resource(&self, signer_addr: &AccountAddress) -> Result<AccountResource> {
        let account_access_path = AccessPath::resource_access_path(ResourceKey::new(
            *signer_addr,
            AccountResource::struct_tag(),
        ));
        let account_blob = self
            .storage
            .get_state_value(&StateKey::AccessPath(account_access_path))
            .unwrap()
            .ok_or_else(|| {
                format_err!(
                "Failed to fetch account resource under address {}. Has the account been created?",
                signer_addr
            )
            })?;
        Ok(bcs::from_bytes(&account_blob).unwrap())
    }

    /// Obtain the AptosCoin amount under address `signer_addr`
    fn fetch_account_balance(&self, signer_addr: &AccountAddress) -> Result<u64> {
        let aptos_coin_tag = CoinStoreResource::struct_tag();

        let coin_access_path = AccessPath::resource_access_path(ResourceKey::new(
            *signer_addr,
            aptos_coin_tag.clone(),
        ));

        let balance_blob = self
            .storage
            .get_state_value(&StateKey::AccessPath(coin_access_path))
            .unwrap()
            .ok_or_else(|| {
                format_err!(
                    "Failed to fetch balance resource under address {}.",
                    signer_addr
                )
            })?;

        let annotated =
            MoveValueAnnotator::new(&self.storage).view_resource(&aptos_coin_tag, &balance_blob)?;

        // Filter the Coin resource and return the resouce value
        for (key, val) in annotated.value {
            if key != Identifier::new("coin").unwrap() {
                continue;
            }

            if let AnnotatedMoveValue::Struct(s) = val {
                for (key, val) in s.value {
                    if key != Identifier::new("value").unwrap() {
                        continue;
                    }

                    if let AnnotatedMoveValue::U64(v) = val {
                        return Ok(v);
                    }
                }
            }
        }

        bail!("Failed to fetch balance under address {}.", signer_addr)
    }

    /// Derive the default transaction parameters from the account and balance resources fetched
    /// from storage. In the future, we are planning to allow the user to override these using
    /// command arguments.
    fn fetch_transaction_parameters(
        &self,
        signer_addr: &AccountAddress,
        sequence_number: Option<u64>,
        expiration_time: Option<u64>,
        gas_unit_price: Option<u64>,
        max_gas_amount: Option<u64>,
    ) -> Result<TransactionParameters> {
        let account_resource = self.fetch_account_resource(signer_addr)?;

        let sequence_number = sequence_number.unwrap_or_else(|| account_resource.sequence_number());
        let max_number_of_gas_units =
            TransactionGasParameters::initial().maximum_number_of_gas_units;
        let gas_unit_price = gas_unit_price.unwrap_or(1000);
        let max_gas_amount = match max_gas_amount {
            Some(max_gas_amount) => max_gas_amount,
            None => {
                if gas_unit_price == 0 {
                    u64::from(max_number_of_gas_units)
                } else {
                    let account_balance = self.fetch_account_balance(signer_addr).unwrap();
                    std::cmp::min(
                        u64::from(max_number_of_gas_units),
                        account_balance / gas_unit_price,
                    )
                }
            }
        };
        let expiration_timestamp_secs = expiration_time.unwrap_or(40000);

        Ok(TransactionParameters {
            sequence_number,
            gas_unit_price,
            max_gas_amount,
            expiration_timestamp_secs,
        })
    }

    /// Perform a single transaction.
    ///
    /// Should error if the transaction ends up being discarded, or having a status other than
    /// EXECUTED.
    fn run_transaction(&mut self, txn: Transaction) -> Result<TransactionOutput> {
        let mut outputs = AptosVM::execute_block_and_keep_vm_status(vec![txn], &self.storage)?;

        assert_eq!(outputs.len(), 1);

        let (status, output) = outputs.pop().unwrap();
        match output.status() {
            TransactionStatus::Keep(kept_vm_status) => {
                self.storage.add_write_set(output.write_set());
                match kept_vm_status {
                    ExecutionStatus::Success => Ok(output),
                    _ => {
                        bail!("Failed to execute transaction. ExecutionStatus: {}", status)
                    }
                }
            }
            TransactionStatus::Discard(_) => {
                bail!("Transaction discarded. VMStatus: {}", status)
            }
            TransactionStatus::Retry => panic!(),
        }
    }

    fn create_and_fund_account(&mut self, account_addr: AccountAddress, amount: u64) {
        let parameters = self
            .fetch_transaction_parameters(&aptos_test_root_address(), None, None, None, None)
            .unwrap();

        let txn = RawTransaction::new(
            aptos_test_root_address(),
            parameters.sequence_number,
            cached_packages::aptos_stdlib::aptos_account_create_account(account_addr),
            parameters.max_gas_amount,
            parameters.gas_unit_price,
            parameters.expiration_timestamp_secs,
            ChainId::test(),
        )
        .sign(&GENESIS_KEYPAIR.0, GENESIS_KEYPAIR.1.clone())
        .unwrap()
        .into_inner();

        self.run_transaction(Transaction::UserTransaction(txn))
            .expect("Failed to create an account. This should not happen.");

        let txn = RawTransaction::new(
            aptos_test_root_address(),
            parameters.sequence_number + 1,
            cached_packages::aptos_stdlib::aptos_coin_mint(account_addr, amount),
            parameters.max_gas_amount,
            parameters.gas_unit_price,
            parameters.expiration_timestamp_secs,
            ChainId::test(),
        )
        .sign(&GENESIS_KEYPAIR.0, GENESIS_KEYPAIR.1.clone())
        .unwrap()
        .into_inner();

        self.run_transaction(Transaction::UserTransaction(txn))
            .expect("Failed to mint aptos coin. This should not happen.");
    }
}

impl<'a> MoveTestAdapter<'a> for AptosTestAdapter<'a> {
    type ExtraInitArgs = AptosInitArgs;
    type ExtraPublishArgs = AptosPublishArgs;
    type ExtraRunArgs = AptosRunArgs;
    type Subcommand = AptosSubCommand;
    type ExtraValueArgs = ();

    fn compiled_state(&mut self) -> &mut CompiledState<'a> {
        &mut self.compiled_state
    }

    fn default_syntax(&self) -> SyntaxChoice {
        self.default_syntax
    }

    fn init(
        default_syntax: SyntaxChoice,
        pre_compiled_deps: Option<&'a FullyCompiledProgram>,
        task_opt: Option<TaskInput<(InitCommand, Self::ExtraInitArgs)>>,
    ) -> (Self, Option<String>) {
        // Named address mapping
        let additional_named_address_mapping = match task_opt.as_ref().map(|t| &t.command) {
            Some((InitCommand { named_addresses }, _)) => {
                verify_and_create_named_address_mapping(named_addresses.clone()).unwrap()
            }
            None => BTreeMap::new(),
        };

        let mut named_address_mapping = framework::named_addresses().clone();

        for (name, addr) in additional_named_address_mapping.clone() {
            if named_address_mapping.contains_key(&name) {
                panic!("Invalid init. The named address '{}' already exists.", name)
            }
            named_address_mapping.insert(name, addr);
        }

        // Genesis modules
        let mut storage = FakeDataStore::new(HashMap::new()).into_move_resolver();
        storage.add_write_set(GENESIS_CHANGE_SET_HEAD.write_set());

        // Builtin private key mapping
        let mut private_key_mapping = BTreeMap::new();
        for (name, private_key) in aptos_framework_private_key_mapping() {
            private_key_mapping.insert(name, private_key);
        }

        // Initial coins to mint, defaults to 5000
        let mut coins_to_mint = 5000;

        if let Some(TaskInput {
            command: (_, init_args),
            ..
        }) = task_opt
        {
            // Private key mapping
            if let Some(additional_private_key_mapping) = init_args.private_keys {
                for (name, private_key) in additional_private_key_mapping {
                    if private_key_mapping.contains_key(name.as_str()) {
                        panic!(
                            "Invalid init. The named private key '{}' already exists.",
                            name
                        )
                    }
                    private_key_mapping.insert(name.as_str().to_string(), private_key);
                }
            }

            if let Some(initial_coins) = init_args.initial_coins {
                coins_to_mint = initial_coins;
            }
        }

        let mut adapter = Self {
            compiled_state: CompiledState::new(named_address_mapping, pre_compiled_deps, None),
            default_syntax,
            storage,
            private_key_mapping,
        };

        for (_, addr) in additional_named_address_mapping {
            adapter.create_and_fund_account(addr.into_inner(), coins_to_mint);
        }

        (adapter, None)
    }

    fn publish_module(
        &mut self,
        module: CompiledModule,
        mut named_addr_opt: Option<Identifier>,
        gas_budget: Option<u64>,
        extra_args: Self::ExtraPublishArgs,
    ) -> Result<(Option<String>, CompiledModule)> {
        let module_id = module.self_id();

        // TODO: hack to allow the signer to be overridden.
        // See if we can implement it in a cleaner way.
        let signer = match extra_args.override_signer {
            Some(addr) => {
                if let ParsedAddress::Named(named_addr) = &addr {
                    named_addr_opt = Some(Identifier::new(named_addr.clone()).unwrap())
                }
                self.compiled_state().resolve_address(&addr)
            }
            None => *module_id.address(),
        };

        let params = self.fetch_transaction_parameters(
            &signer,
            extra_args.sequence_number,
            extra_args.expiration_time,
            extra_args.gas_unit_price,
            gas_budget,
        )?;

        let mut module_blob = vec![];
        module.serialize(&mut module_blob).unwrap();

        let private_key = match (extra_args.private_key, named_addr_opt) {
            (Some(private_key), _) => self.resolve_private_key(&private_key),
            (None, Some(named_addr)) => match self
                .private_key_mapping
                .get(&named_addr.as_str().to_string())
            {
                Some(private_key) => private_key.clone(),
                None => panic_missing_private_key_named("publish", named_addr.as_str()),
            },
            (None, None) => panic_missing_private_key("publish"),
        };

        let txn = RawTransaction::new_module(
            signer,
            params.sequence_number,
            TransactionModule::new(module_blob),
            params.max_gas_amount,
            params.gas_unit_price,
            params.expiration_timestamp_secs,
            ChainId::test(),
        )
        .sign(&private_key, Ed25519PublicKey::from(&private_key))?
        .into_inner();

        self.run_transaction(Transaction::UserTransaction(txn))?;

        Ok((None, module))
    }

    fn execute_script(
        &mut self,
        script: CompiledScript,
        type_args: Vec<TypeTag>,
        signers: Vec<ParsedAddress>,
        txn_args: Vec<MoveValue>,
        gas_budget: Option<u64>,
        extra_args: Self::ExtraRunArgs,
    ) -> Result<(Option<String>, SerializedReturnValues)> {
        let signer0 = self.compiled_state().resolve_address(&signers[0]);

        if gas_budget.is_some() {
            panic!("Cannot set gas budget for script.")
        }
        if extra_args.gas_unit_price.is_some() {
            panic!("Cannot set gas price for script.")
        }
        if extra_args.expiration_time.is_some() {
            panic!("Cannot set expiration time for script.")
        }
        if extra_args.secondary_signers.is_some() {
            panic!("Cannot set secondary signers for script.")
        }

        let private_key = match (extra_args.private_key, &signers[0]) {
            (Some(private_key), _) => self.resolve_private_key(&private_key),
            (None, ParsedAddress::Named(named_addr)) => {
                match self.private_key_mapping.get(named_addr) {
                    Some(private_key) => private_key.clone(),
                    None => panic_missing_private_key_named("run", named_addr.as_str()),
                }
            }
            (None, ParsedAddress::Numerical(_)) => panic_missing_private_key("run"),
        };

        let mut script_blob = vec![];
        script.serialize(&mut script_blob)?;

        let params = self.fetch_transaction_parameters(
            &signer0,
            extra_args.sequence_number,
            None,
            None,
            None,
        )?;

        let txn = RawTransaction::new_script(
            signer0,
            params.sequence_number,
            TransactionScript::new(
                script_blob,
                type_args,
                txn_args
                    .into_iter()
                    .map(|arg| TransactionArgument::try_from(arg).unwrap())
                    .collect(),
            ),
            params.max_gas_amount,
            params.gas_unit_price,
            params.expiration_timestamp_secs,
            ChainId::test(),
        )
        .sign(&private_key, Ed25519PublicKey::from(&private_key))
        .unwrap()
        .into_inner();

        let output = self.run_transaction(Transaction::UserTransaction(txn))?;

        let output = if extra_args.show_events {
            render_events(output.events())
        } else {
            None
        };

        //TODO: replace this dummy value with actual txn return value
        let a = SerializedReturnValues {
            mutable_reference_outputs: vec![(0, vec![0], MoveTypeLayout::U8)],
            return_values: vec![(vec![0], MoveTypeLayout::U8)],
        };

        Ok((output, a))
    }

    fn call_function(
        &mut self,
        module: &ModuleId,
        function: &IdentStr,
        type_args: Vec<TypeTag>,
        signers: Vec<ParsedAddress>,
        txn_args: Vec<MoveValue>,
        gas_budget: Option<u64>,
        extra_args: Self::ExtraRunArgs,
    ) -> Result<(Option<String>, SerializedReturnValues)> {
        if extra_args.script {
            panic!("Entry functions are not supported.")
        }

        if signers.len() != 1 {
            panic!("Expected 1 signer, got {}.", signers.len());
        }
        let signer = self.compiled_state().resolve_address(&signers[0]);

        let private_key = match (extra_args.private_key, &signers[0]) {
            (Some(private_key), _) => self.resolve_private_key(&private_key),
            (None, ParsedAddress::Named(named_addr)) => {
                match self.private_key_mapping.get(named_addr) {
                    Some(private_key) => private_key.clone(),
                    None => panic_missing_private_key_named("run", named_addr.as_str()),
                }
            }
            (None, ParsedAddress::Numerical(_)) => panic_missing_private_key("run"),
        };

        let params = self.fetch_transaction_parameters(
            &signer,
            extra_args.sequence_number,
            extra_args.expiration_time,
            extra_args.gas_unit_price,
            gas_budget,
        )?;
        let txn = RawTransaction::new_entry_function(
            signer,
            params.sequence_number,
            TransactionEntryFunction::new(
                module.clone(),
                function.to_owned(),
                type_args,
                convert_txn_args(
                    &txn_args
                        .into_iter()
                        .map(|arg| TransactionArgument::try_from(arg).unwrap())
                        .collect::<Vec<_>>(),
                ),
            ),
            params.max_gas_amount,
            params.gas_unit_price,
            params.expiration_timestamp_secs,
            ChainId::test(),
        );

        let txn = match &extra_args.secondary_signers {
            Some(secondary_signers) => {
                let (secondary_signers, secondary_private_keys) =
                    self.resolve_secondary_signers(secondary_signers);

                txn.sign_multi_agent(
                    &private_key,
                    secondary_signers,
                    secondary_private_keys.iter().collect(),
                )?
                .into_inner()
            }
            None => txn
                .sign(&private_key, Ed25519PublicKey::from(&private_key))?
                .into_inner(),
        };

        let output = self.run_transaction(Transaction::UserTransaction(txn))?;

        let output = if extra_args.show_events {
            render_events(output.events())
        } else {
            None
        };

        //TODO: replace this dummy value with actual txn return value
        let a = SerializedReturnValues {
            mutable_reference_outputs: vec![(0, vec![0], MoveTypeLayout::U8)],
            return_values: vec![(vec![0], MoveTypeLayout::U8)],
        };
        Ok((output, a))
    }

    fn view_data(
        &mut self,
        address: AccountAddress,
        module: &ModuleId,
        resource: &IdentStr,
        type_args: Vec<TypeTag>,
    ) -> Result<String> {
        view_resource_in_move_storage(&self.storage, address, module, resource, type_args)
    }

    fn handle_subcommand(&mut self, input: TaskInput<Self::Subcommand>) -> Result<Option<String>> {
        match input.command {
            AptosSubCommand::BlockCommand(block_cmd) => {
                let proposer = self.compiled_state().resolve_address(&block_cmd.proposer);
                let metadata = BlockMetadata::new(
                    HashValue::zero(),
                    0,
                    block_cmd.time,
                    proposer,
                    vec![],
                    vec![],
                    block_cmd.time,
                );

                let output = self.run_transaction(Transaction::BlockMetadata(metadata))?;

                Ok(render_events(output.events()))
            }
            AptosSubCommand::ViewTableCommand(view_table_cmd) => {
                let resolver = self.storage.as_move_resolver();
                let converter = resolver.as_converter(Arc::new(FakeDbReader {}));

                let vm_key = converter
                    .try_into_vm_value(&view_table_cmd.key_type, view_table_cmd.key_value)
                    .unwrap();
                let raw_key = vm_key.undecorate().simple_serialize().unwrap();

                let state_key =
                    StateKey::table_item(TableHandle(view_table_cmd.table_handle), raw_key);

                let bytes = self
                    .storage
                    .get_state_value(&state_key)
                    .unwrap()
                    .ok_or_else(|| format_err!("Failed to fetch table item.",))?;

                let move_value =
                    converter.try_into_move_value(&view_table_cmd.value_type, &bytes)?;

                Ok(Some(serde_json::to_string(&move_value).unwrap()))
            }
        }
    }
}

/**
 * Misc
 */

struct PrettyEvent<'a>(&'a ContractEvent);

impl<'a> fmt::Display for PrettyEvent<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{{")?;
        writeln!(f, "    key:     {}", self.0.key())?;
        writeln!(f, "    seq_num: {}", self.0.sequence_number())?;
        writeln!(f, "    type:    {}", self.0.type_tag())?;
        writeln!(f, "    data:    {:?}", hex::encode(self.0.event_data()))?;
        write!(f, "}}")
    }
}

struct PrettyEvents<'a>(&'a [ContractEvent]);

impl<'a> fmt::Display for PrettyEvents<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Events:")?;
        for event in self.0.iter() {
            writeln!(f)?;
            write!(f, "{}", PrettyEvent(event))?;
        }
        Ok(())
    }
}

fn render_events(events: &[ContractEvent]) -> Option<String> {
    if events.is_empty() {
        None
    } else {
        Some(format!("{}", PrettyEvents(events)))
    }
}

pub fn run_aptos_test(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: remove once bundles removed
    aptos_vm::aptos_vm::allow_module_bundle_for_test();
    run_test_impl::<AptosTestAdapter>(path, Some(&*PRECOMPILED_APTOS_FRAMEWORK))
}
