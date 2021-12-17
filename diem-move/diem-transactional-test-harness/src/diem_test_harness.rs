// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, format_err, Result};
use diem_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    hash::HashValue,
    PrivateKey, ValidCryptoMaterialStringExt,
};
use diem_keygen::KeyGen;
use diem_state_view::StateView;
use diem_types::{
    access_path::AccessPath,
    account_config::{
        self, diem_root_address, testnet_dd_account_address, treasury_compliance_account_address,
        type_tag_for_currency_code, AccountResource, BalanceResource, XUS_IDENTIFIER, XUS_NAME,
    },
    block_metadata::BlockMetadata,
    chain_id::ChainId,
    contract_event::ContractEvent,
    transaction::{
        Module as TransactionModule, RawTransaction, Script as TransactionScript,
        ScriptFunction as TransactionScriptFunction, Transaction, TransactionOutput,
        TransactionStatus,
    },
    vm_status::KeptVMStatus,
};
use diem_vm::DiemVM;
use language_e2e_tests::data_store::{FakeDataStore, GENESIS_CHANGE_SET_FRESH};
use move_binary_format::file_format::{CompiledModule, CompiledScript};
use move_compiler::{
    shared::{verify_and_create_named_address_mapping, NumberFormat, NumericalAddress},
    FullyCompiledProgram,
};
use move_core_types::{
    account_address::AccountAddress,
    gas_schedule::{GasAlgebra, GasConstants},
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, ResourceKey, StructTag, TypeTag},
    move_resource::MoveStructType,
    transaction_argument::{convert_txn_args, TransactionArgument},
};
use move_transactional_test_runner::{
    framework::{run_test_impl, CompiledState, MoveTestAdapter},
    tasks::{InitCommand, RawAddress, SyntaxChoice, TaskInput},
    vm_test_harness::view_resource_in_move_storage,
};
use once_cell::sync::Lazy;
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    path::Path,
};
use structopt::StructOpt;
use vm_genesis::GENESIS_KEYPAIR;

/*************************************************************************************************
 *
 * Definitions
 *
 *
 ************************************************************************************************/

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
    #[structopt(long = "private-key", parse(try_from_str = RawPrivateKey::parse))]
    private_key: Option<RawPrivateKey>,

    #[structopt(long = "expiration")]
    expiration_time: Option<u64>,

    #[structopt(long = "sequence-number")]
    sequence_number: Option<u64>,

    #[structopt(long = "gas-price")]
    gas_unit_price: Option<u64>,

    #[structopt(long = "gas-currency")]
    gas_currency_code: Option<String>,
}

#[derive(Debug)]
struct SignerAndKeyPair {
    address: RawAddress,
    private_key: Option<RawPrivateKey>,
}

/// Diem-specifc arguments for the run command.
#[derive(StructOpt, Debug)]
struct DiemRunArgs {
    #[structopt(long = "private-key", parse(try_from_str = RawPrivateKey::parse))]
    private_key: Option<RawPrivateKey>,

    #[structopt(long = "admin-script")]
    admin_script: bool,

    #[structopt(long = "expiration")]
    expiration_time: Option<u64>,

    #[structopt(long = "sequence-number")]
    sequence_number: Option<u64>,

    #[structopt(long = "gas-price")]
    gas_unit_price: Option<u64>,

    #[structopt(long = "gas-currency")]
    gas_currency_code: Option<String>,

    #[structopt(long = "show-events")]
    show_events: bool,

    #[structopt(long = "secondary-signers", parse(try_from_str = SignerAndKeyPair::parse))]
    secondary_signers: Option<Vec<SignerAndKeyPair>>,
}

/// Diem-specifc arguments for the init command.
#[derive(StructOpt, Debug)]
struct DiemInitArgs {
    #[structopt(long = "private-keys", parse(try_from_str = parse_named_private_key))]
    private_keys: Option<Vec<(Identifier, Ed25519PrivateKey)>>,

    #[structopt(long = "validators", parse(try_from_str = parse_identifier))]
    validators: Option<Vec<Identifier>>,

    #[structopt(long = "parent-vasps", parse(try_from_str = ParentVaspInitArgs::parse))]
    parent_vasps: Option<Vec<ParentVaspInitArgs>>,
}

