// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    common::{check_network, decode_bcs, find_fa_currency, native_coin, parse_coin_currency},
    error::{ApiError, ApiResult},
    types::*,
    RosettaContext,
};
use aptos_logger::debug;
use aptos_sdk::move_types::language_storage::TypeTag;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{RawTransaction, SignedTransaction, TransactionPayload},
};
use serde::de::DeserializeOwned;

/// Construction parse command (OFFLINE)
///
/// Parses operations from a transaction, used for verifying transaction construction
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionparse)
pub(crate) async fn construction_parse(
    request: ConstructionParseRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionParseResponse> {
    debug!("/construction/parse {:?}", request);
    check_network(request.network_identifier, &server_context)?;

    // For signed transactions, we can pull the signers and the raw transaction
    let metadata;
    let (account_identifier_signers, unsigned_txn) = if request.signed {
        let signed_txn: SignedTransaction = decode_bcs(&request.transaction, "SignedTransaction")?;
        metadata = Some(ConstructionParseMetadata {
            unsigned_transaction: None,
            signed_transaction: Some(signed_txn.clone()),
        });
        let mut account_identifier_signers: Vec<_> =
            vec![AccountIdentifier::base_account(signed_txn.sender())];

        for account in signed_txn.authenticator().secondary_signer_addresses() {
            account_identifier_signers.push(AccountIdentifier::base_account(account))
        }

        (
            Some(account_identifier_signers),
            signed_txn.into_raw_transaction(),
        )
    } else {
        // For unsigned transactions,w e can only pull the transaction
        let unsigned_txn: RawTransaction = decode_bcs(&request.transaction, "UnsignedTransaction")?;
        metadata = Some(ConstructionParseMetadata {
            unsigned_transaction: Some(unsigned_txn.clone()),
            signed_transaction: None,
        });
        (None, unsigned_txn)
    };

    // The sender however should always be present, even if not signed
    let sender = unsigned_txn.sender();

    // This is messy, but all we can do is to manually go through and check the entry functions associated to convert to Rosetta operations
    // TODO: We should centralize all this operation -> entry function / entry function -> operation code
    let operations = match unsigned_txn.into_payload() {
        TransactionPayload::EntryFunction(inner) => {
            let (module, function_name, type_args, args) = inner.into_inner();

            match (
                *module.address(),
                module.name().as_str(),
                function_name.as_str(),
            ) {
                (AccountAddress::ONE, COIN_MODULE, TRANSFER_FUNCTION)
                | (AccountAddress::ONE, APTOS_ACCOUNT_MODULE, TRANSFER_COINS_FUNCTION) => {
                    parse_transfer_operation(&server_context, sender, &type_args, &args)?
                },
                (AccountAddress::ONE, APTOS_ACCOUNT_MODULE, TRANSFER_FUNCTION) => {
                    parse_account_transfer_operation(sender, &type_args, &args)?
                },
                (AccountAddress::ONE, APTOS_ACCOUNT_MODULE, CREATE_ACCOUNT_FUNCTION) => {
                    parse_create_account_operation(sender, &type_args, &args)?
                },
                (AccountAddress::ONE, PRIMARY_FUNGIBLE_STORE_MODULE, TRANSFER_FUNCTION)
                | (AccountAddress::ONE, APTOS_ACCOUNT_MODULE, TRANSFER_FUNGIBLE_ASSETS_FUNCTION) => {
                    parse_primary_fa_transfer_operation(&server_context, sender, &type_args, &args)?
                },
                (AccountAddress::ONE, FUNGIBLE_ASSET_MODULE, TRANSFER_FUNCTION) => {
                    parse_fa_transfer_operation(&server_context, sender, &type_args, &args)?
                },
                (
                    AccountAddress::ONE,
                    STAKING_CONTRACT_MODULE,
                    SWITCH_OPERATOR_WITH_SAME_COMMISSION_FUNCTION,
                ) => parse_set_operator_operation(sender, &type_args, &args)?,
                (AccountAddress::ONE, STAKING_CONTRACT_MODULE, UPDATE_VOTER_FUNCTION) => {
                    parse_set_voter_operation(sender, &type_args, &args)?
                },
                (
                    AccountAddress::ONE,
                    STAKING_CONTRACT_MODULE,
                    CREATE_STAKING_CONTRACT_FUNCTION,
                ) => parse_create_stake_pool_operation(sender, &type_args, &args)?,
                (AccountAddress::ONE, STAKING_CONTRACT_MODULE, RESET_LOCKUP_FUNCTION) => {
                    parse_reset_lockup_operation(sender, &type_args, &args)?
                },
                (AccountAddress::ONE, STAKING_CONTRACT_MODULE, UPDATE_COMMISSION_FUNCTION) => {
                    parse_update_commission_operation(sender, &type_args, &args)?
                },
                (AccountAddress::ONE, STAKING_CONTRACT_MODULE, UNLOCK_STAKE_FUNCTION) => {
                    parse_unlock_stake_operation(sender, &type_args, &args)?
                },
                (
                    AccountAddress::ONE,
                    STAKING_CONTRACT_MODULE,
                    DISTRIBUTE_STAKING_REWARDS_FUNCTION,
                ) => parse_distribute_staking_rewards_operation(sender, &type_args, &args)?,
                (
                    AccountAddress::ONE,
                    DELEGATION_POOL_MODULE,
                    DELEGATION_POOL_ADD_STAKE_FUNCTION,
                ) => parse_delegation_pool_add_stake_operation(sender, &type_args, &args)?,
                (
                    AccountAddress::ONE,
                    DELEGATION_POOL_MODULE,
                    DELEGATION_POOL_WITHDRAW_FUNCTION,
                ) => parse_delegation_pool_withdraw_operation(sender, &type_args, &args)?,
                (AccountAddress::ONE, DELEGATION_POOL_MODULE, DELEGATION_POOL_UNLOCK_FUNCTION) => {
                    parse_delegation_pool_unlock_operation(sender, &type_args, &args)?
                },
                _ => {
                    return Err(ApiError::TransactionParseError(Some(format!(
                        "Unsupported entry function type {:x}::{}::{}",
                        module.address(),
                        module.name(),
                        function_name
                    ))));
                },
            }
        },
        payload => {
            return Err(ApiError::TransactionParseError(Some(format!(
                "Unsupported transaction payload type {:?}",
                payload
            ))))
        },
    };

    Ok(ConstructionParseResponse {
        operations,
        account_identifier_signers,
        metadata,
    })
}

/// Parses 0x1::aptos_account::create(auth_key: address)
fn parse_create_account_operation(
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    // There are no typeargs for create account
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Create account should not have type arguments: {:?}",
            type_args
        ))));
    }

    // Create account
    if let Some(encoded_address) = args.first() {
        let new_address: AccountAddress = bcs::from_bytes(encoded_address)?;

        Ok(vec![Operation::create_account(
            0,
            None,
            new_address,
            sender,
        )])
    } else {
        Err(ApiError::InvalidOperations(Some(
            "Create account doesn't have an address argument".to_string(),
        )))
    }
}

