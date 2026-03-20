// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    config::{BaseState, Config},
    delta::{load_delta, save_delta},
    txn_output::{save_events, save_write_set},
};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_gas_profiling::GasProfiler;
use aptos_resource_viewer::{AnnotatedMoveValue, AptosValueAnnotator};
use aptos_rest_client::{AptosBaseUrl, Client};
use aptos_transaction_simulation::{
    DeltaStateStore, EitherStateView, EmptyStateView, SimulationStateStore, GENESIS_CHANGE_SET_HEAD,
};
use aptos_types::{
    account_address::{create_derived_object_address, AccountAddress},
    account_config::events::new_block::BlockResource,
    block_metadata::BlockMetadata,
    fee_statement::FeeStatement,
    on_chain_config::{ConfigurationResource, CurrentTimeMicroseconds, ValidatorSet},
    randomness::PerBlockRandomness,
    state_store::{state_key::StateKey, TStateView},
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, AuxiliaryInfo,
        SignedTransaction, Transaction, TransactionExecutable, TransactionOutput,
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
    NewBlock {
        old_timestamp_usecs: u64,
        new_timestamp_usecs: u64,
        old_epoch: u64,
        new_epoch: u64,
    },
}

/// Specifies the timestamp for a new block.
#[derive(Debug, Clone, Copy, Default)]
pub enum BlockTimestamp {
    /// Use the current on-chain timestamp plus 1 microsecond.
    #[default]
    Default,
    /// Use an absolute timestamp in microseconds.
    Absolute(u64),
    /// Advance the current on-chain timestamp by the given number of microseconds.
    Offset(u64),
}

/// Information about the result of executing a new block.
#[derive(Debug, Serialize, Deserialize)]
pub struct NewBlockResult {
    /// The new block timestamp in microseconds.
    pub new_timestamp_usecs: u64,
    /// The epoch before the block.
    pub old_epoch: u64,
    /// The epoch after the block. May differ from `old_epoch` if the block
    /// triggered a reconfiguration.
    pub new_epoch: u64,
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

        // Patch a synthetic randomness seed so transactions using on-chain randomness can
        // be simulated. On a real network the seed is derived from validator consensus,
        // which we can't reproduce locally. See also the re-patch in
        // `execute_block_metadata_transaction`.
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

    /// Injects a synthetic randomness seed into the state store.
    ///
    /// Called at session init and after each block metadata transaction. Without a valid
    /// seed, transactions that use on-chain randomness APIs would abort. On a real network
    /// the seed is derived from validator consensus, which we can't reproduce locally, so
    /// randomness-dependent behavior will always differ from production.
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

    /// Completes an operation by incrementing the op counter and persisting session state.
    ///
    /// If `save_state` is true, the state delta is also saved. This should be set for
    /// operations that modify state (e.g., fund, execute, new_block), but not for read-only
    /// operations (e.g., view).
    fn finish_op(&mut self, save_state: bool) -> Result<()> {
        self.config.ops += 1;
        self.config.save_to_file(&self.path.join("config.json"))?;
        if save_state {
            save_delta(&self.path.join("delta.json"), &self.state_store.delta())?;
        }
        Ok(())
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

        self.finish_op(true)?;

        Ok(())
    }

