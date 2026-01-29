// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use aptos_types::{
    error,
    transaction::{
        authenticator::AuthenticationKey,
        user_transaction_context::{
            EntryFunctionPayload, TransactionIndexKind, UserTransactionContext,
        },
    },
};
use better_any::{Tid, TidAble};
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::{NumArgs, NumBytes};
use move_vm_runtime::{
    native_extensions::{NativeRuntimeRefCheckModelsCompleted, SessionListener},
    native_functions::NativeFunction,
};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

pub mod abort_codes {
    pub const ETRANSACTION_CONTEXT_NOT_AVAILABLE: u64 = 1;
    pub const EMONOTONICALLY_INCREASING_COUNTER_OVERFLOW: u64 = 2;
    pub const ETRANSACTION_INDEX_NOT_AVAILABLE: u64 = 5;
}

use move_core_types::language_storage::{OPTION_NONE_TAG, OPTION_SOME_TAG};

/// The native transaction context extension. This needs to be attached to the
/// NativeContextExtensions value which is passed into session functions, so it
/// is accessible from natives of this extension.
#[derive(Tid)]
pub struct NativeTransactionContext {
    session_hash: Vec<u8>,
    /// The number of AUIDs (Aptos unique identifiers) issued during the
    /// execution of this transaction.
    auid_counter: u64,
    /// The local counter to support the monotonically increasing counter feature.
    /// The monotically increasing counter outputs `<reserved_byte> timestamp || transaction_index || session counter || local_counter`.
    local_counter: u16,

    script_hash: Vec<u8>,
    chain_id: u8,
    /// A transaction context is available upon transaction prologue/execution/epilogue. It is not available
    /// when a VM session is created for other purposes, such as for processing validator transactions.
    user_transaction_context_opt: Option<UserTransactionContext>,
    /// A number to represent the sessions inside the execution of a transaction. Used for computing the `monotonically_increasing_counter` method.
    session_counter: u8,
}

impl NativeRuntimeRefCheckModelsCompleted for NativeTransactionContext {
    // No native functions in this context return references, so no models to add.
}

impl SessionListener for NativeTransactionContext {
    fn start(&mut self, session_hash: &[u8; 32], script_hash: &[u8], session_counter: u8) {
        self.session_hash = session_hash.to_vec();
        self.auid_counter = 0;
        self.local_counter = 0;
        self.script_hash = script_hash.to_vec();
        // Chain ID is persisted.
        // User transaction context is persisted.
        self.session_counter = session_counter;
    }

    fn finish(&mut self) {
        // No state changes to save.
    }

    fn abort(&mut self) {
        // No state changes to abort. Context will be reset on new session's start.
    }
}