/// Parses 0x1::coin::transfer<CoinType>(receiver: address, amount: u64)
fn parse_transfer_operation(
    server_context: &RosettaContext,
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    let mut operations = Vec::new();

    // Check coin is the native coin
    let currency = match type_args.first() {
        Some(TypeTag::Struct(struct_tag)) => parse_coin_currency(server_context, struct_tag)?,
        _ => {
            return Err(ApiError::TransactionParseError(Some(
                "No coin type in transfer".to_string(),
            )))
        },
    };

    // Retrieve the args for the operations
    let receiver: AccountAddress = if let Some(receiver) = args.first() {
        bcs::from_bytes(receiver)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No receiver in transfer".to_string(),
        )));
    };
    let amount: u64 = if let Some(amount) = args.get(1) {
        bcs::from_bytes(amount)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No amount in transfer".to_string(),
        )));
    };

    operations.push(Operation::withdraw(
        0,
        None,
        AccountIdentifier::base_account(sender),
        currency.clone(),
        amount,
    ));
    operations.push(Operation::deposit(
        1,
        None,
        AccountIdentifier::base_account(receiver),
        currency,
        amount,
    ));
    Ok(operations)
}

/// Parses 0x1::aptos_account::transfer(receiver: address, amount: u64)
fn parse_account_transfer_operation(
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    // There are no typeargs for account transfer
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Account transfer should not have type arguments: {:?}",
            type_args
        ))));
    }
    let mut operations = Vec::new();

    // Retrieve the args for the operations
    // TODO: This is the same as coin::transfer, we should combine them
    let receiver: AccountAddress = if let Some(receiver) = args.first() {
        bcs::from_bytes(receiver)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No receiver in account transfer".to_string(),
        )));
    };
    let amount: u64 = if let Some(amount) = args.get(1) {
        bcs::from_bytes(amount)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No amount in account transfer".to_string(),
        )));
    };

    operations.push(Operation::withdraw(
        0,
        None,
        AccountIdentifier::base_account(sender),
        native_coin(),
        amount,
    ));
    operations.push(Operation::deposit(
        1,
        None,
        AccountIdentifier::base_account(receiver),
        native_coin(),
        amount,
    ));
    Ok(operations)
}

