// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Transaction options and local simulation support for the `aptos move simulate` command.

use crate::{local_simulation, MoveDebugger, MoveEnv};
// Re-export from aptos-cli-common to eliminate the duplicate definition.
pub use aptos_cli_common::ReplayProtectionType;
use aptos_cli_common::{
    format_txn_status, get_account_with_state, CliError, CliTypedResult, EncodingOptions,
    ExtractEd25519PublicKey,
    GasOptions, PrivateKeyInputOptions, ProfileOptions, PromptOptions, RestOptions,
    TransactionSummary, ACCEPTED_CLOCK_SKEW_US, US_IN_SECS,
};
use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519Signature};
use aptos_rest_client::Client;
use aptos_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use aptos_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    contract_event::ContractEvent,
    transaction::{
        PersistedAuxiliaryInfo, ReplayProtector, SignedTransaction, TransactionPayload,
        TransactionStatus,
    },
    write_set::{BaseStateOp, WriteOp},
};
use aptos_vm_types::{abstract_write_op::AbstractResourceWriteOp, output::VMOutput};
use clap::Parser;
use move_core_types::{
    language_storage::TypeTag,
    value::{MoveTypeLayout, MoveValue},
    vm_status::VMStatus,
};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

fn serialize_as_json<T: Serialize>(value: &T) -> CliTypedResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|err| CliError::UnexpectedError(err.to_string()))
}

fn type_tag_to_simple_layout(type_tag: &TypeTag) -> Option<MoveTypeLayout> {
    use TypeTag::*;

    match type_tag {
        Bool => Some(MoveTypeLayout::Bool),
        U8 => Some(MoveTypeLayout::U8),
        U16 => Some(MoveTypeLayout::U16),
        U32 => Some(MoveTypeLayout::U32),
        U64 => Some(MoveTypeLayout::U64),
        U128 => Some(MoveTypeLayout::U128),
        U256 => Some(MoveTypeLayout::U256),
        I8 => Some(MoveTypeLayout::I8),
        I16 => Some(MoveTypeLayout::I16),
        I32 => Some(MoveTypeLayout::I32),
        I64 => Some(MoveTypeLayout::I64),
        I128 => Some(MoveTypeLayout::I128),
        I256 => Some(MoveTypeLayout::I256),
        Address => Some(MoveTypeLayout::Address),
        Vector(inner) => {
            type_tag_to_simple_layout(inner).map(|layout| MoveTypeLayout::Vector(Box::new(layout)))
        },
        Struct(_) | Signer | Function(_) => None,
    }
}

fn event_data_to_json(
    event_data: &[u8],
    layout: Option<&MoveTypeLayout>,
    raw_data_hex: &str,
) -> serde_json::Value {
    if let Some(layout) = layout
        && let Ok(decoded) = MoveValue::simple_deserialize(event_data, layout)
        && let Ok(decoded_json) = serde_json::to_value(decoded)
    {
        return decoded_json;
    }
    serde_json::json!(raw_data_hex)
}

fn contract_event_to_json(event: &ContractEvent) -> serde_json::Value {
    let decode_layout = type_tag_to_simple_layout(event.type_tag());
    let raw_data_hex = hex::encode(event.event_data());
    serde_json::json!({
        "event_key": event.event_key().map(|key| key.to_string()),
        "sequence_number": event.v1().ok().map(|v1| v1.sequence_number()),
        "type": event.type_tag().to_canonical_string(),
        "data": event_data_to_json(event.event_data(), decode_layout.as_ref(), &raw_data_hex),
        "raw_data_hex": raw_data_hex,
    })
}

fn local_events_to_json(events: &[(ContractEvent, Option<MoveTypeLayout>)]) -> serde_json::Value {
    serde_json::Value::Array(
        events
            .iter()
            .map(|(event, _layout)| contract_event_to_json(event))
            .collect(),
    )
}

fn write_op_kind_to_str(write_op: &WriteOp) -> &'static str {
    match write_op.as_base_op() {
        BaseStateOp::Creation(_) => "creation",
        BaseStateOp::Modification(_) => "modification",
        BaseStateOp::Deletion(_) => "deletion",
        BaseStateOp::MakeHot => "make_hot",
    }
}

fn write_op_to_json(write_op: &WriteOp) -> serde_json::Value {
    if matches!(write_op.as_base_op(), BaseStateOp::MakeHot) {
        serde_json::json!({
            "op_type": write_op_kind_to_str(write_op),
            "data_hex": serde_json::Value::Null,
            "metadata": serde_json::Value::Null,
        })
    } else {
        serde_json::json!({
            "op_type": write_op_kind_to_str(write_op),
            "data_hex": write_op.bytes().map(hex::encode),
            "metadata": write_op.metadata().clone().into_persistable(),
        })
    }
}

fn encode_state_key_for_output(
    state_key: &aptos_types::state_store::state_key::StateKey,
) -> String {
    hex::encode(state_key.encoded())
}