/// A raw private key -- either a literal or an unresolved name.
#[derive(Debug)]
enum RawPrivateKey {
    Named(Identifier),
    Anonymous(Ed25519PrivateKey),
}

/// A fully qualified type name, where the address could be either a literal or an unresolved name.
#[derive(Debug)]
struct TypeName {
    address: RawAddress,
    module_name: Identifier,
    type_name: Identifier,
}

/// Arguments to initialize a parent vasp account.
#[derive(Debug)]
struct ParentVaspInitArgs {
    name: Identifier,
    currency_type: TypeName,
}

/// Command to initiate a block metadata transaction.
#[derive(StructOpt, Debug)]
struct BlockCommand {
    #[structopt(long = "proposer", parse(try_from_str = RawAddress::parse))]
    proposer: RawAddress,

    #[structopt(long = "time")]
    time: u64,
}

/// Custom commands for the Diem transactional test flow.
#[derive(StructOpt, Debug)]
enum DiemSubCommand {
    #[structopt(name = "block")]
    BlockCommand(BlockCommand),
}

/*************************************************************************************************
 *
 * Parsing
 *
 *
 ************************************************************************************************/

impl TypeName {
    fn parse(s: &str) -> Result<Self> {
        let parts = s.split("::").collect::<Vec<_>>();

        if parts.len() != 3 {
            bail!(
                "Invalid type name {}. Must be of form <addr>::<module_name>::<type_name>",
                s
            )
        }

        let address = RawAddress::parse(parts[0])?;
        let module_name = Identifier::new(parts[1])
            .map_err(|_| format_err!("Invalid module name {}. Expected identifier.", parts[1]))?;
        let type_name = Identifier::new(parts[1])
            .map_err(|_| format_err!("Invalid type name {}. Expected identifier.", parts[2]))?;

        Ok(Self {
            address,
            module_name,
            type_name,
        })
    }
}

impl ParentVaspInitArgs {
    fn parse(s: &str) -> Result<Self> {
        if let Ok(name) = Identifier::new(s) {
            return Ok(Self {
                name,
                currency_type: TypeName {
                    address: RawAddress::Named(Identifier::new("DiemFramework").unwrap()),
                    module_name: XUS_IDENTIFIER.to_owned(),
                    type_name: XUS_IDENTIFIER.to_owned(),
                },
            });
        }

        let parts = s.split('=').collect::<Vec<_>>();
        if parts.len() != 2 {
            bail!("Invalid parent VSAP. Must be either <name> or <name>=<currency_type_tag>, but found {}.", s);
        }

        let name = Identifier::new(parts[0]).map_err(|_| {
            format_err!(
                "Invalid parent vasp name {}. Expected identifier.",
                parts[0]
            )
        })?;
        let currency_type = TypeName::parse(parts[1])?;

        Ok(Self {
            name,
            currency_type,
        })
    }
}

fn parse_identifier(s: &str) -> Result<Identifier> {
    Identifier::new(s).map_err(|_| format_err!("Failed to parse identifier"))
}

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
        if let Ok(address) = RawAddress::parse(s) {
            return Ok(Self {
                address,
                private_key: None,
            });
        };

        let before_after = s.split('=').collect::<Vec<_>>();

        if before_after.len() != 2 {
            bail!("Invalid signer and key pair. Must be of the form <signer addr>=<private_key> or <named signer addr>, but found '{}'", s);
        }

        let address = RawAddress::parse(before_after[0])?;
        let private_key = RawPrivateKey::parse(before_after[1])?;

        Ok(Self {
            address,
            private_key: Some(private_key),
        })
    }
}

/*************************************************************************************************
 *
 * Helpers
 *
 *
 ************************************************************************************************/

/// Default private key mappings for special Diem accounts.
fn diem_framework_private_key_mapping() -> Vec<(String, Ed25519PrivateKey)> {
    vec![
        ("DiemRoot".to_string(), GENESIS_KEYPAIR.0.clone()),
        ("TreasuryCompliance".to_string(), GENESIS_KEYPAIR.0.clone()),
        ("DesignatedDealer".to_string(), GENESIS_KEYPAIR.0.clone()),
    ]
}