impl NativeTransactionContext {
    /// Create a new instance of a native transaction context. This must be passed in via an
    /// extension into VM session functions.
    pub fn new(
        session_hash: Vec<u8>,
        script_hash: Vec<u8>,
        chain_id: u8,
        user_transaction_context_opt: Option<UserTransactionContext>,
        session_counter: u8,
    ) -> Self {
        Self {
            session_hash,
            auid_counter: 0,
            local_counter: 0,
            script_hash,
            chain_id,
            user_transaction_context_opt,
            session_counter,
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
    _ty_args: &[Type],
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_GET_TXN_HASH_BASE)?;
    let transaction_context = context.extensions().get::<NativeTransactionContext>();

    Ok(smallvec![Value::vector_u8(
        transaction_context.session_hash.clone()
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
    _ty_args: &[Type],
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_GENERATE_UNIQUE_ADDRESS_BASE)?;

    let transaction_context = context
        .extensions_mut()
        .get_mut::<NativeTransactionContext>();
    transaction_context.auid_counter += 1;

    let auid = AuthenticationKey::auid(
        transaction_context.session_hash.clone(),
        transaction_context.auid_counter,
    )
    .account_address();
    Ok(smallvec![Value::address(auid)])
}

/***************************************************************************************************
 * native fun monotonically_increasing_counter_internal
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_monotonically_increasing_counter_internal(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_MONOTONICALLY_INCREASING_COUNTER_BASE)?;

    let transaction_context = context
        .extensions_mut()
        .get_mut::<NativeTransactionContext>();
    if transaction_context.local_counter == u16::MAX {
        return Err(SafeNativeError::abort_with_message(
            error::invalid_state(abort_codes::EMONOTONICALLY_INCREASING_COUNTER_OVERFLOW),
            "Monotonically increasing counter has reached maximum value (too many calls in this session)",
        ));
    }
    transaction_context.local_counter += 1;
    let local_counter = transaction_context.local_counter as u128;
    let session_counter = transaction_context.session_counter as u128;

    let user_transaction_context_opt: &Option<UserTransactionContext> =
        get_user_transaction_context_opt_from_context(context);
    if let Some(user_transaction_context) = user_transaction_context_opt {
        // monotonically_increasing_counter (128 bits) = `<reserved_byte (8 bits)> || timestamp_us (64 bits) || transaction_index (32 bits) || session counter (8 bits) || local_counter (16 bits)`
        // reserved_byte: 0 for block/chunk execution (V1), 1 for validation/simulation (TimestampNotYetAssignedV1)
        let timestamp_us = safely_pop_arg!(args, u64);
        let transaction_index_kind = user_transaction_context.transaction_index_kind();

        let (reserved_byte, transaction_index) = match transaction_index_kind {
            TransactionIndexKind::BlockExecution { transaction_index } => {
                (0u128, transaction_index)
            },
            TransactionIndexKind::ValidationOrSimulation { transaction_index } => {
                (1u128, transaction_index)
            },
            TransactionIndexKind::NotAvailable => {
                return Err(SafeNativeError::abort_with_message(
                    error::invalid_state(abort_codes::ETRANSACTION_INDEX_NOT_AVAILABLE),
                    "Transaction index is not available in this execution context",
                ));
            },
        };

        let mut monotonically_increasing_counter: u128 = reserved_byte << 120;
        monotonically_increasing_counter |= (timestamp_us as u128) << 56;
        monotonically_increasing_counter |= (transaction_index as u128) << 24;
        monotonically_increasing_counter |= session_counter << 16;
        monotonically_increasing_counter |= local_counter;
        Ok(smallvec![Value::u128(monotonically_increasing_counter)])
    } else {
        // When transaction context is not available, return an error
        Err(SafeNativeError::abort_with_message(
            error::invalid_state(abort_codes::ETRANSACTION_CONTEXT_NOT_AVAILABLE),
            "Transaction context is not available (must be called during transaction execution)",
        ))
    }
}

/***************************************************************************************************
 * native fun monotonically_increasing_counter_internal_for_test_only
 *
 *   gas cost: base_cost
 *
 *   This is a test-only version that returns increasing counter values without requiring
 *   a user transaction context. Used when COMPILE_FOR_TESTING flag is enabled.
 *
 **************************************************************************************************/
fn native_monotonically_increasing_counter_internal_for_test_only(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_MONOTONICALLY_INCREASING_COUNTER_BASE)?;

    let transaction_context = context
        .extensions_mut()
        .get_mut::<NativeTransactionContext>();
    if transaction_context.local_counter == u16::MAX {
        return Err(SafeNativeError::abort_with_message(
            error::invalid_state(abort_codes::EMONOTONICALLY_INCREASING_COUNTER_OVERFLOW),
            "Monotonically increasing counter has reached maximum value (too many calls in this session)",
        ));
    }
    transaction_context.local_counter += 1;
    let local_counter = transaction_context.local_counter as u128;

    // For testing, return just the local counter value to verify monotonically increasing behavior
    Ok(smallvec![Value::u128(local_counter)])
}

/***************************************************************************************************
 * native fun get_script_hash
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_get_script_hash(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
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
    _ty_args: &[Type],
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_SENDER_BASE)?;

    let user_transaction_context_opt = get_user_transaction_context_opt_from_context(context);
    if let Some(transaction_context) = user_transaction_context_opt {
        Ok(smallvec![Value::address(transaction_context.sender())])
    } else {
        Err(SafeNativeError::abort_with_message(
            error::invalid_state(abort_codes::ETRANSACTION_CONTEXT_NOT_AVAILABLE),
            "Transaction context is not available (sender information can only be accessed during transaction execution)",
        ))
    }
}

fn native_secondary_signers_internal(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
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
        Err(SafeNativeError::abort_with_message(
            error::invalid_state(abort_codes::ETRANSACTION_CONTEXT_NOT_AVAILABLE),
            "Transaction context is not available (secondary signers can only be accessed during transaction execution)",
        ))
    }
}

fn native_gas_payer_internal(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_FEE_PAYER_BASE)?;

    let user_transaction_context_opt = get_user_transaction_context_opt_from_context(context);
    if let Some(transaction_context) = user_transaction_context_opt {
        Ok(smallvec![Value::address(transaction_context.gas_payer())])
    } else {
        Err(SafeNativeError::abort_with_message(
            error::invalid_state(abort_codes::ETRANSACTION_CONTEXT_NOT_AVAILABLE),
            "Transaction context is not available (gas payer information can only be accessed during transaction execution)",
        ))
    }
}

fn native_max_gas_amount_internal(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_MAX_GAS_AMOUNT_BASE)?;

    let user_transaction_context_opt = get_user_transaction_context_opt_from_context(context);
    if let Some(transaction_context) = user_transaction_context_opt {
        Ok(smallvec![Value::u64(transaction_context.max_gas_amount())])
    } else {
        Err(SafeNativeError::abort_with_message(
            error::invalid_state(abort_codes::ETRANSACTION_CONTEXT_NOT_AVAILABLE),
            "Transaction context is not available (max gas amount can only be accessed during transaction execution)",
        ))
    }
}

fn native_gas_unit_price_internal(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_GAS_UNIT_PRICE_BASE)?;

    let user_transaction_context_opt = get_user_transaction_context_opt_from_context(context);
    if let Some(transaction_context) = user_transaction_context_opt {
        Ok(smallvec![Value::u64(transaction_context.gas_unit_price())])
    } else {
        Err(SafeNativeError::abort_with_message(
            error::invalid_state(abort_codes::ETRANSACTION_CONTEXT_NOT_AVAILABLE),
            "Transaction context is not available (gas unit price can only be accessed during transaction execution)",
        ))
    }
}

fn native_chain_id_internal(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_CHAIN_ID_BASE)?;

    let user_transaction_context_opt = get_user_transaction_context_opt_from_context(context);
    if let Some(transaction_context) = user_transaction_context_opt {
        Ok(smallvec![Value::u8(transaction_context.chain_id())])
    } else {
        Err(SafeNativeError::abort_with_message(
            error::invalid_state(abort_codes::ETRANSACTION_CONTEXT_NOT_AVAILABLE),
            "Transaction context is not available (chain ID can only be accessed during transaction execution)",
        ))
    }
}

fn create_option_some(enum_option_enabled: bool, value: Value) -> PartialVMResult<Value> {
    Ok(if enum_option_enabled {
        Value::struct_(Struct::pack_variant(OPTION_SOME_TAG, vec![value]))
    } else {
        // Note: the collection is homogeneous because it contains only one value.
        Value::struct_(Struct::pack(vec![Value::vector_unchecked(vec![value])?]))
    })
}

fn create_option_none(enum_option_enabled: bool) -> PartialVMResult<Value> {
    Ok(if enum_option_enabled {
        Value::struct_(Struct::pack_variant(OPTION_NONE_TAG, vec![]))
    } else {
        // We are creating empty vector - this is safe to do.
        Value::struct_(Struct::pack(vec![Value::vector_unchecked(vec![])?]))
    })
}

fn create_string_value(s: String) -> Value {
    Value::struct_(Struct::pack(vec![Value::vector_u8(s.as_bytes().to_vec())]))
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

fn create_entry_function_payload(
    entry_function_payload: EntryFunctionPayload,
) -> PartialVMResult<Value> {
    let args = entry_function_payload
        .args
        .into_iter()
        .map(Value::vector_u8)
        .collect::<Vec<_>>();

    let ty_args = entry_function_payload
        .ty_arg_names
        .into_iter()
        .map(create_string_value)
        .collect::<Vec<_>>();

    Ok(Value::struct_(Struct::pack(vec![
        Value::address(entry_function_payload.account_address),
        create_string_value(entry_function_payload.module_name),
        create_string_value(entry_function_payload.function_name),
        // SAFETY: both type arguments and arguments are homogeneous collections.
        Value::vector_unchecked(ty_args)?,
        Value::vector_unchecked(args)?,
    ])))
}

fn native_entry_function_payload_internal(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_ENTRY_FUNCTION_PAYLOAD_BASE)?;

    let user_transaction_context_opt = get_user_transaction_context_opt_from_context(context);
    let enum_option_enabled = context.get_feature_flags().is_enum_option_enabled();
    if let Some(transaction_context) = user_transaction_context_opt {
        if let Some(entry_function_payload) = transaction_context.entry_function_payload() {
            let num_bytes = num_bytes_from_entry_function_payload(&entry_function_payload);
            context.charge(
                TRANSACTION_CONTEXT_ENTRY_FUNCTION_PAYLOAD_PER_BYTE_IN_STR
                    * NumBytes::new(num_bytes as u64),
            )?;
            let payload = create_entry_function_payload(entry_function_payload)?;
            Ok(smallvec![create_option_some(enum_option_enabled, payload)?])
        } else {
            Ok(smallvec![create_option_none(enum_option_enabled)?])
        }
    } else {
        Err(SafeNativeError::abort_with_message(
            error::invalid_state(abort_codes::ETRANSACTION_CONTEXT_NOT_AVAILABLE),
            "Transaction context is not available (entry function payload can only be accessed during transaction execution)",
        ))
    }
}

fn native_multisig_payload_internal(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_MULTISIG_PAYLOAD_BASE)?;

    let user_transaction_context_opt = get_user_transaction_context_opt_from_context(context);
    let enum_option_enabled = context.get_feature_flags().is_enum_option_enabled();
    if let Some(transaction_context) = user_transaction_context_opt {
        if let Some(multisig_payload) = transaction_context.multisig_payload() {
            let inner_entry_fun_payload =
                if let Some(entry_function_payload) = multisig_payload.entry_function_payload {
                    let num_bytes = num_bytes_from_entry_function_payload(&entry_function_payload);
                    context.charge(
                        TRANSACTION_CONTEXT_MULTISIG_PAYLOAD_PER_BYTE_IN_STR
                            * NumBytes::new(num_bytes as u64),
                    )?;
                    let inner_entry_fun_payload =
                        create_entry_function_payload(entry_function_payload)?;
                    create_option_some(enum_option_enabled, inner_entry_fun_payload)?
                } else {
                    create_option_none(enum_option_enabled)?
                };
            let multisig_payload = Value::struct_(Struct::pack(vec![
                Value::address(multisig_payload.multisig_address),
                inner_entry_fun_payload,
            ]));
            Ok(smallvec![create_option_some(
                enum_option_enabled,
                multisig_payload
            )?])
        } else {
            Ok(smallvec![create_option_none(enum_option_enabled)?])
        }
    } else {
        Err(SafeNativeError::abort_with_message(
            error::invalid_state(abort_codes::ETRANSACTION_CONTEXT_NOT_AVAILABLE),
            "Transaction context is not available (multisig payload can only be accessed during transaction execution)",
        ))
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
        (
            "monotonically_increasing_counter_internal",
            native_monotonically_increasing_counter_internal,
        ),
        (
            "monotonically_increasing_counter_internal_for_test_only",
            native_monotonically_increasing_counter_internal_for_test_only,
        ),
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_extension_update() {
        let mut ctx = NativeTransactionContext::new(vec![2; 32], vec![1; 2], 2, None, 32);
        ctx.auid_counter = 100;
        ctx.local_counter = 23;
        ctx.start(&[4; 32], &[2; 3], 44);

        let NativeTransactionContext {
            session_hash,
            auid_counter,
            local_counter,
            script_hash,
            chain_id,
            user_transaction_context_opt,
            session_counter,
        } = ctx;

        assert_eq!(session_hash, vec![4; 32]);
        assert_eq!(auid_counter, 0);
        assert_eq!(local_counter, 0);
        assert_eq!(script_hash, vec![2; 3]);
        assert_eq!(chain_id, 2);
        assert!(user_transaction_context_opt.is_none());
        assert_eq!(session_counter, 44);
    }
}
