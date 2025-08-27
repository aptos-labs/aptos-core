// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use aptos_types::{
    error,
    transaction::{
        authenticator::AuthenticationKey,
        user_transaction_context::{EntryFunctionPayload, UserTransactionContext},
    },
};
use better_any::{Tid, TidAble};
use move_core_types::gas_algebra::{NumArgs, NumBytes};
use move_vm_runtime::{
    native_extensions::VersionControlledNativeExtension, native_functions::NativeFunction,
};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

pub mod abort_codes {
    pub const ETRANSACTION_CONTEXT_NOT_AVAILABLE: u64 = 1;
}

/// The native transaction context extension. This needs to be attached to the
/// NativeContextExtensions value which is passed into session functions, so it
/// is accessible from natives of this extension.
#[derive(Tid)]
pub struct NativeTransactionContext {
    txn_hash: Vec<u8>,
    /// The number of AUIDs (Aptos unique identifiers) issued during the
    /// execution of this transaction.
    auid_counter: u64,
    script_hash: Vec<u8>,
    chain_id: u8,
    /// A transaction context is available upon transaction prologue/execution/epilogue. It is not available
    /// when a VM session is created for other purposes, such as for processing validator transactions.
    user_transaction_context_opt: Option<UserTransactionContext>,
}

impl VersionControlledNativeExtension for NativeTransactionContext {
    fn undo(&mut self) {
        // No-op: nothing to undo.
    }

    fn save(&mut self) {
        // No-op: nothing to save.
    }

    fn update(&mut self, txn_hash: &[u8; 32], script_hash: &[u8]) {
        self.txn_hash = txn_hash.to_vec();
        self.script_hash = script_hash.to_vec();
        self.auid_counter = 0;
    }
}

impl NativeTransactionContext {
    /// Create a new instance of a native transaction context. This must be passed in via an
    /// extension into VM session functions.
    pub fn new(
        txn_hash: Vec<u8>,
        script_hash: Vec<u8>,
        chain_id: u8,
        user_transaction_context_opt: Option<UserTransactionContext>,
    ) -> Self {
        Self {
            txn_hash,
            auid_counter: 0,
            script_hash,
            chain_id,
            user_transaction_context_opt,
        }
    }

    pub fn chain_id(&self) -> u8 {
        self.chain_id
    }
}