fn panic_missing_private_key_named(cmd_name: &str, name: &IdentStr) -> ! {
    panic!(
        "Missing private key. Either add a `--private-key <priv_key>` argument \
            to the {} command, or associate an address to the \
            name '{}' in the init command.",
        cmd_name, name,
    )
}

fn test_only_named_addresses() -> Vec<(String, NumericalAddress)> {
    vec![(
        "DesignatedDealer".to_string(),
        NumericalAddress::new(testnet_dd_account_address().into_bytes(), NumberFormat::Hex),
    )]
}

fn panic_missing_private_key(cmd_name: &str) -> ! {
    panic!(
        "Missing private key. Try adding a `--private-key <priv_key>` \
            argument to the {} command.",
        cmd_name
    )
}

static PRECOMPILED_DIEM_FRAMEWORK: Lazy<FullyCompiledProgram> = Lazy::new(|| {
    let program_res = move_compiler::construct_pre_compiled_lib(
        &diem_framework::diem_stdlib_files(),
        None,
        move_compiler::Flags::empty().set_sources_shadow_deps(false),
        diem_framework::diem_framework_named_addresses(),
    )
    .unwrap();
    match program_res {
        Ok(df) => df,
        Err((files, errors)) => {
            eprintln!("!!!Diem Framework failed to compile!!!");
            move_compiler::diagnostics::report_diagnostics(&files, errors)
        }
    }
});

