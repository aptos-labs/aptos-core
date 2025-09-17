// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{Account, TxnArg, TxnArgType, TxnArgTypeWithRef},
    language::LanguageSetting,
    subexec::SubExec,
};
use anyhow::{anyhow, bail, Result};
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey};
use aptos_types::transaction::authenticator::AuthenticationKey;
use lazy_static::lazy_static;
use log::{debug, error, info};
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    binary_views::BinaryIndexedView,
    file_format::{
        CompiledModule, CompiledScript, FunctionDefinition, SignatureToken, StructDefinition,
        Visibility,
    },
};
use move_core_types::{ability::AbilitySet, account_address::AccountAddress};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, RwLock},
};
use tempfile::TempDir;

/// Disables telemetry
const ENV_APTOS_DISABLE_TELEMETRY: &str = "APTOS_DISABLE_TELEMETRY";

/// Default gas unit price
const DEFAULT_GAS_UNIT_PRICE: u32 = 100;
// TODO(mengxu): kept in sync with aptos_config::global_constants::GAS_UNIT_PRICE

/// Default gas maximum
const DEFAULT_MAX_GAS_AMOUNT: u32 = 2_000_000;
// TODO(mengxu): kept in sync with aptos_config::global_constants::MAX_GAS_AMOUNT

fn fund_amount_for_num_txns(count: u64) -> Result<u64> {
    let gas_budget_per_txn = u64::from(DEFAULT_MAX_GAS_AMOUNT)
        .checked_mul(u64::from(DEFAULT_GAS_UNIT_PRICE))
        .expect("gas constants should fit in u64");
    count
        .checked_mul(gas_budget_per_txn)
        .ok_or_else(|| anyhow!("requested faucet funding overflow for {count} transactions"))
}

lazy_static! {
    /// Path to the current Aptos CLI
    static ref APTOS_BIN: PathBuf =
        std::env::current_exe()
            .and_then(|p| p.canonicalize())
            .expect("current executable path");
}

/// Whether this is an owned or referred address
pub enum AddressNamespace {
    Ref(BTreeSet<String>),
    Owned(String),
}

/// Configuration for the simulator
struct Config {
    /// language setting
    language: LanguageSetting,
    /// use realistic gas setting
    realistic_gas: bool,
}

/// Target that can be executed
pub enum Runnable {
    Entry {
        address: AccountAddress,
        module: String,
        function: String,
    },
    Script {
        path: PathBuf,
    },
}

/// Simulator for local testnet
pub struct Simulator {
    /// simulator config
    config: Config,
    /// temporary working directory
    workdir: TempDir,
    /// the local testnet process in background
    executor: SubExec,

    /// accounts registered
    named_accounts: BTreeMap<String, Account>,
    /// address reverse lookup
    address_lookup: BTreeMap<AccountAddress, AddressNamespace>,
    /// sender-address -> signer profile mapping for resource accounts
    resource_signers: BTreeMap<AccountAddress, String>,

    /// compiled modules
    modules: BTreeMap<AccountAddress, BTreeMap<PathBuf, BTreeMap<String, CompiledModule>>>,
    /// compiled scripts
    scripts: BTreeMap<String, CompiledScript>,
}

impl Simulator {
    /// Initialize a new simulator
    pub fn new(language: LanguageSetting, realistic_gas: bool) -> Result<Self> {
        // always create a fresh temp directory
        let workdir = TempDir::new()?;

        // launch the testnet
        info!("launching local testnet");
        let mut command = Command::new(APTOS_BIN.as_path());
        command
            .args([
                "node",
                "run-local-testnet",
                "--with-faucet",
                "--force-restart",
            ])
            .env(ENV_APTOS_DISABLE_TELEMETRY, "1")
            .current_dir(workdir.path());

        let stderr = Arc::new(RwLock::new(vec![]));
        let mut executor = SubExec::run(command, None, Some(Arc::clone(&stderr)))?;

        // wait for testnet to be ready
        debug!("waiting for local testnet to initialize");
        let mut next_line = 0;
        'wait: loop {
            // check if the testnet is still running
            match executor.probe() {
                Ok(None) => (),
                Ok(Some(status)) => bail!("local testnet process terminated prematurely: {status}"),
                Err(err) => {
                    error!("unable to probe local testnet process: {err}");
                    executor.interrupt()?;
                    bail!("local testnet process killed due to probing error");
                },
            };

            // analyze the output
            let lines = stderr.read().expect("stderr read lock");
            let count = lines.len();
            if count == next_line {
                continue;
            }

            // new content available
            for line in &lines[next_line..count] {
                if line == "Setup is complete, you can now use the localnet!" {
                    break 'wait;
                }
            }
            next_line = count;
        }