/***************************************************************************************************
 * native fun get_txn_hash
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_get_txn_hash(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_GET_TXN_HASH_BASE)?;
    let transaction_context = context.extensions().get::<NativeTransactionContext>();

    Ok(smallvec![Value::vector_u8(
        transaction_context.txn_hash.clone()
    )])
}

/***************************************************************************************************
 * native fun generate_unique_address
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_generate_unique_address(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_GENERATE_UNIQUE_ADDRESS_BASE)?;

    let transaction_context = context
        .extensions_mut()
        .get_mut::<NativeTransactionContext>();
    transaction_context.auid_counter += 1;

    let auid = AuthenticationKey::auid(
        transaction_context.txn_hash.clone(),
        transaction_context.auid_counter,
    )
    .account_address();
    Ok(smallvec![Value::address(auid)])
}

/***************************************************************************************************
 * native fun get_script_hash
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_get_script_hash(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_GET_SCRIPT_HASH_BASE)?;

    let transaction_context = context.extensions().get::<NativeTransactionContext>();

    Ok(smallvec![Value::vector_u8(
        transaction_context.script_hash.clone()
    )])
}

fn native_sender_internal(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_SENDER_BASE)?;

    let user_transaction_context_opt = get_user_transaction_context_opt_from_context(context);
    if let Some(transaction_context) = user_transaction_context_opt {
        Ok(smallvec![Value::address(transaction_context.sender())])
    } else {
        Err(SafeNativeError::Abort {
            abort_code: error::invalid_state(abort_codes::ETRANSACTION_CONTEXT_NOT_AVAILABLE),
        })
    }
}

fn native_secondary_signers_internal(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_SECONDARY_SIGNERS_BASE)?;

    let user_transaction_context_opt = get_user_transaction_context_opt_from_context(context);
    if let Some(transaction_context) = user_transaction_context_opt {
        let secondary_signers = transaction_context.secondary_signers();
        context.charge(
            TRANSACTION_CONTEXT_SECONDARY_SIGNERS_PER_SIGNER
                * NumArgs::new(secondary_signers.len() as u64),
        )?;
        Ok(smallvec![Value::vector_address(secondary_signers)])
    } else {
        Err(SafeNativeError::Abort {
            abort_code: error::invalid_state(abort_codes::ETRANSACTION_CONTEXT_NOT_AVAILABLE),
        })
    }
}

fn native_gas_payer_internal(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_FEE_PAYER_BASE)?;

    let user_transaction_context_opt = get_user_transaction_context_opt_from_context(context);
    if let Some(transaction_context) = user_transaction_context_opt {
        Ok(smallvec![Value::address(transaction_context.gas_payer())])
    } else {
        Err(SafeNativeError::Abort {
            abort_code: error::invalid_state(abort_codes::ETRANSACTION_CONTEXT_NOT_AVAILABLE),
        })
    }
}

fn native_max_gas_amount_internal(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_MAX_GAS_AMOUNT_BASE)?;

    let user_transaction_context_opt = get_user_transaction_context_opt_from_context(context);
    if let Some(transaction_context) = user_transaction_context_opt {
        Ok(smallvec![Value::u64(transaction_context.max_gas_amount())])
    } else {
        Err(SafeNativeError::Abort {
            abort_code: error::invalid_state(abort_codes::ETRANSACTION_CONTEXT_NOT_AVAILABLE),
        })
    }
}

fn native_gas_unit_price_internal(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_GAS_UNIT_PRICE_BASE)?;

    let user_transaction_context_opt = get_user_transaction_context_opt_from_context(context);
    if let Some(transaction_context) = user_transaction_context_opt {
        Ok(smallvec![Value::u64(transaction_context.gas_unit_price())])
    } else {
        Err(SafeNativeError::Abort {
            abort_code: error::invalid_state(abort_codes::ETRANSACTION_CONTEXT_NOT_AVAILABLE),
        })
    }
}

fn native_chain_id_internal(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_CHAIN_ID_BASE)?;

    let user_transaction_context_opt = get_user_transaction_context_opt_from_context(context);
    if let Some(transaction_context) = user_transaction_context_opt {
        Ok(smallvec![Value::u8(transaction_context.chain_id())])
    } else {
        Err(SafeNativeError::Abort {
            abort_code: error::invalid_state(abort_codes::ETRANSACTION_CONTEXT_NOT_AVAILABLE),
        })
    }
}

fn create_option_some_value(value: Value) -> Value {
    Value::struct_(Struct::pack(vec![create_singleton_vector(value)]))
}

fn create_option_none() -> Value {
    Value::struct_(Struct::pack(vec![create_empty_vector()]))
}

fn create_string_value(s: String) -> Value {
    Value::struct_(Struct::pack(vec![Value::vector_u8(s.as_bytes().to_vec())]))
}

fn create_vector_value(vv: Vec<Value>) -> Value {
    // This is safe because this function is only used to create vectors of homogenous values.
    Value::vector_for_testing_only(vv)
}

fn create_singleton_vector(v: Value) -> Value {
    create_vector_value(vec![v])
}

fn create_empty_vector() -> Value {
    create_vector_value(vec![])
}

fn num_bytes_from_entry_function_payload(entry_function_payload: &EntryFunctionPayload) -> usize {
    entry_function_payload.account_address.len()
        + entry_function_payload.module_name.len()
        + entry_function_payload.function_name.len()
        + entry_function_payload
            .ty_arg_names
            .iter()
            .map(|s| s.len())
            .sum::<usize>()
        + entry_function_payload
            .args
            .iter()
            .map(|v| v.len())
            .sum::<usize>()
}

fn create_entry_function_payload(entry_function_payload: EntryFunctionPayload) -> Value {
    let args = entry_function_payload
        .args
        .iter()
        .map(|arg| Value::vector_u8(arg.clone()))
        .collect::<Vec<_>>();

    let ty_args = entry_function_payload
        .ty_arg_names
        .iter()
        .map(|ty_arg| create_string_value(ty_arg.clone()))
        .collect::<Vec<_>>();

    Value::struct_(Struct::pack(vec![
        Value::address(entry_function_payload.account_address),
        create_string_value(entry_function_payload.module_name),
        create_string_value(entry_function_payload.function_name),
        create_vector_value(ty_args),
        create_vector_value(args),
    ]))
}

fn native_entry_function_payload_internal(
    context: &mut SafeNativeContext,
    mut _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_ENTRY_FUNCTION_PAYLOAD_BASE)?;

    let user_transaction_context_opt = get_user_transaction_context_opt_from_context(context);

    if let Some(transaction_context) = user_transaction_context_opt {
        if let Some(entry_function_payload) = transaction_context.entry_function_payload() {
            let num_bytes = num_bytes_from_entry_function_payload(&entry_function_payload);
            context.charge(
                TRANSACTION_CONTEXT_ENTRY_FUNCTION_PAYLOAD_PER_BYTE_IN_STR
                    * NumBytes::new(num_bytes as u64),
            )?;
            let payload = create_entry_function_payload(entry_function_payload);
            Ok(smallvec![create_option_some_value(payload)])
        } else {
            Ok(smallvec![create_option_none()])
        }
    } else {
        Err(SafeNativeError::Abort {
            abort_code: error::invalid_state(abort_codes::ETRANSACTION_CONTEXT_NOT_AVAILABLE),
        })
    }
}

fn native_multisig_payload_internal(
    context: &mut SafeNativeContext,
    mut _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_MULTISIG_PAYLOAD_BASE)?;

    let user_transaction_context_opt = get_user_transaction_context_opt_from_context(context);

    if let Some(transaction_context) = user_transaction_context_opt {
        if let Some(multisig_payload) = transaction_context.multisig_payload() {
            if let Some(entry_function_payload) = multisig_payload.entry_function_payload {
                let num_bytes = num_bytes_from_entry_function_payload(&entry_function_payload);
                context.charge(
                    TRANSACTION_CONTEXT_MULTISIG_PAYLOAD_PER_BYTE_IN_STR
                        * NumBytes::new(num_bytes as u64),
                )?;
                let inner_entry_fun_payload = create_entry_function_payload(entry_function_payload);
                let multisig_payload = Value::struct_(Struct::pack(vec![
                    Value::address(multisig_payload.multisig_address),
                    create_option_some_value(inner_entry_fun_payload),
                ]));
                Ok(smallvec![create_option_some_value(multisig_payload)])
            } else {
                let multisig_payload = Value::struct_(Struct::pack(vec![
                    Value::address(multisig_payload.multisig_address),
                    create_option_none(),
                ]));
                Ok(smallvec![create_option_some_value(multisig_payload)])
            }
        } else {
            Ok(smallvec![create_option_none()])
        }
    } else {
        Err(SafeNativeError::Abort {
            abort_code: error::invalid_state(abort_codes::ETRANSACTION_CONTEXT_NOT_AVAILABLE),
        })
    }
}

fn get_user_transaction_context_opt_from_context<'a>(
    context: &'a SafeNativeContext,
) -> &'a Option<UserTransactionContext> {
    &context
        .extensions()
        .get::<NativeTransactionContext>()
        .user_transaction_context_opt
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("get_script_hash", native_get_script_hash as RawSafeNative),
        ("generate_unique_address", native_generate_unique_address),
        ("get_txn_hash", native_get_txn_hash),
        ("sender_internal", native_sender_internal),
        (
            "secondary_signers_internal",
            native_secondary_signers_internal,
        ),
        ("gas_payer_internal", native_gas_payer_internal),
        ("max_gas_amount_internal", native_max_gas_amount_internal),
        ("gas_unit_price_internal", native_gas_unit_price_internal),
        ("chain_id_internal", native_chain_id_internal),
        (
            "entry_function_payload_internal",
            native_entry_function_payload_internal,
        ),
        (
            "multisig_payload_internal",
            native_multisig_payload_internal,
        ),
    ];

    builder.make_named_natives(natives)
}