/// Parses 0x1::primary_fungible_store::transfer(metadata: address, receiver: address, amount: u64)
/// or 0x1::aptos_account::transfer_fungible_assets(metadata: address, receiver: address, amount: u64)
fn parse_primary_fa_transfer_operation(
    server_context: &RosettaContext,
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    // There should be one type arg
    if type_args.len() != 1 {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Primary fungible store transfer should have one type argument: {:?}",
            type_args
        ))));
    }
    let mut operations = Vec::new();

    // Retrieve the args for the operations
    let metadata: AccountAddress = if let Some(metadata) = args.first() {
        bcs::from_bytes(metadata)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No metadata address in primary fungible transfer".to_string(),
        )));
    };
    let receiver: AccountAddress = if let Some(receiver) = args.get(1) {
        bcs::from_bytes(receiver)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No receiver address in primary fungible transfer".to_string(),
        )));
    };
    let amount: u64 = if let Some(amount) = args.get(2) {
        bcs::from_bytes(amount)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No amount in primary fungible transfer".to_string(),
        )));
    };

    // Grab currency accordingly

    let maybe_currency = find_fa_currency(&server_context.currencies, metadata);

    if let Some(currency) = maybe_currency {
        operations.push(Operation::withdraw(
            0,
            None,
            AccountIdentifier::base_account(sender),
            currency.clone(),
            amount,
        ));
        operations.push(Operation::deposit(
            1,
            None,
            AccountIdentifier::base_account(receiver),
            currency.clone(),
            amount,
        ));
        Ok(operations)
    } else {
        Err(ApiError::UnsupportedCurrency(Some(metadata.to_string())))
    }
}

/// Parses 0x1::fungible_asset::transfer(metadata: address, receiver: address, amount: u64)
///
/// This is only for using directly from a store, please prefer using primary fa.
fn parse_fa_transfer_operation(
    server_context: &RosettaContext,
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    // There is one type arg for the object
    if type_args.len() != 1 {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Fungible asset transfer should have one type argument: {:?}",
            type_args
        ))));
    }
    let mut operations = Vec::new();

    // Retrieve the args for the operations
    let metadata: AccountAddress = if let Some(metadata) = args.first() {
        bcs::from_bytes(metadata)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No metadata address in fungible asset transfer".to_string(),
        )));
    };
    let receiver: AccountAddress = if let Some(receiver) = args.get(1) {
        bcs::from_bytes(receiver)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No receiver address in fungible asset transfer".to_string(),
        )));
    };
    let amount: u64 = if let Some(amount) = args.get(2) {
        bcs::from_bytes(amount)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No amount in fungible transfer".to_string(),
        )));
    };

    // Grab currency accordingly

    let maybe_currency = find_fa_currency(&server_context.currencies, metadata);

    if let Some(currency) = maybe_currency {
        operations.push(Operation::withdraw(
            0,
            None,
            AccountIdentifier::base_account(sender),
            currency.clone(),
            amount,
        ));
        operations.push(Operation::deposit(
            1,
            None,
            AccountIdentifier::base_account(receiver),
            currency.clone(),
            amount,
        ));
        Ok(operations)
    } else {
        Err(ApiError::UnsupportedCurrency(Some(metadata.to_string())))
    }
}

/// Parses a specific BCS function argument to the given type
pub fn parse_function_arg<T: DeserializeOwned>(
    name: &str,
    args: &[Vec<u8>],
    index: usize,
) -> ApiResult<T> {
    if let Some(arg) = args.get(index) {
        if let Ok(arg) = bcs::from_bytes::<T>(arg) {
            return Ok(arg);
        }
    }

    Err(ApiError::InvalidInput(Some(format!(
        "Argument {} of {} failed to parse",
        index, name
    ))))
}

/// Parses 0x1::staking_contract::switch_operator_with_same_commission(old_operator: address, new_operator: address)
pub fn parse_set_operator_operation(
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Set operator should not have type arguments: {:?}",
            type_args
        ))));
    }

    let old_operator = parse_function_arg("set_operator", args, 0)?;
    let new_operator = parse_function_arg("set_operator", args, 1)?;
    Ok(vec![Operation::set_operator(
        0,
        None,
        sender,
        Some(AccountIdentifier::base_account(old_operator)),
        AccountIdentifier::base_account(new_operator),
        None,
    )])
}

/// Parses 0x1::staking_contract::update_voter(operator: address, new_voter: address)
pub fn parse_set_voter_operation(
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Set voter should not have type arguments: {:?}",
            type_args
        ))));
    }

    let operator = parse_function_arg("set_voter", args, 0)?;
    let new_voter = parse_function_arg("set_voter", args, 1)?;
    Ok(vec![Operation::set_voter(
        0,
        None,
        sender,
        Some(AccountIdentifier::base_account(operator)),
        AccountIdentifier::base_account(new_voter),
    )])
}

