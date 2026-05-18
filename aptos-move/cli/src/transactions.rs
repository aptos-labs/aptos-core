// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Transaction options and local simulation support for the `aptos move simulate` command.

use crate::{local_simulation, MoveDebugger, MoveEnv};
// Re-export from aptos-cli-common to eliminate the duplicate definition.
pub use aptos_cli_common::ReplayProtectionType;
use aptos_cli_common::{
    format_txn_status, get_account_with_state, CliError, CliTypedResult, EncodingOptions,
    GasOptions, PrivateKeyInputOptions, ProfileOptions, PromptOptions, RestOptions,
    TransactionSummary, ACCEPTED_CLOCK_SKEW_US, US_IN_SECS,
};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519Signature},
    hash::CryptoHash,
    PrivateKey,
};
use aptos_resource_viewer::AptosValueAnnotator;
use aptos_rest_client::Client;
use aptos_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use aptos_types::{
    access_path::Path,
    account_address::AccountAddress,
    chain_id::ChainId,
    contract_event::ContractEvent,
    state_store::state_key::inner::StateKeyInner,
    transaction::{
        PersistedAuxiliaryInfo, ReplayProtector, SignedTransaction, TransactionOutput,
        TransactionPayload, TransactionStatus,
    },
    write_set::WriteSet,
};
use aptos_vm::data_cache::AsMoveResolver;
use aptos_vm_types::output::VMOutput;
use clap::Parser;
#[cfg(test)]
use move_core_types::value::MoveTypeLayout;
use move_core_types::vm_status::VMStatus;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

fn serialize_as_json<T: Serialize>(value: &T) -> CliTypedResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|err| CliError::UnexpectedError(err.to_string()))
}

#[cfg(test)]
fn local_events_to_json(
    state_view: &impl aptos_types::state_store::StateView,
    events: &[(ContractEvent, Option<MoveTypeLayout>)],
) -> CliTypedResult<serde_json::Value> {
    let events = events
        .iter()
        .map(|(event, _layout)| event.clone())
        .collect::<Vec<_>>();
    local_contract_events_to_json(state_view, &events)
}

fn local_contract_events_to_json(
    state_view: &impl aptos_types::state_store::StateView,
    events: &[ContractEvent],
) -> CliTypedResult<serde_json::Value> {
    let annotator = AptosValueAnnotator::new(state_view);
    let parsed_events = events
        .iter()
        .map(|event| {
            let data = annotator.view_value(event.type_tag(), event.event_data())?;
            let data = aptos_api_types::MoveValue::try_from(data)?.json()?;
            Ok::<_, anyhow::Error>(aptos_api_types::Event::from((event, data)))
        })
        .collect::<anyhow::Result<Vec<_>>>()
        .map_err(|err| CliError::UnexpectedError(err.to_string()))?;
    serialize_as_json(&parsed_events)
}