        // now we have a new simulator
        info!("local testnet is ready");
        Ok(Self {
            config: Config {
                language,
                realistic_gas,
            },
            workdir,
            executor,
            named_accounts: BTreeMap::new(),
            address_lookup: BTreeMap::new(),
            resource_signers: BTreeMap::new(),
            modules: BTreeMap::new(),
            scripts: BTreeMap::new(),
        })
    }

    /// Add a dependency address to the system
    pub fn add_address(&mut self, name: String, address: AccountAddress) -> Result<()> {
        // ensure no duplication
        if self.named_accounts.contains_key(&name) {
            bail!("address already exists: {}", name);
        }

        // add the pair
        if let Some(ns) = self.address_lookup.get_mut(&address) {
            match ns {
                AddressNamespace::Ref(names) => {
                    if !names.insert(name.clone()) {
                        bail!(
                            "duplicated address registration: @{} => {}",
                            name,
                            address.to_standard_string()
                        );
                    }
                },
                AddressNamespace::Owned(..) => bail!(
                    "address cannot be both owned and referred: @{} => {}",
                    name,
                    address.to_standard_string()
                ),
            }
        } else {
            let mut names = BTreeSet::new();
            names.insert(name.clone());
            self.address_lookup
                .insert(address, AddressNamespace::Ref(names));
        }
        self.named_accounts.insert(name, Account::Ref(address));

        // done
        Ok(())
    }

    /// Register a user account and also fund it if requested
    pub fn register_account(
        &mut self,
        name: String,
        key: Ed25519PrivateKey,
        fund_for_num_txns: Option<u64>,
    ) -> Result<AccountAddress> {
        // ensure no duplication
        if self.named_accounts.contains_key(&name) {
            bail!("address already exists: {}", name);
        }

        // register account
        let key_string = format!("0x{}", hex::encode(key.to_bytes()));
        let mut command = Command::new(APTOS_BIN.as_path());
        command
            .args([
                "init",
                "--network",
                "local",
                "--profile",
                &name,
                "--private-key",
                &key_string,
                "--skip-faucet",
                "--assume-yes",
            ])
            .env(ENV_APTOS_DISABLE_TELEMETRY, "1")
            .current_dir(self.workdir.path());
        if !SubExec::invoke(command)? {
            bail!("failed to create account {}", name);
        }
        debug!("account registered: {}", name);

        // fund the account if requested
        match fund_for_num_txns {
            None | Some(0) => (),
            Some(count) => {
                let fund = fund_amount_for_num_txns(count)?;
                let mut command = Command::new(APTOS_BIN.as_path());
                command
                    .args([
                        "account",
                        "fund-with-faucet",
                        "--profile",
                        &name,
                        "--account",
                        &name,
                        "--amount",
                        &fund.to_string(),
                    ])
                    .env(ENV_APTOS_DISABLE_TELEMETRY, "1")
                    .current_dir(self.workdir.path());
                if !SubExec::invoke(command)? {
                    bail!("failed to fund account {} with {} tokens", name, fund);
                }
                debug!("account {} is funded with {} tokens", name, fund);
            },
        }

        // add the pair
        let address = AuthenticationKey::ed25519(&key.public_key()).account_address();
        let existing = self
            .address_lookup
            .insert(address, AddressNamespace::Owned(name.clone()));
        if existing.is_some() {
            bail!(
                "duplicated account address registration: @{} => {}",
                name,
                address.to_standard_string()
            );
        }
        self.named_accounts.insert(name, Account::Owned(key));

        // done
        Ok(address)
    }

    pub fn add_resource_address(
        &mut self,
        name: String,
        address: AccountAddress,
        signer_profile: String,
    ) -> Result<()> {
        self.add_address(name, address)?;
        match self
            .resource_signers
            .insert(address, signer_profile.clone())
        {
            None => Ok(()),
            Some(existing) if existing == signer_profile => Ok(()),
            Some(existing) => bail!(
                "conflicting signer profiles for resource address {}: {} vs {}",
                address.to_standard_string(),
                existing,
                signer_profile
            ),
        }
    }

    /// Lookup address by name
    pub fn get_address(&self, name: &str) -> Option<AccountAddress> {
        self.named_accounts
            .get(name)
            .map(|account| account.address())
    }

    /// Lookup namespace by address
    pub fn lookup_namespace_by_address(
        &self,
        address: &AccountAddress,
    ) -> Option<&AddressNamespace> {
        self.address_lookup.get(address)
    }

    pub fn signing_profile_for_address(&self, address: &AccountAddress) -> Option<&str> {
        match self.address_lookup.get(address) {
            Some(AddressNamespace::Owned(name)) => Some(name.as_str()),
            Some(AddressNamespace::Ref(_)) | None => {
                self.resource_signers.get(address).map(String::as_str)
            },
        }
    }

    /// Publish a package
    pub fn publish_package(
        &mut self,
        package_name: &str,
        package_path: &Path,
        signer_profile: &str,
        sender_account: &AccountAddress,
        named_addresses: &BTreeMap<String, AccountAddress>,
        language: LanguageSetting,
    ) -> Result<()> {
        let named_address_pairs: Vec<_> = named_addresses
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        // command: basics
        let mut command = Command::new(APTOS_BIN.as_path());
        command.args(["move", "publish"]);
        // command: sender
        command.args([
            "--profile",
            signer_profile,
            "--sender-account",
            &sender_account.to_standard_string(),
        ]);
        // command: gas
        if !self.config.realistic_gas {
            command.args([
                "--gas-unit-price",
                &DEFAULT_GAS_UNIT_PRICE.to_string(),
                "--max-gas",
                &DEFAULT_MAX_GAS_AMOUNT.to_string(),
            ]);
        }
        // command: project
        command
            .arg("--package-dir")
            .arg(package_path)
            .arg("--named-addresses")
            .arg(named_address_pairs.join(","))
            .arg("--skip-fetch-latest-git-deps");
        // command: language
        language.derive_cli_options(&mut command);
        // command: configs
        command.args(["--included-artifacts", "none", "--override-size-check"]);
        // command: misc
        command
            .arg("--assume-yes")
            .env(ENV_APTOS_DISABLE_TELEMETRY, "1")
            .current_dir(self.workdir.path());

        if !SubExec::invoke(command)? {
            bail!("failed to publish package {}", package_name);
        }
        Ok(())
    }

    /// Add a module
    pub fn add_module(&mut self, pkg_dir: PathBuf, module: CompiledModule) -> Result<()> {
        let module_addr = *module.address();
        let module_name = module.name().to_string();
        if self
            .modules
            .get(&module_addr)
            .is_some_and(|l1| l1.iter().any(|(_, l2)| l2.contains_key(&module_name)))
        {
            bail!(
                "two modules share the same key: {}::{}",
                module_addr.to_standard_string(),
                module_name
            );
        }
        // add the module
        self.modules
            .entry(module_addr)
            .or_default()
            .entry(pkg_dir)
            .or_default()
            .insert(module_name, module);
        Ok(())
    }

    /// Look for a matching function in a module
    fn find_function_definition_in_module<'a>(
        module: &'a CompiledModule,
        function_name: &str,
    ) -> Option<&'a FunctionDefinition> {
        for def in module.function_defs() {
            let handle = module.function_handle_at(def.function);
            if module.identifier_at(handle.name).as_str() == function_name {
                return Some(def);
            }
        }
        None
    }

    /// Look for a matching struct in a module
    fn find_struct_definition_in_module<'a>(
        module: &'a CompiledModule,
        struct_name: &str,
    ) -> Option<&'a StructDefinition> {
        for def in module.struct_defs() {
            let handle = module.struct_handle_at(def.struct_handle);
            if module.identifier_at(handle.name).as_str() == struct_name {
                return Some(def);
            }
        }
        None
    }

    /// Look up an item in a package
    fn lookup_in_package<'a, T, F>(
        modules: &'a BTreeMap<String, CompiledModule>,
        module_name: Option<&str>,
        item_name: &str,
        finder: F,
    ) -> Vec<(&'a CompiledModule, &'a T)>
    where
        F: Fn(&'a CompiledModule, &str) -> Option<&'a T>,
    {
        let mut candidates = vec![];
        match module_name {
            None => {
                for module in modules.values() {
                    match finder(module, item_name) {
                        None => continue,
                        Some(item) => candidates.push((module, item)),
                    }
                }
            },
            Some(name) => match modules.get(name) {
                None => (),
                Some(module) => match finder(module, item_name) {
                    None => (),
                    Some(item) => candidates.push((module, item)),
                },
            },
        }
        candidates
    }

    /// Look up an item in an address
    fn lookup_in_address<'a, T, F>(
        packages: &'a BTreeMap<PathBuf, BTreeMap<String, CompiledModule>>,
        module_name: Option<&str>,
        item_name: &str,
        finder: F,
    ) -> Vec<(&'a Path, &'a CompiledModule, &'a T)>
    where
        F: Fn(&'a CompiledModule, &str) -> Option<&'a T>,
    {
        let mut candidates = vec![];
        for (pkg_dir, modules) in packages {
            for (module, item) in Self::lookup_in_package(modules, module_name, item_name, &finder)
            {
                candidates.push((pkg_dir.as_path(), module, item));
            }
        }
        candidates
    }

    /// Look up an item in the whole project
    fn lookup_in_project<'a, T, F>(
        project: &'a BTreeMap<AccountAddress, BTreeMap<PathBuf, BTreeMap<String, CompiledModule>>>,
        module_addr: Option<AccountAddress>,
        module_name: Option<&str>,
        item_name: &str,
        finder: F,
    ) -> Vec<(&'a Path, &'a CompiledModule, &'a T)>
    where
        F: Fn(&'a CompiledModule, &str) -> Option<&'a T>,
    {
        let mut candidates = vec![];
        match module_addr {
            None => {
                for modules in project.values() {
                    candidates.extend(Self::lookup_in_address(
                        modules,
                        module_name,
                        item_name,
                        &finder,
                    ));
                }
            },
            Some(addr) => match project.get(&addr) {
                None => (),
                Some(modules) => {
                    candidates.extend(Self::lookup_in_address(
                        modules,
                        module_name,
                        item_name,
                        &finder,
                    ));
                },
            },
        }
        candidates
    }

    /// Lookup a function by an optional module id, function name, and a filter
    fn lookup_in_project_with_filter<'a, T, F, V>(
        &'a self,
        module_id: Option<(Option<AccountAddress>, &str)>,
        item_name: &str,
        finder: F,
        filter: V,
    ) -> Result<(&'a Path, &'a CompiledModule, &'a T)>
    where
        F: Fn(&'a CompiledModule, &str) -> Option<&'a T>,
        V: Fn(&'a CompiledModule, &'a T) -> bool,
    {
        let candidates = match module_id {
            None => Self::lookup_in_project(&self.modules, None, None, item_name, finder),
            Some((None, module_name)) => {
                Self::lookup_in_project(&self.modules, None, Some(module_name), item_name, finder)
            },
            Some((Some(module_addr), module_name)) => Self::lookup_in_project(
                &self.modules,
                Some(module_addr),
                Some(module_name),
                item_name,
                finder,
            ),
        };

        let mut found = Err(0);
        for (path, module, item) in candidates {
            if !filter(module, item) {
                continue;
            }
            match found {
                Err(0) => found = Ok((path, module, item)),
                Ok(_) => found = Err(2),
                Err(n) => found = Err(n + 1),
            }
        }
        found.map_err(|n| {
            anyhow!(
                "{} found for function {}::{}::{}",
                if n == 0 {
                    "no candidate"
                } else {
                    "more than one candidates"
                },
                module_id
                    .and_then(|(addr, _)| addr)
                    .map_or("*".to_string(), |e| e.to_standard_string()),
                module_id.map_or("*", |(_, name)| name),
                item_name
            )
        })
    }

    /// Lookup an entry function
    pub fn lookup_entry_function(
        &self,
        module_id: Option<(Option<AccountAddress>, &str)>,
        function_name: &str,
    ) -> Result<(&Path, &CompiledModule, Vec<AbilitySet>, Vec<TxnArgType>)> {
        let (pkg_dir, module, def) = self.lookup_in_project_with_filter(
            module_id,
            function_name,
            Self::find_function_definition_in_module,
            |_, def| def.is_entry,
        )?;
        let handle = module.function_handle_at(def.function);

        let arg_types: Vec<_> = module
            .signature_at(handle.parameters)
            .0
            .iter()
            .map(|token| TxnArgType::convert(BinaryIndexedView::Module(module), token))
            .collect::<Result<_>>()?;

        Ok((pkg_dir, module, handle.type_parameters.clone(), arg_types))
    }

    /// Lookup a public function
    pub fn lookup_public_function(
        &self,
        module_id: Option<(Option<AccountAddress>, &str)>,
        function_name: &str,
    ) -> Result<(
        &Path,
        &CompiledModule,
        Vec<AbilitySet>,
        Vec<TxnArgTypeWithRef>,
        Option<bool>,
    )> {
        let (pkg_dir, module, def) = self.lookup_in_project_with_filter(
            module_id,
            function_name,
            Self::find_function_definition_in_module,
            |_, def| matches!(def.visibility, Visibility::Public),
        )?;
        let handle = module.function_handle_at(def.function);

        // check its argument types are fine
        let arg_types: Vec<_> = module
            .signature_at(handle.parameters)
            .0
            .iter()
            .map(|token| TxnArgTypeWithRef::convert(BinaryIndexedView::Module(module), token))
            .collect::<Result<_>>()?;

        // ensure that its return type is fine as well
        let ret_ty = module.signature_at(handle.return_);
        let mut return_ref = None;
        let mut iter = ret_ty.0.iter();
        match iter.next() {
            None => (),
            Some(token) => {
                if !TxnArgTypeWithRef::is_droppable(
                    BinaryIndexedView::Module(module),
                    &handle.type_parameters,
                    token,
                ) {
                    bail!("the return type of the public function is not droppable");
                }
                return_ref = Some(matches!(
                    token,
                    SignatureToken::Reference(_) | SignatureToken::MutableReference(_)
                ));
            },
        }
        match iter.next() {
            None => (),
            Some(_) => bail!("expect at most one return type"),
        }

        Ok((
            pkg_dir,
            module,
            handle.type_parameters.clone(),
            arg_types,
            return_ref,
        ))
    }

    /// Lookup a struct
    pub fn lookup_struct(
        &self,
        module_id: Option<(Option<AccountAddress>, &str)>,
        struct_name: &str,
    ) -> Result<(&Path, &CompiledModule)> {
        let (pkg_dir, module, _) = self.lookup_in_project_with_filter(
            module_id,
            struct_name,
            Self::find_struct_definition_in_module,
            |_, _| true,
        )?;
        Ok((pkg_dir, module))
    }

    /// Add a script
    pub fn add_script(&mut self, name: String, script: CompiledScript) -> Result<()> {
        if self.scripts.contains_key(&name) {
            bail!("two scripts share the same name: {}", name);
        }
        self.scripts.insert(name, script);
        Ok(())
    }

    /// Get a script
    pub fn lookup_script(
        &self,
        name: &str,
    ) -> Result<(&CompiledScript, Vec<AbilitySet>, Vec<TxnArgType>)> {
        let script = self
            .scripts
            .get(name)
            .ok_or_else(|| anyhow!("no such script: {}", name))?;

        let arg_types: Vec<_> = script
            .signature_at(script.parameters)
            .0
            .iter()
            .map(|token| TxnArgType::convert(BinaryIndexedView::Script(script), token))
            .collect::<Result<_>>()?;

        Ok((script, script.type_parameters.clone(), arg_types))
    }

    /// Execute a runnable
    pub fn run(
        &self,
        signer_profile: &str,
        sender_account: &AccountAddress,
        runnable: &Runnable,
        ty_args: &[String],
        txn_args: &[TxnArg],
        simulate: bool,
    ) -> Result<(bool, Vec<String>)> {
        let formatted_args: Vec<_> = txn_args
            .iter()
            .map(|arg| match arg {
                TxnArg::Bool(v) => format!("bool:{}", v),
                TxnArg::U8(v) => format!("u8:{}", v),
                TxnArg::I8(v) => format!("i8:{}", v),
                TxnArg::U16(v) => format!("u16:{}", v),
                TxnArg::I16(v) => format!("i16:{}", v),
                TxnArg::U32(v) => format!("u32:{}", v),
                TxnArg::I32(v) => format!("i32:{}", v),
                TxnArg::U64(v) => format!("u64:{}", v),
                TxnArg::I64(v) => format!("i64:{}", v),
                TxnArg::U128(v) => format!("u128:{}", v),
                TxnArg::I128(v) => format!("i128:{}", v),
                TxnArg::U256(v) => format!("u256:{}", v),
                TxnArg::I256(v) => format!("i256:{}", v),
                TxnArg::Address(v) => format!("address:{}", v.to_standard_string()),
                TxnArg::Signer(v) => format!("signer:{}", v.to_standard_string()),
                TxnArg::String(v) => format!("string:{}", v),
                TxnArg::Vector(sub, _) => format!("{}:{}", sub.type_mark(), arg.to_cli_string()),
            })
            .collect();

        // command: basics
        let mut command = Command::new(APTOS_BIN.as_path());
        match runnable {
            Runnable::Entry {
                address,
                module,
                function,
            } => {
                command.args(["move", "run", "--function-id"]);
                command.arg(format!(
                    "{}::{}::{}",
                    address.to_standard_string(),
                    module,
                    function
                ));
            },
            Runnable::Script { path } => {
                command.args(["move", "run-script", "--compiled-script-path"]);
                command.arg(path);
            },
        }
        // command: sender
        command.args([
            "--profile",
            signer_profile,
            "--sender-account",
            &sender_account.to_standard_string(),
        ]);
        // command: gas
        if !self.config.realistic_gas {
            command.args([
                "--gas-unit-price",
                &DEFAULT_GAS_UNIT_PRICE.to_string(),
                "--max-gas",
                &DEFAULT_MAX_GAS_AMOUNT.to_string(),
            ]);
        }
        // command: type arguments
        if !ty_args.is_empty() {
            command.arg("--type-args").args(ty_args);
        }
        // command: args
        if !formatted_args.is_empty() {
            command.arg("--args").args(formatted_args);
        }
        // command: configs
        if simulate {
            command.arg("--local");
        }
        // command: misc
        command
            .env(ENV_APTOS_DISABLE_TELEMETRY, "1")
            .current_dir(self.workdir.path());
        SubExec::output_stdout(command)
    }

    /// Compile a move script
    pub fn compile_script(&self, path_src: &Path, path_out: &Path) -> Result<()> {
        let named_address_pairs: Vec<_> = self
            .named_accounts
            .iter()
            .map(|(k, v)| format!("{}={}", k, v.address().to_standard_string()))
            .collect();

        // command: basics
        let mut command = Command::new(APTOS_BIN.as_path());
        command.args(["move", "compile-script"]);
        // command: named addresses
        command
            .arg("--named-addresses")
            .arg(named_address_pairs.join(","));
        self.config.language.derive_cli_options(&mut command);
        // command: input and output
        command.arg("--package-dir").arg(path_src);
        command.arg("--output-file").arg(path_out);
        // command: misc
        command.arg("--skip-fetch-latest-git-deps");
        command
            .env(ENV_APTOS_DISABLE_TELEMETRY, "1")
            .current_dir(self.workdir.path());

        // invoke
        if !SubExec::invoke(command)? {
            bail!(
                "failed to compile script {}",
                path_src.to_str().unwrap_or("<non-ascii-path>")
            );
        }
        Ok(())
    }

    /// Tear down the simulator
    pub fn destroy(self) -> Result<()> {
        let Self {
            config: _,
            workdir,
            executor,
            named_accounts: _,
            address_lookup: _,
            resource_signers: _,
            modules: _,
            scripts: _,
        } = self;

        executor.interrupt()?;
        workdir.close()?;

        // done with the destruction
        info!("local testnet is shutdown");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{fund_amount_for_num_txns, DEFAULT_GAS_UNIT_PRICE, DEFAULT_MAX_GAS_AMOUNT};

    #[test]
    fn test_fund_amount_for_num_txns_matches_maximum_gas_budget() {
        assert_eq!(
            fund_amount_for_num_txns(3).unwrap(),
            3 * u64::from(DEFAULT_GAS_UNIT_PRICE) * u64::from(DEFAULT_MAX_GAS_AMOUNT)
        );
    }

    #[test]
    fn test_fund_amount_for_num_txns_detects_overflow() {
        assert!(fund_amount_for_num_txns(u64::MAX).is_err());
    }
}