/// Parses 0x1::staking_contract::create_staking_contract(operator: address, voter: address, amount: u64, commission_percentage: u64)
pub fn parse_create_stake_pool_operation(
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Create stake pool should not have type arguments: {:?}",
            type_args
        ))));
    }

    let operator = parse_function_arg("create_stake_pool", args, 0)?;
    let voter = parse_function_arg("create_stake_pool", args, 1)?;
    let amount: u64 = parse_function_arg("create_stake_pool", args, 2)?;
    let commission_percentage: u64 = parse_function_arg("create_stake_pool", args, 3)?;
    Ok(vec![Operation::create_stake_pool(
        0,
        None,
        sender,
        Some(operator),
        Some(voter),
        Some(amount),
        Some(commission_percentage),
    )])
}

/// Parses 0x1::staking_contract::reset_lockup(operator: address)
pub fn parse_reset_lockup_operation(
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Reset lockup should not have type arguments: {:?}",
            type_args
        ))));
    }

    let operator: AccountAddress = parse_function_arg("reset_lockup", args, 0)?;
    Ok(vec![Operation::reset_lockup(
        0,
        None,
        sender,
        Some(AccountIdentifier::base_account(operator)),
    )])
}

/// Parses 0x1::staking_contract::unlock_stake(operator: address, amount: u64)
pub fn parse_unlock_stake_operation(
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Unlock stake should not have type arguments: {:?}",
            type_args
        ))));
    }

    let operator: AccountAddress = parse_function_arg("unlock_stake", args, 0)?;
    let amount: u64 = parse_function_arg("unlock_stake", args, 1)?;

    Ok(vec![Operation::unlock_stake(
        0,
        None,
        sender,
        Some(AccountIdentifier::base_account(operator)),
        Some(amount),
    )])
}

/// Parses 0x1::staking_contract::update_commission(operator: address, new_commission_percentage: u64)
pub fn parse_update_commission_operation(
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Unlock stake should not have type arguments: {:?}",
            type_args
        ))));
    }

    let operator: AccountAddress = parse_function_arg("update_commision", args, 0)?;
    let new_commission_percentage: u64 = parse_function_arg("update_commision", args, 1)?;

    Ok(vec![Operation::update_commission(
        0,
        None,
        sender,
        Some(AccountIdentifier::base_account(operator)),
        Some(new_commission_percentage),
    )])
}

/// Parses 0x1::staking_contract::distribute(staker: address, operator: address)
pub fn parse_distribute_staking_rewards_operation(
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Distribute should not have type arguments: {:?}",
            type_args
        ))));
    }

    let staker: AccountAddress = parse_function_arg("distribute_staking_rewards", args, 0)?;
    let operator: AccountAddress = parse_function_arg("distribute_staking_rewards", args, 1)?;

    Ok(vec![Operation::distribute_staking_rewards(
        0,
        None,
        sender,
        AccountIdentifier::base_account(operator),
        AccountIdentifier::base_account(staker),
    )])
}

/// Parses 0x1::delegation_pool::add_stake(pool_address: address, amount: u64)
pub fn parse_delegation_pool_add_stake_operation(
    delegator: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "add_delegated_stake should not have type arguments: {:?}",
            type_args
        ))));
    }

    let pool_address: AccountAddress = parse_function_arg("add_delegated_stake", args, 0)?;
    let amount: u64 = parse_function_arg("add_delegated_stake", args, 1)?;

    Ok(vec![Operation::add_delegated_stake(
        0,
        None,
        delegator,
        AccountIdentifier::base_account(pool_address),
        Some(amount),
    )])
}

/// Parses 0x1::delegation_pool::unlock(pool_address: address, amount: u64)
pub fn parse_delegation_pool_unlock_operation(
    delegator: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Unlock delegated stake should not have type arguments: {:?}",
            type_args
        ))));
    }

    let pool_address: AccountAddress = parse_function_arg("unlock_delegated_stake", args, 0)?;
    let amount: u64 = parse_function_arg("unlock_delegated_stake", args, 1)?;

    Ok(vec![Operation::unlock_delegated_stake(
        0,
        None,
        delegator,
        AccountIdentifier::base_account(pool_address),
        Some(amount),
    )])
}

/// Parses 0x1::delegation_pool::withdraw(pool_address: address, amount: u64)
pub fn parse_delegation_pool_withdraw_operation(
    delegator: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "add_delegated_stake should not have type arguments: {:?}",
            type_args
        ))));
    }

    let pool_address: AccountAddress = parse_function_arg("withdraw_undelegated", args, 0)?;
    let amount: u64 = parse_function_arg("withdraw_undelegated", args, 1)?;

    Ok(vec![Operation::withdraw_undelegated_stake(
        0,
        None,
        delegator,
        AccountIdentifier::base_account(pool_address),
        Some(amount),
    )])
}