fn local_write_set_to_json(
    state_view: &impl aptos_types::state_store::StateView,
    write_set: &WriteSet,
) -> CliTypedResult<serde_json::Value> {
    let annotator = AptosValueAnnotator::new(state_view);
    let mut changes = Vec::with_capacity(write_set.write_op_iter().size_hint().0);

    for (state_key, op) in write_set.write_op_iter() {
        let state_key_hash = state_key.hash().to_hex_literal();
        match state_key.inner() {
            StateKeyInner::AccessPath(access_path) => match op.bytes() {
                None => match access_path.get_path() {
                    Path::Code(module_id) => {
                        // User-submitted transactions usually do not delete modules, but we keep
                        // this branch for output completeness and consistency with API conversion.
                        changes.push(aptos_api_types::WriteSetChange::DeleteModule(
                            aptos_api_types::DeleteModule {
                                address: access_path.address.into(),
                                state_key_hash,
                                module: module_id.into(),
                            },
                        ));
                    },
                    Path::Resource(typ) | Path::ResourceGroup(typ) => {
                        changes.push(aptos_api_types::WriteSetChange::DeleteResource(
                            aptos_api_types::DeleteResource {
                                address: access_path.address.into(),
                                state_key_hash,
                                resource: typ.into(),
                            },
                        ));
                    },
                },
                Some(bytes) => match access_path.get_path() {
                    Path::Code(_) => {
                        let data = aptos_api_types::MoveModuleBytecode::new(bytes.to_vec())
                            .try_parse_abi()
                            .map_err(|err| CliError::UnexpectedError(err.to_string()))?;
                        changes.push(aptos_api_types::WriteSetChange::WriteModule(
                            aptos_api_types::WriteModule {
                                address: access_path.address.into(),
                                state_key_hash,
                                data,
                            },
                        ));
                    },
                    Path::Resource(typ) => {
                        let data = annotator
                            .view_resource(&typ, bytes)
                            .and_then(aptos_api_types::MoveResource::try_from)
                            .map_err(|err| CliError::UnexpectedError(err.to_string()))?;
                        changes.push(aptos_api_types::WriteSetChange::WriteResource(
                            aptos_api_types::WriteResource {
                                address: access_path.address.into(),
                                state_key_hash,
                                data,
                            },
                        ));
                    },
                    Path::ResourceGroup(_) => {
                        let group: BTreeMap<move_core_types::language_storage::StructTag, Vec<u8>> =
                            bcs::from_bytes(bytes)
                                .map_err(|err| CliError::UnexpectedError(err.to_string()))?;
                        for (tag, value) in group {
                            let resource = annotator.view_resource(&tag, &value);
                            let resource = resource
                                .map_err(|err| CliError::UnexpectedError(err.to_string()))?;
                            let data = aptos_api_types::MoveResource::try_from(resource)
                                .map_err(|err| CliError::UnexpectedError(err.to_string()))?;
                            changes.push(aptos_api_types::WriteSetChange::WriteResource(
                                aptos_api_types::WriteResource {
                                    address: access_path.address.into(),
                                    state_key_hash: state_key_hash.clone(),
                                    data,
                                },
                            ));
                        }
                    },
                },
            },
            StateKeyInner::TableItem { handle, key } => {
                let handle = handle.0.to_vec().into();
                let key: aptos_api_types::HexEncodedBytes = key.clone().into();
                match op.bytes() {
                    None => {
                        changes.push(aptos_api_types::WriteSetChange::DeleteTableItem(
                            aptos_api_types::DeleteTableItem {
                                state_key_hash,
                                handle,
                                key,
                                data: None,
                            },
                        ));
                    },
                    Some(bytes) => {
                        changes.push(aptos_api_types::WriteSetChange::WriteTableItem(
                            aptos_api_types::WriteTableItem {
                                state_key_hash,
                                handle,
                                key,
                                value: bytes.to_vec().into(),
                                data: None,
                            },
                        ));
                    },
                }
            },
            StateKeyInner::TradingNative(_) => {
                return Err(CliError::UnexpectedError(format!(
                    "Can't convert trading-native key {:?} to WriteSetChange",
                    state_key.inner()
                )));
            },
            StateKeyInner::Raw(_) => {},
        }
    }

    serialize_as_json(&changes)
}

fn local_materialize_output(
    state_view: &impl aptos_types::state_store::StateView,
    vm_output: VMOutput,
) -> CliTypedResult<TransactionOutput> {
    let resolver = state_view.as_move_resolver();
    vm_output
        .try_materialize_into_transaction_output(&resolver)
        .map_err(|err| CliError::UnexpectedError(err.to_string()))
}

/// Transaction options for the `Simulate` and `Replay` commands.
///
/// A lighter-weight alternative to `TransactionOptions` that provides local
/// simulation, benchmarking, gas profiling, and remote simulation capabilities
/// directly, without routing through `AptosContext`.
#[derive(Debug, Default, Parser)]
pub(crate) struct TxnOptions {
    /// Sender account address
    ///
    /// This allows you to override the account address from the derived account address
    /// in the event that the authentication key was rotated or for a resource account
    #[clap(long, value_parser = aptos_cli_common::load_account_arg)]
    pub(crate) sender_account: Option<AccountAddress>,

    #[clap(flatten)]
    pub(crate) private_key_options: PrivateKeyInputOptions,
    #[clap(flatten)]
    pub(crate) encoding_options: EncodingOptions,
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
    #[clap(flatten)]
    pub(crate) gas_options: GasOptions,
    #[clap(flatten)]
    pub prompt_options: PromptOptions,
    /// Replay protection mechanism to use when generating the transaction.
    ///
    /// When "nonce" is chosen, the transaction will be an orderless transaction and contains a replay protection nonce.
    ///
    /// When "seqnum" is chosen, the transaction will contain a sequence number that matches with the sender's onchain sequence number.
    #[clap(long, default_value_t = ReplayProtectionType::Seqnum)]
    pub(crate) replay_protection_type: ReplayProtectionType,
}

impl TxnOptions {
    /// Builds a rest client
    pub fn rest_client(&self) -> CliTypedResult<Client> {
        self.rest_options.client(&self.profile_options)
    }

