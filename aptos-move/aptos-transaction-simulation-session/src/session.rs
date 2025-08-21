// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{BaseState, Config},
    delta::{load_delta, save_delta},
    txn_output::{save_events, save_write_set},
};
use anyhow::Result;
use aptos_resource_viewer::AptosValueAnnotator;
use aptos_rest_client::{AptosBaseUrl, Client};
use aptos_transaction_simulation::{
    DeltaStateStore, EitherStateView, EmptyStateView, SimulationStateStore, GENESIS_CHANGE_SET_HEAD,
};
use aptos_types::{
    account_address::AccountAddress,
    fee_statement::FeeStatement,
    transaction::{
        AuxiliaryInfo, PersistedAuxiliaryInfo, SignedTransaction, TransactionExecutable,
        TransactionOutput, TransactionPayload, TransactionPayloadInner, TransactionStatus,
    },
    vm_status::VMStatus,
};
use aptos_validator_interface::{DebuggerStateView, RestDebuggerInterface};
use aptos_vm::{data_cache::AsMoveResolver, AptosVM};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{module_and_script_storage::AsAptosCodeStorage, resolver::StateStorageView};
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use url::Url;

type SessionStateStore = DeltaStateStore<EitherStateView<EmptyStateView, DebuggerStateView>>;

/// Formats a module ID for display by adjusting the address for better readability.
fn format_module_id(module_id: &ModuleId) -> String {
    let address = module_id.address();
    let name = module_id.name();

    // Format address: add 0x prefix, trim leading zeros, limit to 4 digits
    let address_str = format!("{:x}", address);
    let trimmed = address_str.trim_start_matches('0');
    let display_address = if trimmed.is_empty() {
        "0".to_string()
    } else if trimmed.len() > 4 {
        format!("{}...", &trimmed[..4])
    } else {
        trimmed.to_string()
    };

    format!("0x{}::{}", display_address, name)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ViewResult {
    Success(Vec<serde_json::Value>),
    Error(String),
}

/// A summary of a completed session operation.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Summary {
    FundFungible {
        account: AccountAddress,
        amount: u64,
        before: u64,
        after: u64,
    },
    ExecuteTransaction {
        status: TransactionStatus,
        gas_used: u64,
        fee_statement: Option<FeeStatement>,
    },
    View {
        result: ViewResult,
        gas_used: u64,
    },
}

/// A session for simulating transactions, with data being persisted to a directory, allowing the session
/// to be restored or continued in the future.
///
/// For each session operation, additional info gets saved to allow for easy inspection.
pub struct Session {
    config: Config,
    path: PathBuf,
    state_store: SessionStateStore,
}

impl Session {
    /// Returns a reference to the underlying state store.
    pub fn state_store(&self) -> &impl SimulationStateStore {
        &self.state_store
    }

    /// Creates a new session using an empty base state, then applies the Aptos genesis
    /// change set on top of it.
    ///
    /// Useful for local simulations and integration tests where a clean genesis state is required.
    pub fn init(session_path: impl AsRef<Path>) -> Result<Self> {
        let session_path = session_path.as_ref().to_path_buf();

        std::fs::create_dir_all(&session_path)?;

        if session_path.read_dir()?.next().is_some() {
            anyhow::bail!(
                "Cannot initialize new session at {} -- directory is not empty.",
                session_path.display()
            );
        }

        // Write config with empty base state
        let config = Config::new();
        let config_path = session_path.join("config.json");
        config.save_to_file(&config_path)?;

        // Initialize state store -- need to populate with head genesis
        // TODO: allow caller to specify genesis
        let state_store = DeltaStateStore::new_with_base(EitherStateView::Left(EmptyStateView));
        state_store.apply_write_set(GENESIS_CHANGE_SET_HEAD.write_set())?;

        // Save delta to file
        let delta_path = session_path.join("delta.json");
        save_delta(&delta_path, &state_store.delta())?;

        Ok(Self {
            config,
            path: session_path,
            state_store,
        })
    }

