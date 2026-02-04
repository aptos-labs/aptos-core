// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    config::{BaseState, Config},
    delta::{load_delta, save_delta},
    txn_output::{save_events, save_write_set},
};
use anyhow::Result;
use aptos_resource_viewer::{AnnotatedMoveValue, AptosValueAnnotator};
use aptos_rest_client::{AptosBaseUrl, Client};
use aptos_transaction_simulation::{
    DeltaStateStore, EitherStateView, EmptyStateView, SimulationStateStore, GENESIS_CHANGE_SET_HEAD,
};
use aptos_types::{
    account_address::{create_derived_object_address, AccountAddress},
    fee_statement::FeeStatement,
    randomness::PerBlockRandomness,
    state_store::{state_key::StateKey, TStateView},
    transaction::{
        AuxiliaryInfo, SignedTransaction, TransactionExecutable, TransactionOutput,
        TransactionPayload, TransactionPayloadInner, TransactionStatus,
    },
    vm_status::VMStatus,
};
use aptos_validator_interface::{DebuggerStateView, RestDebuggerInterface};
use aptos_vm::{data_cache::AsMoveResolver, AptosVM};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::module_and_script_storage::AsAptosCodeStorage;
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use url::Url;

type SessionStateStore = DeltaStateStore<EitherStateView<EmptyStateView, DebuggerStateView>>;

/// Formats an account address for display.
/// Truncates the address if it's more than 4 digits.
fn format_address(address: &AccountAddress) -> String {
    let address_str = address.to_hex_literal();
    if address_str.len() > 6 {
        format!("{}...", &address_str[..6])
    } else {
        address_str
    }
}