    /// Retrieves the private key and the associated address
    /// TODO: Cache this information
    pub fn get_key_and_address(&self) -> CliTypedResult<(Ed25519PrivateKey, AccountAddress)> {
        self.private_key_options.extract_private_key_and_address(
            self.encoding_options.encoding,
            &self.profile_options,
            self.sender_account,
        )
    }

    pub fn get_address(&self) -> CliTypedResult<AccountAddress> {
        self.private_key_options.extract_address(
            self.encoding_options.encoding,
            &self.profile_options,
            self.sender_account,
        )
    }

    pub async fn simulate_remotely(
        &self,
        rng: &mut rand::rngs::StdRng,
        payload: TransactionPayload,
        show_details: bool,
    ) -> CliTypedResult<TransactionSummary> {
        let client = self.rest_client()?;
        let sender_address = self.get_address()?;
        let (sender_private_key, _) = self.get_key_and_address()?;
        let sender_public_key = sender_private_key.public_key();

        let gas_unit_price = if let Some(gas_unit_price) = self.gas_options.gas_unit_price {
            gas_unit_price
        } else {
            client.estimate_gas_price().await?.into_inner().gas_estimate
        };

        // Get sequence number for account
        let (account, state) = get_account_with_state(&client, sender_address).await?;
        let sequence_number = account.sequence_number;

        // Retrieve local time, and ensure it's within an expected skew of the blockchain
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| CliError::UnexpectedError(err.to_string()))?
            .as_secs();
        let now_usecs = now * US_IN_SECS;

        // Warn local user that clock is skewed behind the blockchain.
        // There will always be a little lag from real time to blockchain time
        if now_usecs < state.timestamp_usecs - ACCEPTED_CLOCK_SKEW_US {
            eprintln!("Local clock is is skewed from blockchain clock.  Clock is more than {} seconds behind the blockchain {}", ACCEPTED_CLOCK_SKEW_US, state.timestamp_usecs / US_IN_SECS );
        }
        let expiration_time_secs = now + self.gas_options.expiration_secs;

        let chain_id = ChainId::new(state.chain_id);

        let transaction_factory =
            TransactionFactory::new(chain_id).with_gas_unit_price(gas_unit_price);

        let mut txn_builder = transaction_factory
            .payload(payload.clone())
            .sender(sender_address)
            .sequence_number(sequence_number)
            .expiration_timestamp_secs(expiration_time_secs);
        if self.replay_protection_type == ReplayProtectionType::Nonce {
            txn_builder = txn_builder.upgrade_payload_with_rng(rng, true, true);
        }
        let unsigned_transaction = txn_builder.build();

        let signed_transaction = SignedTransaction::new(
            unsigned_transaction,
            sender_public_key,
            Ed25519Signature::try_from([0u8; 64].as_ref()).unwrap(),
        );

        let simulated_txn = client
            .simulate_with_gas_estimation(&signed_transaction, true, false)
            .await?
            .into_inner()
            .into_iter()
            .next()
            .ok_or_else(|| {
                CliError::UnexpectedError(
                    "Simulation returned an empty transaction list".to_string(),
                )
            })?;

        let replay_protector = simulated_txn.request.replay_protector();
        let sequence_number = match &replay_protector {
            ReplayProtector::SequenceNumber(sequence_number) => Some(*sequence_number),
            _ => None,
        };

        let mut summary = TransactionSummary {
            transaction_hash: simulated_txn.info.hash,
            gas_used: Some(simulated_txn.info.gas_used.0),
            gas_unit_price: Some(simulated_txn.request.gas_unit_price.0),
            pending: None,
            sender: Some(*simulated_txn.request.sender.inner()),
            replay_protector: Some(replay_protector),
            sequence_number,
            success: Some(simulated_txn.info.success),
            timestamp_us: None,
            version: Some(simulated_txn.info.version.0),
            vm_status: Some(simulated_txn.info.vm_status.clone()),
            deployed_object_address: None,
            events: None,
            changes: None,
        };
        if show_details {
            summary.events = Some(serialize_as_json(&simulated_txn.events)?);
            summary.changes = Some(serialize_as_json(&simulated_txn.info.changes)?);
        }