    /// Executes a new block at the given timestamp.
    ///
    /// This creates and executes a `BlockMetadata` transaction through the VM, triggering
    /// the full block prologue in Move, which:
    /// - Updates `CurrentTimeMicroseconds` to `timestamp_usecs`
    /// - Emits a `NewBlockEvent`
    /// - May trigger an epoch change if enough time has passed since the last reconfiguration
    /// - Updates staking performance statistics
    ///
    /// The resulting timestamp must be strictly greater than the current on-chain timestamp.
    pub fn new_block(&mut self, timestamp: BlockTimestamp) -> Result<NewBlockResult> {
        let config_resource: ConfigurationResource = self.state_store.get_on_chain_config()?;
        let old_epoch = config_resource.epoch();

        let current_timestamp: CurrentTimeMicroseconds = self.state_store.get_on_chain_config()?;
        let old_timestamp_usecs = current_timestamp.microseconds;

        let new_timestamp_usecs = match timestamp {
            BlockTimestamp::Default => old_timestamp_usecs.checked_add(1).ok_or_else(|| {
                anyhow::anyhow!("timestamp overflow: current timestamp is u64::MAX")
            })?,
            BlockTimestamp::Absolute(ts) => {
                if ts <= old_timestamp_usecs {
                    anyhow::bail!(
                        "timestamp must be strictly greater than the current on-chain \
                         timestamp ({old_timestamp_usecs}), got {ts}"
                    );
                }
                ts
            },
            BlockTimestamp::Offset(delta) => {
                if delta == 0 {
                    anyhow::bail!(
                        "offset must be greater than zero to ensure the new timestamp is \
                         strictly greater than the current on-chain timestamp \
                         ({old_timestamp_usecs})"
                    );
                }
                old_timestamp_usecs.checked_add(delta).ok_or_else(|| {
                    anyhow::anyhow!(
                        "timestamp overflow: {old_timestamp_usecs} + {delta} exceeds u64::MAX"
                    )
                })?
            },
        };

        self.run_new_block(new_timestamp_usecs, old_timestamp_usecs, old_epoch)
    }