/// Formats a module ID for display by adjusting the address for better readability.
fn format_module_id(module_id: &ModuleId) -> String {
    let address = module_id.address();
    let name = module_id.name();
    format!("{}::{}", format_address(address), name)
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
    ViewResource {
        resource_type: String,
        resource_value: Option<serde_json::Value>,
    },
    ViewResourceGroup {
        group_type: String,
        group_value: Option<serde_json::Value>,
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
    pub fn state_store(&self) -> &(impl SimulationStateStore + use<>) {
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

        // Patch randomness seed so that transactions using on-chain randomness can be simulated.
        // In normal block execution, the block prologue sets the seed, but since we're simulating
        // transactions directly without a block prologue, we need to provide a synthetic seed.
        Self::patch_randomness_seed(&state_store)?;

        // Save delta to file
        let delta_path = session_path.join("delta.json");
        save_delta(&delta_path, &state_store.delta())?;

        Ok(Self {
            config,
            path: session_path,
            state_store,
        })
    }

    /// Patches the randomness seed in the state store.
    ///
    /// This is needed because in simulation mode, there's no block prologue to set the
    /// `PerBlockRandomness` seed. Without a valid seed, transactions that use on-chain
    /// randomness APIs will fail when trying to access the seed.
    fn patch_randomness_seed(state_store: &impl SimulationStateStore) -> Result<()> {
        let mut seed = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut seed);

        state_store.set_on_chain_config(&PerBlockRandomness {
            epoch: 0,
            round: 0,
            seed: Some(seed),
        })
    }

    /// Initializes a new session by forking from a remote network state. Data will be fetched
    /// from the remote network on-demand.
    ///
    /// It is strongly recommended that the caller provides an API key to avoid rate limiting.
    ///
    /// Note: Unlike local mode, this does NOT patch the randomness seed. If the remote network
    /// hasn't enabled randomness or the seed is not set, transactions using on-chain randomness
    /// will fail - which accurately reflects what would happen on the actual network.
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
        save_delta(&delta_path, &Default::default())?;

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
        let vm = AptosVM::new(&env);
        let log_context = AdapterLogSchema::new(self.state_store.id(), 0);

        let resolver = self.state_store.as_move_resolver();
        let code_storage = self.state_store.as_aptos_code_storage(&env);

        let (vm_status, vm_output) = vm.execute_user_transaction(
            &resolver,
            &code_storage,
            &txn,
            &log_context,
            &AuxiliaryInfo::new_timestamp_not_yet_assigned(0),
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
            TransactionPayload::EncryptedPayload(_) => "encrypted".to_string(),
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

    /// Views a Move resource.
    pub fn view_resource(
        &mut self,
        account_addr: AccountAddress,
        resource_tag: &StructTag,
    ) -> Result<Option<serde_json::Value>> {
        let state_key = StateKey::resource(&account_addr, resource_tag)?;

        let json_val = match self.state_store.get_state_value_bytes(&state_key)? {
            Some(bytes) => {
                let annotator = AptosValueAnnotator::new(&self.state_store);
                let annotated =
                    AnnotatedMoveValue::Struct(annotator.view_resource(resource_tag, &bytes)?);
                Some(aptos_api_types::MoveValue::try_from(annotated)?.json()?)
            },
            None => None,
        };

        let summary = Summary::ViewResource {
            resource_type: resource_tag.to_canonical_string(),
            resource_value: json_val.clone(),
        };

        let summary_path = self
            .path
            .join(format!(
                "[{}] view resource {}::{}::{}::{}",
                self.config.ops,
                format_address(&account_addr),
                format_address(&resource_tag.address),
                resource_tag.module,
                resource_tag.name,
            ))
            .join("summary.json");
        std::fs::create_dir_all(summary_path.parent().unwrap())?;
        std::fs::write(summary_path, serde_json::to_string_pretty(&summary)?)?;

        self.config.ops += 1;
        self.config.save_to_file(&self.path.join("config.json"))?;

        Ok(json_val)
    }

    pub fn view_resource_group(
        &mut self,
        account_addr: AccountAddress,
        resource_group_tag: &StructTag,
        derived_object_address: Option<AccountAddress>,
    ) -> Result<Option<serde_json::Value>> {
        let account_addr = match derived_object_address {
            Some(addr) => create_derived_object_address(account_addr, addr),
            None => account_addr,
        };

        let state_key = StateKey::resource_group(&account_addr, resource_group_tag);

        let json_val = match self.state_store.get_state_value_bytes(&state_key)? {
            Some(bytes) => {
                let group: BTreeMap<StructTag, Vec<u8>> = bcs::from_bytes(&bytes)?;

                let annotator = AptosValueAnnotator::new(&self.state_store);

                let mut group_deserialized = BTreeMap::new();
                for (resource_tag, bytes) in group {
                    let annotated =
                        AnnotatedMoveValue::Struct(annotator.view_resource(&resource_tag, &bytes)?);
                    group_deserialized.insert(
                        resource_tag.to_canonical_string(),
                        aptos_api_types::MoveValue::try_from(annotated)?.json()?,
                    );
                }

                Some(json!(group_deserialized))
            },
            None => None,
        };

        let summary = Summary::ViewResourceGroup {
            group_type: resource_group_tag.to_canonical_string(),
            group_value: json_val.clone(),
        };

        let summary_path = self
            .path
            .join(format!(
                "[{}] view resource group {}::{}::{}::{}",
                self.config.ops,
                format_address(&account_addr),
                format_address(&resource_group_tag.address),
                resource_group_tag.module,
                resource_group_tag.name,
            ))
            .join("summary.json");
        std::fs::create_dir_all(summary_path.parent().unwrap())?;
        std::fs::write(summary_path, serde_json::to_string_pretty(&summary)?)?;

        self.config.ops += 1;
        self.config.save_to_file(&self.path.join("config.json"))?;

        Ok(json_val)
    }
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

#[test]
fn test_local_session_has_randomness_seed() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let session_path = temp_dir.path();

    let session = Session::init(session_path)?;

    // Verify that the PerBlockRandomness seed is set (not None)
    let randomness_config = session
        .state_store
        .get_on_chain_config::<PerBlockRandomness>()?;

    assert!(
        randomness_config.seed.is_some(),
        "Local session should have randomness seed patched"
    );
    assert_eq!(
        randomness_config.seed.as_ref().unwrap().len(),
        32,
        "Randomness seed should be 32 bytes"
    );
    Ok(())
}
