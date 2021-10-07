// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, format_err, Result};
use diem_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    ValidCryptoMaterialStringExt,
};
use diem_state_view::StateView;
use diem_types::{
    access_path::AccessPath,
    account_config::{
        self, diem_root_address, type_tag_for_currency_code, AccountResource, BalanceResource,
        XUS_NAME,
    },
    chain_id::ChainId,
    transaction::{
        Module as TransactionModule, RawTransaction, Script as TransactionScript,
        ScriptFunction as TransactionScriptFunction, SignedTransaction, Transaction,
        TransactionStatus,
    },
    vm_status::KeptVMStatus,
};
use diem_vm::DiemVM;
use language_e2e_tests::data_store::{FakeDataStore, GENESIS_CHANGE_SET_FRESH};
use move_binary_format::file_format::{CompiledModule, CompiledScript};
use move_core_types::{
    account_address::AccountAddress,
    gas_schedule::{GasAlgebra, GasConstants},
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, ResourceKey, TypeTag},
    move_resource::MoveStructType,
    transaction_argument::{convert_txn_args, TransactionArgument},
};
use move_lang::{shared::verify_and_create_named_address_mapping, FullyCompiledProgram};
use move_transactional_test_runner::{
    framework::{run_test_impl, CompiledState, MoveTestAdapter},
    tasks::{EmptyCommand, InitCommand, RawAddress, SyntaxChoice, TaskInput},
    vm_test_harness::view_resource_in_move_storage,
};
use once_cell::sync::Lazy;
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};
use structopt::StructOpt;
use vm_genesis::GENESIS_KEYPAIR;

/// The Diem transaction test adapter.
///
/// This differs from the SimpleVMTestAdapter in a few ways to ensure that our tests mimics
/// production settings:
///   - It uses a Diem StateView as its storage backend
///   - It executes transactions through DiemVM, instead of MoveVM directly
struct DiemTestAdapter<'a> {
    compiled_state: CompiledState<'a>,
    storage: FakeDataStore,
    default_syntax: SyntaxChoice,
    private_key_mapping: BTreeMap<Identifier, Ed25519PrivateKey>,
}

/// Parameters *required* to create a Diem transaction.
struct TransactionParameters {
    pub sequence_number: u64,
    pub max_gas_amount: u64,
    pub gas_unit_price: u64,
    pub gas_currency_code: String,
    pub expiration_timestamp_secs: u64,
}

/// Diem-specific arguments for the publish command.
#[derive(StructOpt, Debug)]
struct DiemPublishArgs {
    #[structopt(short = "k", long = "private-key", parse(try_from_str = RawPrivateKey::parse))]
    private_key: Option<RawPrivateKey>,
}

/// Diem-specifc arguments for the run command.
#[derive(StructOpt, Debug)]
struct DiemRunArgs {
    #[structopt(short = "k", long = "private-key", parse(try_from_str = RawPrivateKey::parse))]
    private_key: Option<RawPrivateKey>,

    #[structopt(long = "--admin-script")]
    admin_script: bool,
}

/// Diem-specifc arguments for the init command.
#[derive(StructOpt, Debug)]
struct DiemInitArgs {
    #[structopt(long = "private-keys", parse(try_from_str = parse_named_private_key))]
    private_keys: Option<Vec<(Identifier, Ed25519PrivateKey)>>,
}

/// A raw private key -- either a literal or an unresolved name.
#[derive(Debug)]
enum RawPrivateKey {
    Named(Identifier),
    Literal(Ed25519PrivateKey),
}

fn parse_ed25519_private_key(s: &str) -> Result<Ed25519PrivateKey> {
    Ok(Ed25519PrivateKey::from_encoded_string(s)?)
}