        Ok(summary)
    }

    /// Simulates a transaction locally, using the debugger to fetch required data from remote.
    async fn simulate_using_debugger<F>(
        &self,
        payload: TransactionPayload,
        env: &MoveEnv,
        show_details: bool,
        execute: F,
    ) -> CliTypedResult<TransactionSummary>
    where
        F: FnOnce(
            &dyn MoveDebugger,
            u64,
            SignedTransaction,
            aptos_crypto::HashValue,
            PersistedAuxiliaryInfo,
        ) -> CliTypedResult<(VMStatus, VMOutput)>,
    {
        let client = self.rest_client()?;

        // Fetch the chain states required for the simulation
        // TODO(Gas): get the following from the chain
        const DEFAULT_GAS_UNIT_PRICE: u64 = 100;
        const DEFAULT_MAX_GAS: u64 = 2_000_000;

        let (sender_key, sender_address) = self.get_key_and_address()?;
        let gas_unit_price = self
            .gas_options
            .gas_unit_price
            .unwrap_or(DEFAULT_GAS_UNIT_PRICE);
        let (account, state) = get_account_with_state(&client, sender_address).await?;
        let version = state.version;
        let chain_id = ChainId::new(state.chain_id);
        let sequence_number = account.sequence_number;

        let balance = client
            .view_apt_account_balance_at_version(sender_address, version)
            .await
            .map_err(|err| CliError::ApiError(err.to_string()))?
            .into_inner();

        let max_gas = self.gas_options.max_gas.unwrap_or_else(|| {
            if gas_unit_price == 0 {
                DEFAULT_MAX_GAS
            } else {
                std::cmp::min(balance / gas_unit_price, DEFAULT_MAX_GAS)
            }
        });

        let transaction_factory = TransactionFactory::new(chain_id)
            .with_gas_unit_price(gas_unit_price)
            .with_max_gas_amount(max_gas)
            .with_transaction_expiration_time(self.gas_options.expiration_secs);
        let sender_account = &mut LocalAccount::new(sender_address, sender_key, sequence_number);
        let transaction =
            sender_account.sign_with_transaction_builder(transaction_factory.payload(payload));
        let hash = transaction.committed_hash();

        let debugger = env.create_move_debugger(client)?;
        let (vm_status, vm_output) = execute(
            &*debugger,
            version,
            transaction,
            hash,
            PersistedAuxiliaryInfo::None,
        )?;

        let success = match vm_output.status() {
            TransactionStatus::Keep(exec_status) => Some(exec_status.is_success()),
            TransactionStatus::Discard(_) | TransactionStatus::Retry => None,
        };

        let mut summary = TransactionSummary {
            transaction_hash: hash.into(),
            gas_used: Some(vm_output.gas_used()),
            gas_unit_price: Some(gas_unit_price),
            pending: None,
            sender: Some(sender_address),
            sequence_number: None,
            replay_protector: None, // The transaction is not committed so there is no new sequence number.
            success,
            timestamp_us: None,
            version: Some(version), // The transaction is not committed so there is no new version.
            vm_status: Some(format_txn_status(vm_output.status(), &vm_status)),
            deployed_object_address: None,
            events: None,
            changes: None,
        };
        if show_details {
            let state_view = debugger.state_view_at_version(version);
            let transaction_output = local_materialize_output(&state_view, vm_output)?;
            summary.events = Some(local_contract_events_to_json(
                &state_view,
                transaction_output.events(),
            )?);
            summary.changes = Some(local_write_set_to_json(
                &state_view,
                transaction_output.write_set(),
            )?);
        }

        Ok(summary)
    }

    /// Simulates a transaction locally.
    pub async fn simulate_locally(
        &self,
        payload: TransactionPayload,
        env: &MoveEnv,
        show_details: bool,
    ) -> CliTypedResult<TransactionSummary> {
        println!();
        println!("Simulating transaction locally...");

        self.simulate_using_debugger(
            payload,
            env,
            show_details,
            local_simulation::run_transaction_using_debugger,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::{local_events_to_json, local_write_set_to_json, serialize_as_json};
    use aptos_cli_common::TransactionSummary;
    use aptos_crypto::HashValue;
    use aptos_transaction_simulation::EmptyStateView;
    use aptos_types::{
        account_address::AccountAddress,
        event::EventKey,
        state_store::{state_key::StateKey, table::TableHandle},
        write_set::{WriteOp, WriteSet},
    };
    use move_core_types::{ident_str, language_storage::TypeTag};

    #[test]
    fn simulation_summary_omits_optional_fields_by_default() {
        let summary = TransactionSummary {
            transaction_hash: HashValue::zero().into(),
            gas_used: None,
            gas_unit_price: None,
            pending: None,
            sender: None,
            sequence_number: None,
            replay_protector: None,
            success: None,
            timestamp_us: None,
            version: None,
            vm_status: None,
            deployed_object_address: None,
            events: None,
            changes: None,
        };
        let value = serde_json::to_value(summary).unwrap();
        assert!(value.get("events").is_none());
        assert!(value.get("changes").is_none());
    }

    #[test]
    fn simulation_summary_includes_requested_details_only() {
        let summary = TransactionSummary {
            transaction_hash: HashValue::zero().into(),
            gas_used: None,
            gas_unit_price: None,
            pending: None,
            sender: None,
            sequence_number: None,
            replay_protector: None,
            success: None,
            timestamp_us: None,
            version: None,
            vm_status: None,
            deployed_object_address: None,
            events: None,
            changes: None,
        };
        let mut with_events = summary.clone();
        with_events.events = Some(serde_json::json!([{"k": 1}]));
        let value = serde_json::to_value(with_events).unwrap();
        assert!(value.get("events").is_some());
        assert!(value.get("changes").is_none());

        let mut with_changes = summary;
        with_changes.changes = Some(serde_json::json!([{"c": 2}]));
        let value = serde_json::to_value(with_changes).unwrap();
        assert!(value.get("events").is_none());
        assert!(value.get("changes").is_some());
    }

    #[test]
    fn local_events_use_api_event_shape() {
        let key = EventKey::new(9, AccountAddress::from_hex_literal("0x1").unwrap());
        let event = aptos_types::contract_event::ContractEvent::new_v1(
            key,
            7,
            TypeTag::U64,
            bcs::to_bytes(&42u64).unwrap(),
        )
        .unwrap();

        let state_view = EmptyStateView;
        let json = local_events_to_json(&state_view, &[(event, None)]).unwrap();
        let first = json.as_array().unwrap().first().unwrap();
        assert!(first.get("guid").is_some());
        assert_eq!(first.get("sequence_number").unwrap().as_str(), Some("7"));
        assert_eq!(first.get("type").unwrap().as_str(), Some("u64"));
        assert_eq!(first.get("data").unwrap().as_str(), Some("42"));
        assert!(first.get("raw_data_hex").is_none());
    }

    #[test]
    fn serialize_as_json_preserves_shape() {
        let raw = serde_json::json!({
            "events": [{"amount": "1"}],
            "changes": [{"type": "write_resource"}]
        });
        let encoded = serialize_as_json(&raw).unwrap();
        assert_eq!(encoded, raw);
    }

    #[test]
    fn local_changes_use_api_writeset_change_shape() {
        let state_view = EmptyStateView;
        let handle = TableHandle(AccountAddress::from_hex_literal("0x1").unwrap());
        let key = b"hello".to_vec();
        let state_key = StateKey::table_item(&handle, &key);
        let write_set = WriteSet::new([(
            state_key.clone(),
            WriteOp::legacy_modification(vec![1u8, 2u8, 3u8].into()),
        )])
        .unwrap();

        let json = local_write_set_to_json(&state_view, &write_set).unwrap();
        let first = json.as_array().unwrap().first().unwrap();
        assert_eq!(
            first.get("type").and_then(|v| v.as_str()),
            Some("write_table_item")
        );
        assert_eq!(
            first.get("key").and_then(|v| v.as_str()),
            Some("0x68656c6c6f")
        );
        assert_eq!(
            first.get("value").and_then(|v| v.as_str()),
            Some("0x010203")
        );
        assert!(first.get("state_key_hash").is_some());
    }

    #[test]
    fn local_changes_preserve_delete_module_entries() {
        let state_view = EmptyStateView;
        let address = AccountAddress::from_hex_literal("0x1").unwrap();
        let state_key = StateKey::module(&address, ident_str!("test_module"));
        let write_set = WriteSet::new([(state_key, WriteOp::legacy_deletion())]).unwrap();

        let json = local_write_set_to_json(&state_view, &write_set).unwrap();
        let first = json.as_array().unwrap().first().unwrap();
        assert_eq!(
            first.get("type").and_then(|v| v.as_str()),
            Some("delete_module")
        );
        assert_eq!(first.get("address").and_then(|v| v.as_str()), Some("0x1"));
    }

    #[test]
    fn local_changes_reject_trading_native_entries() {
        let state_view = EmptyStateView;
        let address = AccountAddress::from_hex_literal("0x1").unwrap();
        let state_key = StateKey::position(address, address, address);
        let write_set = WriteSet::new([(
            state_key,
            WriteOp::legacy_modification(vec![1u8, 2u8, 3u8].into()),
        )])
        .unwrap();

        let err = local_write_set_to_json(&state_view, &write_set).unwrap_err();
        assert!(err.to_string().contains("TradingNative"));
    }
}