    /// Initializes a new session by forking from a remote network state. Data will be fetched
    /// from the remote network on-demand.
    ///
    /// It is strongly recommended that the caller provides an API key to avoid rate limiting.
    pub fn init_with_remote_state(
        session_path: impl AsRef<Path>,
        node_url: Url,
        network_version: u64,
        api_key: Option<String>,
    ) -> Result<Self> {
        let session_path = session_path.as_ref().to_path_buf();

        std::fs::create_dir_all(&session_path)?;

        if session_path.read_dir()?.next().is_some() {
            anyhow::bail!(
                "Cannot initialize new session at {} -- directory is not empty.",
                session_path.display()
            );
        }

        let config = Config::with_remote(node_url.clone(), network_version, api_key.clone());
        let config_path = session_path.join("config.json");
        config.save_to_file(&config_path)?;

        let delta_path = session_path.join("delta.json");
        save_delta(&delta_path, &HashMap::new())?;

        let mut builder = Client::builder(AptosBaseUrl::Custom(node_url));
        if let Some(api_key) = api_key {
            builder = builder.api_key(&api_key)?;
        }
        let client = builder.build();

        let state_store =
            DeltaStateStore::new_with_base(EitherStateView::Right(DebuggerStateView::new(
                Arc::new(RestDebuggerInterface::new(client)),
                network_version,
            )));

        Ok(Self {
            config,
            path: session_path,
            state_store,
        })
    }

    /// Loads a previously stored session from disk.
    pub fn load(session_path: impl AsRef<Path>) -> Result<Self> {
        let session_path = session_path.as_ref().to_path_buf();
        let config = Config::load_from_file(&session_path.join("config.json"))?;

        let base = match &config.base {
            BaseState::Empty => EitherStateView::Left(EmptyStateView),
            BaseState::Remote {
                node_url,
                network_version,
                api_key,
            } => {
                let mut builder = Client::builder(AptosBaseUrl::Custom(node_url.clone()));
                if let Some(api_key) = api_key {
                    builder = builder.api_key(api_key)?;
                }
                let client = builder.build();

                let debugger = DebuggerStateView::new(
                    Arc::new(RestDebuggerInterface::new(client)),
                    *network_version,
                );
                EitherStateView::Right(debugger)
            },
        };

        let delta = load_delta(&session_path.join("delta.json"))?;
        let state_store = DeltaStateStore::new_with_base_and_delta(base, delta);

        Ok(Self {
            config,
            path: session_path,
            state_store,
        })
    }

    /// Funds an account with APT.
    ///
    /// This counts as a session operation but is not a real transaction, as it modifies the
    /// storage state directly.
    ///
    /// This can be useful for testing -- for example, to fund an account before using it to
    /// send its first transaction.
    pub fn fund_account(&mut self, account: AccountAddress, amount: u64) -> Result<()> {
        let (before, after) = self.state_store.fund_apt_fungible_store(account, amount)?;

        let summary = Summary::FundFungible {
            account,
            amount,
            before,
            after,
        };
        let summary_path = self
            .path
            .join(format!("[{}] fund (fungible)", self.config.ops))
            .join("summary.json");
        std::fs::create_dir_all(summary_path.parent().unwrap())?;
        std::fs::write(summary_path, serde_json::to_string_pretty(&summary)?)?;

        self.config.ops += 1;

        self.config.save_to_file(&self.path.join("config.json"))?;
        save_delta(&self.path.join("delta.json"), &self.state_store.delta())?;

        Ok(())
    }