    /// Advances the simulation to the next epoch.
    ///
    /// This calculates the minimum timestamp needed to cross the epoch boundary and
    /// executes a new block at that timestamp. The block prologue detects that enough
    /// time has passed since the last reconfiguration and calls `reconfiguration::reconfigure()`.
    ///
    /// The epoch interval is read from the on-chain `BlockResource`, so this works
    /// correctly even if the epoch interval has been modified.
    pub fn advance_epoch(&mut self) -> Result<NewBlockResult> {
        let config_resource: ConfigurationResource = self.state_store.get_on_chain_config()?;
        let old_epoch = config_resource.epoch();
        let last_reconfig_time = config_resource.last_reconfiguration_time_micros();

        let current_timestamp: CurrentTimeMicroseconds = self.state_store.get_on_chain_config()?;
        let old_timestamp_usecs = current_timestamp.microseconds;

        let block_resource: BlockResource = self
            .state_store
            .get_resource(AccountAddress::ONE)?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "BlockResource not found at 0x1 -- is the chain properly initialized?"
                )
            })?;
        let epoch_interval_usecs = block_resource.epoch_interval();

        // The block prologue triggers reconfiguration when:
        //   timestamp - last_reconfiguration_time >= epoch_interval
        //
        // The timestamp must also be strictly greater than the current one.
        let epoch_boundary = last_reconfig_time
            .checked_add(epoch_interval_usecs)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "timestamp overflow: last_reconfig_time ({last_reconfig_time}) + \
                 epoch_interval ({epoch_interval_usecs}) exceeds u64::MAX"
                )
            })?;
        let min_next = old_timestamp_usecs
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("timestamp overflow: current timestamp is u64::MAX"))?;
        let new_timestamp_usecs = epoch_boundary.max(min_next);

        self.run_new_block(new_timestamp_usecs, old_timestamp_usecs, old_epoch)
    }

    /// Shared implementation for [`new_block`](Self::new_block) and
    /// [`advance_epoch`](Self::advance_epoch).
    fn run_new_block(
        &mut self,
        new_timestamp_usecs: u64,
        old_timestamp_usecs: u64,
        old_epoch: u64,
    ) -> Result<NewBlockResult> {
        let txn_output = self.execute_block_metadata_transaction(old_epoch, new_timestamp_usecs)?;

        let new_epoch = self
            .state_store
            .get_on_chain_config::<ConfigurationResource>()?
            .epoch();

        // Save summary and artifacts.
        let output_path = self.path.join(format!("[{}] new block", self.config.ops));
        std::fs::create_dir_all(&output_path)?;

        let summary = Summary::NewBlock {
            old_timestamp_usecs,
            new_timestamp_usecs,
            old_epoch,
            new_epoch,
        };
        std::fs::write(
            output_path.join("summary.json"),
            serde_json::to_string_pretty(&summary)?,
        )?;
        save_events(
            &output_path.join("events.json"),
            &self.state_store,
            txn_output.events(),
        )?;
        save_write_set(
            &self.state_store,
            &output_path.join("write_set.json"),
            txn_output.write_set(),
        )?;

        self.finish_op(true)?;

        Ok(NewBlockResult {
            new_timestamp_usecs,
            old_epoch,
            new_epoch,
        })
    }

    /// Executes a `BlockMetadata` transaction through the VM.
    ///
    /// This is the low-level helper used by [`new_block`](Self::new_block). It handles
    /// constructing the `BlockMetadata`, running it through the VM, applying the write set,
    /// and re-patching the randomness seed.
    ///
    /// We use the legacy `Transaction::BlockMetadata` rather than the newer
    /// `Transaction::BlockMetadataExt` used by production validators with
    /// randomness enabled. The key difference is that the legacy path calls
    /// `reconfiguration::reconfigure()` (immediate) while the ext path calls
    /// `reconfiguration_with_dkg::try_start()` (multi-round DKG that requires validator
    /// participation across multiple blocks). Immediate reconfiguration is more practical
    /// for simulation since epoch changes complete in a single block. This is the same
    /// approach used by `FakeExecutor` in the e2e test harness.
    fn execute_block_metadata_transaction(
        &mut self,
        epoch: u64,
        timestamp_usecs: u64,
    ) -> Result<TransactionOutput> {
        // The block prologue requires a non-zero proposer when updating the timestamp.
        let validator_set: ValidatorSet = self.state_store.get_on_chain_config()?;
        let proposer = *validator_set
            .payload()
            .next()
            .ok_or_else(|| {
                anyhow::anyhow!("validator set is empty -- cannot create block metadata")
            })?
            .account_address();

        let num_validators = validator_set.num_validators();
        let previous_block_votes_bitvec = vec![0u8; num_validators.div_ceil(8)];

        let block_metadata = BlockMetadata::new(
            HashValue::zero(),
            epoch,
            0, // round (not tracked in simulation)
            proposer,
            previous_block_votes_bitvec,
            vec![], // no failed proposers
            timestamp_usecs,
        );

        let env = AptosEnvironment::new(&self.state_store);
        let vm = AptosVM::new(&env);
        let log_context = AdapterLogSchema::new(self.state_store.id(), 0);
        let resolver = self.state_store.as_move_resolver();
        let code_storage = self.state_store.as_aptos_code_storage(&env);

        let txn = SignatureVerifiedTransaction::Valid(Transaction::BlockMetadata(block_metadata));
        let (vm_status, vm_output) = vm
            .execute_single_transaction(
                &txn,
                &resolver,
                &code_storage,
                &log_context,
                &AuxiliaryInfo::new_timestamp_not_yet_assigned(0),
            )
            .map_err(|e| anyhow::anyhow!("block prologue execution failed: {:?}", e))?;

        if vm_status != VMStatus::Executed {
            anyhow::bail!(
                "block prologue execution returned non-success status: {:?}",
                vm_status
            );
        }

        let txn_output = vm_output.try_materialize_into_transaction_output(&resolver)?;
        self.state_store.apply_write_set(txn_output.write_set())?;

        // Re-patch the randomness seed. The block prologue clears it because our
        // BlockMetadata doesn't carry a real seed (on a real network this comes from
        // validator consensus). Without re-patching, subsequent transactions that use
        // on-chain randomness would abort.
        Self::patch_randomness_seed(&self.state_store)?;

        Ok(txn_output)
    }

    /// Executes a transaction and updates the session state.
    ///
    /// After execution, selected parts of the transaction output get saved to a dedicated directory for inspection:
    /// - Write set changes
    /// - Emitted events
    ///
    /// If `profile_gas` is `true`, the transaction is executed with the gas profiler enabled.
    /// A `gas-report` directory is generated under the transaction output directory containing
    /// an HTML report with flamegraphs and detailed gas breakdowns.
    pub fn execute_transaction(
        &mut self,
        txn: SignedTransaction,
        profile_gas: bool,
    ) -> Result<(VMStatus, TransactionOutput)> {
        let env = AptosEnvironment::new(&self.state_store);
        let vm = AptosVM::new(&env);
        let log_context = AdapterLogSchema::new(self.state_store.id(), 0);

        let resolver = self.state_store.as_move_resolver();
        let code_storage = self.state_store.as_aptos_code_storage(&env);

        // Execute the transaction, optionally with gas profiling.
        let (vm_status, txn_output, gas_log) = if profile_gas {
            let (vm_status, vm_output, gas_profiler) = vm
                .execute_user_transaction_with_modified_gas_meter(
                    &resolver,
                    &code_storage,
                    &txn,
                    &log_context,
                    |gas_meter| match txn.payload() {
                        TransactionPayload::EntryFunction(entry_function) => {
                            GasProfiler::new_function(
                                gas_meter,
                                entry_function.module().clone(),
                                entry_function.function().to_owned(),
                                entry_function.ty_args().to_vec(),
                            )
                        },
                        _ => GasProfiler::new_script(gas_meter),
                    },
                    &AuxiliaryInfo::new_timestamp_not_yet_assigned(0),
                )
                .map_err(|e| anyhow::anyhow!("transaction execution failed: {:?}", e))?;
            let txn_output = vm_output.try_materialize_into_transaction_output(&resolver)?;
            (vm_status, txn_output, Some(gas_profiler.finish()))
        } else {
            let (vm_status, vm_output) = vm.execute_user_transaction(
                &resolver,
                &code_storage,
                &txn,
                &log_context,
                &AuxiliaryInfo::new_timestamp_not_yet_assigned(0),
            );
            let txn_output = vm_output.try_materialize_into_transaction_output(&resolver)?;
            (vm_status, txn_output, None)
        };

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
                // TODO(ibalajiarun): How do you simulate encrypted transaction?
                TransactionExecutable::Empty | TransactionExecutable::Encrypted => {
                    unimplemented!(
                        "empty/encrypted executable -- unclear how this should be handled"
                    )
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

        let events_path = output_path.join("events.json");
        save_events(&events_path, &self.state_store, txn_output.events())?;

        let write_set_path = output_path.join("write_set.json");
        save_write_set(&self.state_store, &write_set_path, txn_output.write_set())?;

        // Generate gas profiling report if enabled.
        if let Some(gas_log) = gas_log {
            gas_log.generate_html_report(output_path.join("gas-report"), name)?;
        }

        self.finish_op(true)?;

        Ok((vm_status, txn_output))
    }

    /// Executes a view function and returns the output values.
    ///
    /// If `profile_gas` is `true`, the view function is executed with the gas profiler enabled.
    /// A `gas-report` directory is generated under the output directory containing
    /// an HTML report with flamegraphs and detailed gas breakdowns.
    pub fn execute_view_function(
        &mut self,
        module_id: ModuleId,
        function_name: Identifier,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        profile_gas: bool,
    ) -> Result<Vec<serde_json::Value>> {
        let (output, gas_log) = if profile_gas {
            let (output, gas_profiler) = AptosVM::execute_view_function_with_modified_gas_meter(
                &self.state_store,
                module_id.clone(),
                function_name.clone(),
                ty_args.clone(),
                args,
                u64::MAX,
                |gas_meter| {
                    GasProfiler::new_function(
                        gas_meter,
                        module_id.clone(),
                        function_name.clone(),
                        ty_args.clone(),
                    )
                },
            );
            (output, gas_profiler.map(|p| p.finish()))
        } else {
            let output = AptosVM::execute_view_function(
                &self.state_store,
                module_id.clone(),
                function_name.clone(),
                ty_args.clone(),
                args,
                u64::MAX,
            );
            (output, None)
        };

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

        let name = format!("{}::{}", format_module_id(&module_id), function_name);

        let output_path = self
            .path
            .join(format!("[{}] view {}", self.config.ops, name,));
        std::fs::create_dir_all(&output_path)?;

        let summary_path = output_path.join("summary.json");
        std::fs::write(summary_path, serde_json::to_string_pretty(&summary)?)?;

        // Generate gas profiling report if enabled.
        if let Some(gas_log) = gas_log {
            gas_log.generate_html_report(output_path.join("gas-report"), name)?;
        }

        self.finish_op(false)?;

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

        self.finish_op(false)?;

        Ok(json_val)
    }

    /// Views a Move resource group.
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

        self.finish_op(false)?;

        Ok(json_val)
    }
}