/*************************************************************************************************
 *
 * Test Adapter Implementation
 *
 *
 ************************************************************************************************/

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
                (None, RawAddress::Named(named_addr)) => {
                    match self.private_key_mapping.get(named_addr) {
                        Some(private_key) => private_key.clone(),
                        None => panic!(
                            "Failed to resolve private key for secondary signer {}.",
                            named_addr
                        ),
                    }
                }
                (None, RawAddress::Anonymous(addr)) => {
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
    fn fetch_transaction_parameters(
        &self,
        signer_addr: &AccountAddress,
        sequence_number: Option<u64>,
        expiration_time: Option<u64>,
        gas_currency_code: Option<String>,
        gas_unit_price: Option<u64>,
        max_gas_amount: Option<u64>,
    ) -> Result<TransactionParameters> {
        let account_resource = self.fetch_account_resource(signer_addr)?;

        let sequence_number = sequence_number.unwrap_or_else(|| account_resource.sequence_number());
        let gas_currency_code = gas_currency_code.unwrap_or_else(|| XUS_NAME.to_string());
        let max_number_of_gas_units = GasConstants::default().maximum_number_of_gas_units;
        let gas_unit_price = gas_unit_price.unwrap_or(0);
        let max_gas_amount = match max_gas_amount {
            Some(max_gas_amount) => max_gas_amount,
            None => {
                if gas_unit_price == 0 {
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
                }
            }
        };
        let expiration_timestamp_secs = expiration_time.unwrap_or(40000);

        Ok(TransactionParameters {
            sequence_number,
            gas_currency_code,
            gas_unit_price,
            max_gas_amount,
            expiration_timestamp_secs,
        })
    }

    /// Perform a single Diem transaction.
    ///
    /// Should error if the transaction ends up being discarded, or having a status other than
    /// EXECUTED.
    fn run_transaction(&mut self, txn: Transaction) -> Result<TransactionOutput> {
        let mut outputs = DiemVM::execute_block_and_keep_vm_status(vec![txn], &self.storage)?;

        assert_eq!(outputs.len(), 1);

        let (status, output) = outputs.pop().unwrap();
        match output.status() {
            TransactionStatus::Keep(kept_vm_status) => {
                self.storage.add_write_set(output.write_set());
                match kept_vm_status {
                    KeptVMStatus::Executed => Ok(output),
                    _ => {
                        bail!("Failed to execute transaction. VMStatus: {}", status)
                    }
                }
            }
            TransactionStatus::Discard(_) => {
                bail!("Transaction discarded. VMStatus: {}", status)
            }
            TransactionStatus::Retry => panic!(),
        }
    }

    /// Create a validator account with the given credentials.
    ///
    /// Note: this does not add it to the named address or private key mappings.
    /// That needs to be done separately.
    ///
    /// TODO: the Genesis code seems to imply that one can use the same account as a validator owner and a
    /// validator operator, but the `public(script) fun set_validator_operator` aborted when I tried to do
    /// so. We should check if this is intended.
    fn create_validator_account(
        &mut self,
        validator_name: Identifier,

        validator_private_key: Ed25519PrivateKey,
        validator_account_addr: AccountAddress,
        validator_auth_key_prefix: Vec<u8>,

        operator_private_key: Ed25519PrivateKey,
        operator_account_addr: AccountAddress,
        operator_auth_key_prefix: Vec<u8>,
    ) {
        // Step 1. Create validator account.
        let parameters = self
            .fetch_transaction_parameters(&diem_root_address(), None, None, None, None, None)
            .unwrap();
        let txn = RawTransaction::new(
            diem_root_address(),
            parameters.sequence_number,
            diem_transaction_builder::experimental_stdlib::encode_create_validator_account_script_function(
                0,
                validator_account_addr,
                validator_auth_key_prefix,
                validator_name.as_bytes().into(),
            ),
            parameters.max_gas_amount,
            parameters.gas_unit_price,
            parameters.gas_currency_code,
            parameters.expiration_timestamp_secs,
            ChainId::test(),
        )
        .sign(&GENESIS_KEYPAIR.0, GENESIS_KEYPAIR.1.clone())
        .unwrap()
        .into_inner();
        self.run_transaction(Transaction::UserTransaction(txn))
            .expect("Failed to create validator account. This should not happen.");

        // Step 2. Create validator operator account.
        let parameters = self
            .fetch_transaction_parameters(&diem_root_address(), None, None, None, None, None)
            .unwrap();
        let txn = RawTransaction::new(
            diem_root_address(),
            parameters.sequence_number,
            diem_transaction_builder::experimental_stdlib::encode_create_validator_operator_account_script_function(
                0,
                operator_account_addr,
                operator_auth_key_prefix,
                validator_name.as_bytes().into(),
            ),
            parameters.max_gas_amount,
            parameters.gas_unit_price,
            parameters.gas_currency_code,
            parameters.expiration_timestamp_secs,
            ChainId::test(),
        )
        .sign(&GENESIS_KEYPAIR.0, GENESIS_KEYPAIR.1.clone())
        .unwrap()
        .into_inner();
        self.run_transaction(Transaction::UserTransaction(txn))
            .expect("Failed to create validator operator account. This should not happen.");

        // Step 3. Set validator operator account.
        let parameters = self
            .fetch_transaction_parameters(&validator_account_addr, None, None, None, None, None)
            .unwrap();
        let txn = RawTransaction::new(
            validator_account_addr,
            parameters.sequence_number,
            diem_transaction_builder::experimental_stdlib::encode_set_validator_operator_script_function(
                validator_name.as_bytes().into(),
                operator_account_addr,
            ),
            parameters.max_gas_amount,
            parameters.gas_unit_price,
            parameters.gas_currency_code,
            parameters.expiration_timestamp_secs,
            ChainId::test(),
        )
        .sign(
            &validator_private_key,
            validator_private_key.public_key(),
        )
        .unwrap()
        .into_inner();
        self.run_transaction(Transaction::UserTransaction(txn))
            .expect("Failed to set validator operator. This should not happen.");

        // Step 4. Set validator config.
        let parameters = self
            .fetch_transaction_parameters(&operator_account_addr, None, None, None, None, None)
            .unwrap();
        let txn = RawTransaction::new(
            operator_account_addr,
            parameters.sequence_number,
            diem_transaction_builder::experimental_stdlib::encode_register_validator_config_script_function(validator_account_addr, validator_private_key.public_key().to_bytes().to_vec(), vec![], vec![]),
            parameters.max_gas_amount,
            parameters.gas_unit_price,
            parameters.gas_currency_code,
            parameters.expiration_timestamp_secs,
            ChainId::test(),
        )
        .sign(
            &operator_private_key,
            operator_private_key.public_key(),
        )
        .unwrap()
        .into_inner();
        self.run_transaction(Transaction::UserTransaction(txn))
            .expect("Failed to set validator config. This should not happen.");

        // Step 5. Add validator to validator set.
        let parameters = self
            .fetch_transaction_parameters(&diem_root_address(), None, None, None, None, None)
            .unwrap();
        let txn = RawTransaction::new(
                diem_root_address(),
                parameters.sequence_number,
                diem_transaction_builder::experimental_stdlib::encode_add_validator_and_reconfigure_script_function(
                    0,
                    validator_name.as_bytes().into(),
                    validator_account_addr,
                ),
                parameters.max_gas_amount,
                parameters.gas_unit_price,
                parameters.gas_currency_code,
                parameters.expiration_timestamp_secs,
                ChainId::test(),
            )
            .sign(&GENESIS_KEYPAIR.0, GENESIS_KEYPAIR.1.clone())
            .unwrap()
            .into_inner();
        self.run_transaction(Transaction::UserTransaction(txn))
            .expect("Failed to add validator to validator set. This should not happen.");
    }

    /// Create a parent vasp account with the given credentials.
    ///
    /// Note: this does not add it to the named address or private key mappings.
    /// That needs to be done separately.
    fn create_parent_vasp_account(
        &mut self,
        validator_name: Identifier,
        auth_key_prefix: Vec<u8>,
        account_addr: AccountAddress,
        currency_type_name: TypeName,
    ) {
        let parameters = self
            .fetch_transaction_parameters(
                &treasury_compliance_account_address(),
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();

        let currency_type_tag = {
            let address = self
                .compiled_state()
                .resolve_address(&currency_type_name.address);
            TypeTag::Struct(StructTag {
                address,
                module: currency_type_name.module_name,
                name: currency_type_name.type_name,
                type_params: vec![],
            })
        };

        let txn = RawTransaction::new(
            treasury_compliance_account_address(),
            parameters.sequence_number,
            diem_transaction_builder::stdlib::encode_create_parent_vasp_account_script_function(
                currency_type_tag,
                0,
                account_addr,
                auth_key_prefix,
                validator_name.as_bytes().into(),
                false,
            ),
            parameters.max_gas_amount,
            parameters.gas_unit_price,
            parameters.gas_currency_code,
            parameters.expiration_timestamp_secs,
            ChainId::test(),
        )
        .sign(&GENESIS_KEYPAIR.0, GENESIS_KEYPAIR.1.clone())
        .unwrap()
        .into_inner();

        self.run_transaction(Transaction::UserTransaction(txn))
            .expect("Failed to create parent vasp account. This should not happen.");
    }
}

impl<'a> MoveTestAdapter<'a> for DiemTestAdapter<'a> {
    type ExtraInitArgs = DiemInitArgs;
    type ExtraPublishArgs = DiemPublishArgs;
    type ExtraRunArgs = DiemRunArgs;
    type Subcommand = DiemSubCommand;

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
        // Named address mapping
        let additional_named_address_mapping = match task_opt.as_ref().map(|t| &t.command) {
            Some((InitCommand { named_addresses }, _)) => {
                verify_and_create_named_address_mapping(named_addresses.clone()).unwrap()
            }
            None => BTreeMap::new(),
        };

        let mut named_address_mapping = diem_framework::diem_framework_named_addresses();

        for (name, addr) in test_only_named_addresses() {
            assert!(!named_address_mapping.contains_key(&name));
            named_address_mapping.insert(name, addr);
        }

        for (name, addr) in additional_named_address_mapping {
            if named_address_mapping.contains_key(&name) {
                panic!("Invalid init. The named address '{}' already exists.", name)
            }
            named_address_mapping.insert(name, addr);
        }

        // Genesis modules
        // TODO: rework vm-genesis and try not to compile the genesis modules twice.
        let mut storage = FakeDataStore::new(HashMap::new());
        storage.add_write_set(GENESIS_CHANGE_SET_FRESH.write_set());

        // Builtin private key mapping
        let mut private_key_mapping = BTreeMap::new();
        for (name, private_key) in diem_framework_private_key_mapping() {
            private_key_mapping.insert(Identifier::new(name).unwrap(), private_key);
        }

        // Handle extra init args
        let mut keygen = KeyGen::from_seed([0; 32]);
        let mut validators_to_create = vec![];
        let mut parent_vasps_to_create = vec![];

        if let Some(TaskInput {
            command: (_, init_args),
            ..
        }) = task_opt
        {
            // Private key mapping
            if let Some(additional_private_key_mapping) = init_args.private_keys {
                for (name, private_key) in additional_private_key_mapping {
                    if private_key_mapping.contains_key(&name) {
                        panic!(
                            "Invalid init. The named private key '{}' already exists.",
                            name
                        )
                    }
                    private_key_mapping.insert(name, private_key);
                }
            }

            // Validators
            if let Some(validators) = init_args.validators {
                for validator_name in validators {
                    if named_address_mapping.contains_key(validator_name.as_str()) {
                        panic!(
                            "Invalid validator name {} -- named address already exists.",
                            validator_name
                        )
                    }
                    if private_key_mapping.contains_key(&validator_name) {
                        panic!(
                            "Invalid validator name {} -- named private key already exists.",
                            validator_name
                        )
                    }

                    let (validator_private_key, validator_auth_key_prefix, validator_account_addr) =
                        keygen.generate_credentials_for_account_creation();

                    let (operator_private_key, operator_auth_key_prefix, operator_account_addr) =
                        keygen.generate_credentials_for_account_creation();

                    named_address_mapping.insert(
                        validator_name.to_string(),
                        NumericalAddress::new(
                            validator_account_addr.into_bytes(),
                            NumberFormat::Hex,
                        ),
                    );
                    private_key_mapping
                        .insert(validator_name.clone(), validator_private_key.clone());

                    // Note: validator accounts are created at a later time.
                    // This is because we need to fetch the sequence number of DiemRoot, which is
                    // only available after the DiemTestAdapter has been fully initialized.
                    validators_to_create.push((
                        validator_name,
                        validator_private_key,
                        validator_account_addr,
                        validator_auth_key_prefix,
                        operator_private_key,
                        operator_account_addr,
                        operator_auth_key_prefix,
                    ));
                }
            }

            // Parent Vasps
            if let Some(parent_vasps) = init_args.parent_vasps {
                for parent_vasp_init_args in parent_vasps {
                    let parent_vasp_name = parent_vasp_init_args.name;
                    if named_address_mapping.contains_key(parent_vasp_name.as_str()) {
                        panic!(
                            "Invalid validator name {} -- named address already exists.",
                            parent_vasp_name
                        )
                    }
                    if private_key_mapping.contains_key(&parent_vasp_name) {
                        panic!(
                            "Invalid validator name {} -- named private key already exists.",
                            parent_vasp_name
                        )
                    }

                    let (private_key, auth_key_prefix, account_addr) =
                        keygen.generate_credentials_for_account_creation();
                    named_address_mapping.insert(
                        parent_vasp_name.to_string(),
                        NumericalAddress::new(account_addr.into_bytes(), NumberFormat::Hex),
                    );
                    private_key_mapping.insert(parent_vasp_name.clone(), private_key);

                    // Note: parent vasp accounts are created at a later time.
                    // This is because we need to fetch the sequence number of DiemRoot, which is
                    // only available after the DiemTestAdapter has been fully initialized.
                    parent_vasps_to_create.push((
                        parent_vasp_name,
                        auth_key_prefix,
                        account_addr,
                        parent_vasp_init_args.currency_type,
                    ));
                }
            }
        }

        let mut adapter = Self {
            compiled_state: CompiledState::new(named_address_mapping, pre_compiled_deps),
            default_syntax,
            storage,
            private_key_mapping,
        };

        // Create validator accounts
        for (
            validator_name,
            validator_private_key,
            validator_account_addr,
            validator_auth_key_prefix,
            operator_private_key,
            operator_account_addr,
            operator_auth_key_prefix,
        ) in validators_to_create
        {
            adapter.create_validator_account(
                validator_name,
                validator_private_key,
                validator_account_addr,
                validator_auth_key_prefix,
                operator_private_key,
                operator_account_addr,
                operator_auth_key_prefix,
            );
        }

        // Create parent vasp accounts
        for (parent_vasp_name, auth_key_prefix, account_addr, currency_type_name) in
            parent_vasps_to_create
        {
            adapter.create_parent_vasp_account(
                parent_vasp_name,
                auth_key_prefix,
                account_addr,
                currency_type_name,
            );
        }

        adapter
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
        let params = self.fetch_transaction_parameters(
            signer,
            extra_args.sequence_number,
            extra_args.expiration_time,
            extra_args.gas_currency_code,
            extra_args.gas_unit_price,
            gas_budget,
        )?;

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
            params.max_gas_amount,
            params.gas_unit_price,
            params.gas_currency_code,
            params.expiration_timestamp_secs,
            ChainId::test(),
        )
        .sign(&private_key, Ed25519PublicKey::from(&private_key))?
        .into_inner();

        self.run_transaction(Transaction::UserTransaction(txn))?;

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
    ) -> Result<Option<String>> {
        if !extra_args.admin_script {
            panic!(
                "Transactions scripts are not currently allowed on Diem. \
                If you intend to run an admin script, add the `--admin-script` option to the run command."
            )
        }

        if signers.len() != 2 {
            panic!("Expected 2 signer, got {}.", signers.len());
        }
        let signer0 = self.compiled_state().resolve_address(&signers[0]);
        let signer1 = self.compiled_state().resolve_address(&signers[1]);

        if gas_budget.is_some() {
            panic!("Cannot set gas budget for admin script.")
        }
        if extra_args.gas_unit_price.is_some() {
            panic!("Cannot set gas price for admin script.")
        }
        if extra_args.gas_currency_code.is_some() {
            panic!("Cannot set gas currency for admin script.")
        }
        if extra_args.expiration_time.is_some() {
            panic!("Cannot set expiration time for admin script.")
        }
        if extra_args.secondary_signers.is_some() {
            panic!("Cannot set secondary signers for admin script.")
        }

        let private_key = match (extra_args.private_key, &signers[0]) {
            (Some(private_key), _) => self.resolve_private_key(&private_key),
            (None, RawAddress::Named(named_addr)) => match self.private_key_mapping.get(named_addr)
            {
                Some(private_key) => private_key.clone(),
                None => panic_missing_private_key_named("run", named_addr),
            },
            (None, RawAddress::Anonymous(_)) => panic_missing_private_key("run"),
        };

        let mut script_blob = vec![];
        script.serialize(&mut script_blob)?;

        let params = self.fetch_transaction_parameters(
            &signer0,
            extra_args.sequence_number,
            None,
            None,
            None,
            None,
        )?;

        let txn = RawTransaction::new_writeset_script(
            signer0,
            params.sequence_number,
            TransactionScript::new(script_blob, type_args, txn_args),
            signer1,
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

        Ok(output)
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
    ) -> Result<Option<String>> {
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
            (None, RawAddress::Anonymous(_)) => panic_missing_private_key("run"),
        };

        let params = self.fetch_transaction_parameters(
            &signer,
            extra_args.sequence_number,
            extra_args.expiration_time,
            extra_args.gas_currency_code,
            extra_args.gas_unit_price,
            gas_budget,
        )?;
        let txn = RawTransaction::new_script_function(
            signer,
            params.sequence_number,
            TransactionScriptFunction::new(
                module.clone(),
                function.to_owned(),
                type_args,
                convert_txn_args(&txn_args),
            ),
            params.max_gas_amount,
            params.gas_unit_price,
            params.gas_currency_code,
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

        Ok(output)
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
            DiemSubCommand::BlockCommand(block_cmd) => {
                let proposer = self.compiled_state().resolve_address(&block_cmd.proposer);
                let metadata =
                    BlockMetadata::new(HashValue::zero(), 0, block_cmd.time, vec![], proposer);

                let output = self.run_transaction(Transaction::BlockMetadata(metadata))?;

                Ok(render_events(output.events()))
            }
        }
    }
}

/*************************************************************************************************
 *
 * Misc
 *
 *
 ************************************************************************************************/
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

/// Run the Diem transactional test flow, using the given file as input.
pub fn run_test(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    run_test_impl::<DiemTestAdapter>(path, Some(&*PRECOMPILED_DIEM_FRAMEWORK))
}