    /// Executes a transaction and updates the session state.
    ///
    /// After execution, selected parts of the transaction output get saved to a dedicated directory for inspection:
    /// - Write set changes
    /// - Emitted events
    pub fn execute_transaction(
        &mut self,
        txn: SignedTransaction,
    ) -> Result<(VMStatus, TransactionOutput)> {
        let env = AptosEnvironment::new(&self.state_store);
        let vm = AptosVM::new(&env, &self.state_store);
        let log_context = AdapterLogSchema::new(self.state_store.id(), 0);

        let resolver = self.state_store.as_move_resolver();
        let code_storage = self.state_store.as_aptos_code_storage(&env);

        let (vm_status, vm_output) = vm.execute_user_transaction(
            &resolver,
            &code_storage,
            &txn,
            &log_context,
            &AuxiliaryInfo::new(
                PersistedAuxiliaryInfo::V1 {
                    transaction_index: 0,
                },
                None,
            ),
        );
        let txn_output = vm_output.try_materialize_into_transaction_output(&resolver)?;

        self.state_store.apply_write_set(txn_output.write_set())?;

        fn name_from_executable(executable: &TransactionExecutable) -> String {
            match executable {
                TransactionExecutable::Script(_script) => "script".to_string(),
                TransactionExecutable::EntryFunction(entry_function) => {
                    format!(
                        "{}::{}",
                        format_module_id(entry_function.module()),
                        entry_function.function()
                    )
                },
                TransactionExecutable::Empty => {
                    unimplemented!("empty executable -- unclear how this should be handled")
                },
            }
        }
        let name = match &txn.payload() {
            TransactionPayload::EntryFunction(entry_function) => {
                format!(
                    "{}::{}",
                    format_module_id(entry_function.module()),
                    entry_function.function()
                )
            },
            TransactionPayload::Script(_script) => "script".to_string(),
            TransactionPayload::Multisig(multi_sig) => {
                name_from_executable(&multi_sig.as_transaction_executable())
            },
            TransactionPayload::Payload(TransactionPayloadInner::V1 { executable, .. }) => {
                name_from_executable(executable)
            },
            TransactionPayload::ModuleBundle(_) => unreachable!(),
        };

        let output_path = self
            .path
            .join(format!("[{}] execute {}", self.config.ops, name));
        std::fs::create_dir_all(&output_path)?;

        let summary = Summary::ExecuteTransaction {
            status: txn_output.status().clone(),
            gas_used: txn_output.gas_used(),
            fee_statement: txn_output.try_extract_fee_statement()?,
        };
        let summary_path = output_path.join("summary.json");
        std::fs::write(summary_path, serde_json::to_string_pretty(&summary)?)?;

        // Dump events to file
        let events_path = output_path.join("events.json");
        save_events(&events_path, &self.state_store, txn_output.events())?;

        let write_set_path = output_path.join("write_set.json");
        save_write_set(&self.state_store, &write_set_path, txn_output.write_set())?;

        self.config.ops += 1;
        self.config.save_to_file(&self.path.join("config.json"))?;
        save_delta(&self.path.join("delta.json"), &self.state_store.delta())?;

        Ok((vm_status, txn_output))
    }

    /// Executes a view function and returns the output values.
    pub fn execute_view_function(
        &mut self,
        module_id: ModuleId,
        function_name: Identifier,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    ) -> Result<Vec<serde_json::Value>> {
        let output = AptosVM::execute_view_function(
            &self.state_store,
            module_id.clone(),
            function_name.clone(),
            ty_args.clone(),
            args,
            u64::MAX,
        );

        let (summary, res) = match output.values {
            Ok(values) => {
                let annotator = AptosValueAnnotator::new(&self.state_store);

                let returns = annotator.view_function_returns(
                    &module_id,
                    &function_name,
                    &ty_args,
                    &values,
                )?;

                let mut vals = Vec::new();
                for ret_val in returns {
                    vals.push(aptos_api_types::MoveValue::try_from(ret_val)?.json()?);
                }

                let summary = Summary::View {
                    result: ViewResult::Success(vals.clone()),
                    gas_used: output.gas_used,
                };

                (summary, Ok(vals))
            },
            Err(e) => {
                let summary = Summary::View {
                    result: ViewResult::Error(e.to_string()),
                    gas_used: output.gas_used,
                };

                (summary, Err(anyhow::anyhow!(e)))
            },
        };

        let summary_path = self
            .path
            .join(format!(
                "[{}] view {}::{}",
                self.config.ops,
                format_module_id(&module_id),
                function_name
            ))
            .join("summary.json");
        std::fs::create_dir_all(summary_path.parent().unwrap())?;
        std::fs::write(summary_path, serde_json::to_string_pretty(&summary)?)?;

        self.config.ops += 1;
        self.config.save_to_file(&self.path.join("config.json"))?;

        res
    }

    // TODO: view resource
}

#[test]
fn test_init_then_load_session_local() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let session_path = temp_dir.path();

    let session = Session::init(session_path)?;
    let session_loaded = Session::load(session_path)?;

    assert_eq!(session.config, session_loaded.config);
    assert_eq!(
        session.state_store.delta(),
        session_loaded.state_store.delta()
    );

    Ok(())
}

#[tokio::test]
async fn test_init_then_load_session_remote() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let session_path = temp_dir.path();

    let session = Session::init_with_remote_state(
        session_path,
        Url::parse("https://mainnet.aptoslabs.com")?,
        12345,
        Some("my_api_key_12345".to_string()),
    )?;
    let session_loaded = Session::load(session_path)?;

    assert_eq!(session.config, session_loaded.config);
    assert_eq!(
        session.state_store.delta(),
        session_loaded.state_store.delta()
    );

    Ok(())
}