fn local_changes_to_json(vm_output: &VMOutput) -> serde_json::Value {
    let mut changes = Vec::new();

    for (state_key, abstract_write_op) in vm_output.resource_write_set() {
        let (kind, write, abstract_op) = match abstract_write_op {
            AbstractResourceWriteOp::Write(write_op) => {
                ("resource_write", Some(write_op_to_json(write_op)), None)
            },
            AbstractResourceWriteOp::WriteWithDelayedFields(_) => (
                "resource_write_with_delayed_fields",
                None,
                Some(serde_json::json!({ "variant": "WriteWithDelayedFields" })),
            ),
            AbstractResourceWriteOp::WriteResourceGroup(_) => (
                "resource_group_write",
                None,
                Some(serde_json::json!({ "variant": "WriteResourceGroup" })),
            ),
            AbstractResourceWriteOp::InPlaceDelayedFieldChange(_) => (
                "in_place_delayed_field_change",
                None,
                Some(serde_json::json!({ "variant": "InPlaceDelayedFieldChange" })),
            ),
            AbstractResourceWriteOp::ResourceGroupInPlaceDelayedFieldChange(_) => (
                "resource_group_in_place_delayed_field_change",
                None,
                Some(serde_json::json!({ "variant": "ResourceGroupInPlaceDelayedFieldChange" })),
            ),
        };
        changes.push(serde_json::json!({
            "kind": kind,
            "state_key": encode_state_key_for_output(state_key),
            "write": write,
            "abstract_op": abstract_op,
        }));
    }

    for (state_key, module_write) in vm_output.module_write_set() {
        changes.push(serde_json::json!({
            "kind": "module_write",
            "state_key": encode_state_key_for_output(state_key),
            "module_id": module_write.module_id().to_string(),
            "write": write_op_to_json(module_write.write_op()),
        }));
    }

    for (state_key, write_op) in vm_output.aggregator_v1_write_set() {
        changes.push(serde_json::json!({
            "kind": "aggregator_v1_write",
            "state_key": encode_state_key_for_output(state_key),
            "write": write_op_to_json(write_op),
        }));
    }

    for (state_key, delta) in vm_output.aggregator_v1_delta_set() {
        changes.push(serde_json::json!({
            "kind": "aggregator_v1_delta",
            "state_key": encode_state_key_for_output(state_key),
            "delta": format!("{:?}", delta),
        }));
    }

    for (id, change) in vm_output.delayed_field_change_set() {
        changes.push(serde_json::json!({
            "kind": "delayed_field_change",
            "id": format!("{:?}", id),
            "change": format!("{:?}", change),
        }));
    }

    serde_json::Value::Array(changes)
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
        show_events: bool,
        show_changes: bool,
    ) -> CliTypedResult<TransactionSummary> {
        let client = self.rest_client()?;
        let sender_address = self.get_address()?;
        let sender_public_key = self
            .private_key_options
            .extract_public_key(self.encoding_options.encoding, &self.profile_options)?;

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
        if show_events {
            summary.events = Some(serialize_as_json(&simulated_txn.events)?);
        }
        if show_changes {
            summary.changes = Some(serialize_as_json(&simulated_txn.info.changes)?);
        }

        Ok(summary)
    }

    /// Simulates a transaction locally, using the debugger to fetch required data from remote.
    async fn simulate_using_debugger<F>(
        &self,
        payload: TransactionPayload,
        env: &MoveEnv,
        show_events: bool,
        show_changes: bool,
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
        if show_events {
            summary.events = Some(local_events_to_json(vm_output.events()));
        }
        if show_changes {
            summary.changes = Some(local_changes_to_json(&vm_output));
        }

        Ok(summary)
    }

    /// Simulates a transaction locally.
    pub async fn simulate_locally(
        &self,
        payload: TransactionPayload,
        env: &MoveEnv,
        show_events: bool,
        show_changes: bool,
    ) -> CliTypedResult<TransactionSummary> {
        println!();
        println!("Simulating transaction locally...");

        self.simulate_using_debugger(
            payload,
            env,
            show_events,
            show_changes,
            local_simulation::run_transaction_using_debugger,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::{encode_state_key_for_output, local_events_to_json, serialize_as_json};
    use aptos_cli_common::TransactionSummary;
    use aptos_crypto::HashValue;
    use aptos_types::{
        account_address::AccountAddress, event::EventKey, state_store::state_key::StateKey,
    };
    use move_core_types::{language_storage::TypeTag, value::MoveTypeLayout};

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
    fn local_events_decode_data_from_type_tag() {
        let key = EventKey::new(9, AccountAddress::from_hex_literal("0x1").unwrap());
        let event = aptos_types::contract_event::ContractEvent::new_v1(
            key,
            7,
            TypeTag::U64,
            bcs::to_bytes(&42u64).unwrap(),
        )
        .unwrap();

        let json = local_events_to_json(&[(event, Some(MoveTypeLayout::Bool))]);
        let first = json.as_array().unwrap().first().unwrap();
        assert_eq!(first.get("sequence_number").unwrap().as_u64(), Some(7));
        assert_eq!(first.get("type").unwrap().as_str(), Some("u64"));
        assert_eq!(first.get("data").unwrap().as_u64(), Some(42));
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
    fn state_key_output_is_stable_hex() {
        let state_key = StateKey::raw(b"abc");
        assert_eq!(encode_state_key_for_output(&state_key), "ff616263");
    }
}