impl RawPrivateKey {
    fn parse(s: &str) -> Result<Self> {
        if let Ok(private_key) = parse_ed25519_private_key(s) {
            return Ok(Self::Literal(private_key));
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

/// Default private key mappings for special Diem accounts.
fn diem_framework_private_key_mapping() -> Vec<(String, Ed25519PrivateKey)> {
    vec![
        ("DiemRoot".to_string(), GENESIS_KEYPAIR.0.clone()),
        ("TreasuryCompliance".to_string(), GENESIS_KEYPAIR.0.clone()),
        ("DesignatedDealer".to_string(), GENESIS_KEYPAIR.0.clone()),
    ]
}

impl<'a> DiemTestAdapter<'a> {
    /// Look up the named private key in the mapping.
    fn resolve_named_private_key(&self, s: &IdentStr) -> Ed25519PrivateKey {
        if let Some(private_key) = self.private_key_mapping.get(s) {
            return private_key.clone();
        }
        panic!("Failed to resolve private key '{}'", s)
    }

    /// Resolve a raw private key into a numeric one.
    fn resolve_private_key(&self, private_key: &RawPrivateKey) -> Ed25519PrivateKey {
        match private_key {
            RawPrivateKey::Literal(private_key) => private_key.clone(),
            RawPrivateKey::Named(name) => self.resolve_named_private_key(name),
        }
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
            .get(&account_access_path)
            .unwrap()
            .ok_or_else(|| {
                format_err!(
                "Failed to fetch account resource under address {}. Has the account been created?",
                signer_addr
            )
            })?;
        Ok(bcs::from_bytes(&account_blob).unwrap())
    }

    /// Obtain a Rust representation of the balance resource from storage, which is used to derive
    /// a few default transaction parameters.
    fn fetch_balance_resource(
        &self,
        signer_addr: &AccountAddress,
        balance_currency_code: Identifier,
    ) -> Result<BalanceResource> {
        let currency_code_tag = type_tag_for_currency_code(balance_currency_code);
        let balance_resource_tag = BalanceResource::struct_tag_for_currency(currency_code_tag);
        let balance_access_path =
            AccessPath::resource_access_path(ResourceKey::new(*signer_addr, balance_resource_tag));

        let balance_blob = self
            .storage
            .get(&balance_access_path)
            .unwrap()
            .ok_or_else(|| {
                format_err!(
                    "Failed to fetch balance resource under address {}.",
                    signer_addr
                )
            })?;

        Ok(bcs::from_bytes(&balance_blob).unwrap())
    }

    /// Derive the default transaction parameters from the account and balance resources fetched
    /// from storage. In the future, we are planning to allow the user to override these using
    /// command arguments.
    fn fetch_default_transaction_parameters(
        &self,
        signer_addr: &AccountAddress,
    ) -> Result<TransactionParameters> {
        let account_resource = self.fetch_account_resource(signer_addr)?;

        let sequence_number = account_resource.sequence_number();
        let gas_currency_code = XUS_NAME.to_string();
        let max_number_of_gas_units = GasConstants::default().maximum_number_of_gas_units;
        let gas_unit_price = 0;
        let max_gas_amount = if gas_unit_price == 0 {
            max_number_of_gas_units.get()
        } else {
            let account_balance = self.fetch_balance_resource(
                signer_addr,
                account_config::from_currency_code_string(&gas_currency_code).unwrap(),
            )?;
            std::cmp::min(
                max_number_of_gas_units.get(),
                account_balance.coin() / gas_unit_price,
            )
        };

        Ok(TransactionParameters {
            sequence_number,
            gas_currency_code,
            gas_unit_price,
            max_gas_amount,
            expiration_timestamp_secs: 40000,
        })
    }

    /// Perform a single Diem transaction.
    ///
    /// Should error if the transaction ends up being discarded, or having a status other than
    /// EXECUTED.
    fn run_transaction(&mut self, txn: SignedTransaction) -> Result<()> {
        let mut outputs = DiemVM::execute_block_and_keep_vm_status(
            vec![Transaction::UserTransaction(txn)],
            &self.storage,
        )
        .unwrap();

        assert_eq!(outputs.len(), 1);

        let (status, output) = outputs.pop().unwrap();
        match output.status() {
            TransactionStatus::Keep(kept_vm_status) => match kept_vm_status {
                KeptVMStatus::Executed => {
                    self.storage.add_write_set(output.write_set());
                }
                _ => {
                    bail!("Failed to execute transaction. VMStatus: {}", status)
                }
            },
            TransactionStatus::Discard(_) => {
                bail!("Transaction discarded. VMStatus: {}", status)
            }
            TransactionStatus::Retry => panic!(),
        }

        Ok(())
    }
}

fn panic_missing_private_key_named(cmd_name: &str, name: &IdentStr) -> ! {
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

impl<'a> MoveTestAdapter<'a> for DiemTestAdapter<'a> {
    type ExtraInitArgs = DiemInitArgs;
    type ExtraPublishArgs = DiemPublishArgs;
    type ExtraRunArgs = DiemRunArgs;
    type Subcommand = EmptyCommand;

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
    ) -> Self {
        let additional_named_address_mapping = match task_opt.as_ref().map(|t| &t.command) {
            Some((InitCommand { named_addresses }, _)) => {
                verify_and_create_named_address_mapping(named_addresses.clone()).unwrap()
            }
            None => BTreeMap::new(),
        };

        let mut named_address_mapping = diem_framework::diem_framework_named_addresses();
        for (name, addr) in additional_named_address_mapping {
            if named_address_mapping.contains_key(&name) {
                panic!("Invalid init. The named address '{}' already exists.", name)
            }
            named_address_mapping.insert(name, addr);
        }

        // TODO: rework vm-genesis and try not to compile the genesis modules twice.
        let mut storage = FakeDataStore::new(HashMap::new());
        storage.add_write_set(GENESIS_CHANGE_SET_FRESH.write_set());

        let mut private_key_mapping = BTreeMap::new();
        for (name, private_key) in diem_framework_private_key_mapping() {
            private_key_mapping.insert(Identifier::new(name).unwrap(), private_key);
        }
        if let Some(TaskInput {
            command:
                (
                    _,
                    DiemInitArgs {
                        private_keys: Some(additional_private_key_mapping),
                        ..
                    },
                ),
            ..
        }) = task_opt
        {
            for (name, private_key) in additional_private_key_mapping {
                if private_key_mapping.contains_key(&name) {
                    panic!(
                        "Invalid init. The named private key '{}' already exists",
                        name
                    )
                }
                private_key_mapping.insert(name, private_key);
            }
        }

        Self {
            compiled_state: CompiledState::new(named_address_mapping, pre_compiled_deps),
            default_syntax,
            storage,
            private_key_mapping,
        }
    }

    fn publish_module(
        &mut self,
        module: CompiledModule,
        named_addr_opt: Option<Identifier>,
        gas_budget: Option<u64>,
        extra_args: Self::ExtraPublishArgs,
    ) -> Result<()> {
        let module_id = module.self_id();
        let signer = module_id.address();
        let params = self.fetch_default_transaction_parameters(signer)?;

        let mut module_blob = vec![];
        module.serialize(&mut module_blob).unwrap();

        let private_key = match (extra_args.private_key, named_addr_opt) {
            (Some(private_key), _) => self.resolve_private_key(&private_key),
            (None, Some(named_addr)) => match self.private_key_mapping.get(&named_addr) {
                Some(private_key) => private_key.clone(),
                None => panic_missing_private_key_named("publish", &named_addr),
            },
            (None, None) => panic_missing_private_key("publish"),
        };

        let txn = RawTransaction::new_module(
            *signer,
            params.sequence_number,
            TransactionModule::new(module_blob),
            gas_budget.unwrap_or(params.max_gas_amount),
            params.gas_unit_price,
            params.gas_currency_code,
            params.expiration_timestamp_secs,
            ChainId::test(),
        )
        .sign(&private_key, Ed25519PublicKey::from(&private_key))?
        .into_inner();

        self.run_transaction(txn)?;

        Ok(())
    }

    fn execute_script(
        &mut self,
        script: CompiledScript,
        type_args: Vec<TypeTag>,
        signers: Vec<RawAddress>,
        txn_args: Vec<TransactionArgument>,
        gas_budget: Option<u64>,
        extra_args: Self::ExtraRunArgs,
    ) -> Result<()> {
        assert!(!signers.is_empty());

        if !extra_args.admin_script {
            panic!(
                "Transactions scripts are not currently allowed on Diem. \
                If you intend to run an admin script, add the `--admin-script` option to the run command."
            )
        }

        if signers.len() != 1 {
            panic!("Expected 1 signer, got {}.", signers.len());
        }
        let signer = self.compiled_state().resolve_address(&signers[0]);

        if gas_budget.is_some() {
            panic!("Cannot set gas budget for admin script.")
        }

        let mut script_blob = vec![];
        script.serialize(&mut script_blob)?;

        let diem_root = diem_root_address();

        let params = self.fetch_default_transaction_parameters(&diem_root)?;

        let private_key = match &extra_args.private_key {
            Some(private_key) => self.resolve_private_key(private_key),
            None => GENESIS_KEYPAIR.0.clone(),
        };

        let txn = RawTransaction::new_writeset_script(
            diem_root,
            params.sequence_number,
            TransactionScript::new(script_blob, type_args, txn_args),
            signer,
            ChainId::test(),
        )
        .sign(&private_key, Ed25519PublicKey::from(&private_key))
        .unwrap()
        .into_inner();

        self.run_transaction(txn)?;

        Ok(())
    }

    fn call_function(
        &mut self,
        module: &ModuleId,
        function: &IdentStr,
        type_args: Vec<TypeTag>,
        signers: Vec<RawAddress>,
        txn_args: Vec<TransactionArgument>,
        gas_budget: Option<u64>,
        extra_args: Self::ExtraRunArgs,
    ) -> Result<()> {
        assert!(!signers.is_empty());

        if extra_args.admin_script {
            panic!("Admin script functions are not supported.")
        }

        if signers.len() != 1 {
            panic!("Expected 1 signer, got {}.", signers.len());
        }
        let signer = self.compiled_state().resolve_address(&signers[0]);

        let private_key = match (extra_args.private_key, &signers[0]) {
            (Some(private_key), _) => self.resolve_private_key(&private_key),
            (None, RawAddress::Named(named_addr)) => match self.private_key_mapping.get(named_addr)
            {
                Some(private_key) => private_key.clone(),
                None => panic_missing_private_key_named("run", named_addr),
            },
            (None, RawAddress::Literal(_)) => panic_missing_private_key("run"),
        };

        let params = self.fetch_default_transaction_parameters(&signer)?;
        let txn = RawTransaction::new_script_function(
            signer,
            params.sequence_number,
            TransactionScriptFunction::new(
                module.clone(),
                function.to_owned(),
                type_args,
                convert_txn_args(&txn_args),
            ),
            gas_budget.unwrap_or(params.max_gas_amount),
            params.gas_unit_price,
            params.gas_currency_code,
            params.expiration_timestamp_secs,
            ChainId::test(),
        )
        .sign(&private_key, Ed25519PublicKey::from(&private_key))?
        .into_inner();

        self.run_transaction(txn)?;

        Ok(())
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

    fn handle_subcommand(
        &mut self,
        _subcommand: TaskInput<Self::Subcommand>,
    ) -> Result<Option<String>> {
        unreachable!()
    }
}

static PRECOMPILED_DIEM_FRAMEWORK: Lazy<FullyCompiledProgram> = Lazy::new(|| {
    let program_res = move_lang::construct_pre_compiled_lib(
        &diem_framework::diem_stdlib_files(),
        None,
        move_lang::Flags::empty().set_sources_shadow_deps(false),
        diem_framework::diem_framework_named_addresses(),
    )
    .unwrap();
    match program_res {
        Ok(df) => df,
        Err((files, errors)) => {
            eprintln!("!!!Diem Framework failed to compile!!!");
            move_lang::diagnostics::report_diagnostics(&files, errors)
        }
    }
});

/// Run the Diem transactional test flow, using the given file as input.
pub fn run_test(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    run_test_impl::<DiemTestAdapter>(path, Some(&*PRECOMPILED_DIEM_FRAMEWORK))
}
