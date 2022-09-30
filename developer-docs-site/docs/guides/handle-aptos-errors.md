---
title: "Handling Errors in Aptos and Move"
slug: "handle-aptos-errors"
---
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Handling Errors in Aptos and Move

This page catalogs common errors encountered in the Aptos blockchain and explains how to resolve them wherever possible. As with all software, the code itself is the source of truth for error handling and will always contain entries not found here. Instead, this matrix aims to help you address those errors most typically found, misunderstood, or both.

For the sources of these errors, see:

  * [vm_status.rs](https://github.com/move-language/move/blob/main/language/move-core/types/src/vm_status.rs)
  * [error.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/move-stdlib/sources/error.move)
  * [account.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/account.move)
  * [coin.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/coin.move)
  * [token.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/sources/token.move)
  * [token_transfers.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/sources/token_transfers.move)

Help us update this list by sending pull requests containing the errors you encounter. If you don't know how to resolve the error, as described int the *Action* column, simply leave it blank.

## Move Virtual Machine (VM)

|Error |Meaning  |
--- | :---: |
|UNKNOWN_VALIDATION_STATUS|We don't want the default value to be valid.|
|INVALID_SIGNATURE|The transaction has a bad signature.|
|INVALID_AUTH_KEY|Bad account authentication key.|
|SEQUENCE_NUMBER_TOO_OLD|Sequence number is too old.|
|SEQUENCE_NUMBER_TOO_NEW|Sequence number is too new.|
|INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE|Insufficient balance to pay minimum transaction fee.|
|TRANSACTION_EXPIRED|The transaction has expired.|
|SENDING_ACCOUNT_DOES_NOT_EXIST|The sending account does not exist.|
|REJECTED_WRITE_SET|This write set transaction was rejected because it did not meet the requirements for one.|
|INVALID_WRITE_SET|This write set transaction cannot be applied to the current state.|
|EXCEEDED_MAX_TRANSACTION_SIZE|ength of program field in raw transaction exceeded max length.|
|UNKNOWN_SCRIPT|This script is not in our allowlist of scripts.|
|UNKNOWN_MODULE|Transaction is trying to publish a new module.|
|MAX_GAS_UNITS_EXCEEDS_MAX_GAS_UNITS_BOUND|Max gas units submitted with transaction exceeds max gas units bound in VM.|
|MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS|Max gas units submitted with transaction not enough to cover the intrinsic cost of the transaction.|
|GAS_UNIT_PRICE_BELOW_MIN_BOUND|Gas unit price submitted with transaction is below minimum gas price set in the VM.|
|GAS_UNIT_PRICE_ABOVE_MAX_BOUND|Gas unit price submitted with the transaction is above the maximum gas price set in the VM.|
|INVALID_GAS_SPECIFIER|Gas specifier submitted is either malformed (not a valid identifier), or does not refer to an accepted gas specifier.|
|SENDING_ACCOUNT_FROZEN|The sending account is frozen.|
|UNABLE_TO_DESERIALIZE_ACCOUNT|Unable to deserialize the account blob.|
|CURRENCY_INFO_DOES_NOT_EXIST|The currency info was unable to be found.|
|INVALID_MODULE_PUBLISHER|The account sender doesn't have permissions to publish modules.|
|NO_ACCOUNT_ROLE|The sending account has no role.|
|BAD_CHAIN_ID|The transaction's chain_id does not match the one published on-chain.|
|SEQUENCE_NUMBER_TOO_BIG|The sequence number is too large and would overflow if the transaction were executed.|
|BAD_TRANSACTION_FEE_CURRENCY|The gas currency is not registered as a TransactionFee currency.|
|FEATURE_UNDER_GATING|The feature requested is intended for a future Diem version instead of the current one.|
|SECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH|The number of secondary signer addresses is different from the number of secondary public keys provided.|
|SIGNERS_CONTAIN_DUPLICATES|There are duplicates among signers, including the sender and all the secondary signers.|
|SEQUENCE_NONCE_INVALID|The sequence nonce in the transaction is invalid (too new, too old, or already used).|
|CHAIN_ACCOUNT_INFO_DOES_NOT_EXIST|There was an error when accessing chain-specific account information.|
|MODULE_ADDRESS_DOES_NOT_MATCH_SENDER|he self address of a module the transaction is publishing is not the sender address.|
|ZERO_SIZED_STRUCT|Reported when a struct has zero fields.|
|DUPLICATE_MODULE_NAME|The sender is trying to publish two modules with the same name in one transaction.|
|BACKWARD_INCOMPATIBLE_MODULE_UPDATE|The sender is trying to publish a module that breaks the compatibility checks.|
|CYCLIC_MODULE_DEPENDENCY|The updated module introduces a cyclic dependency (i.e., A uses B and B also uses A).|
|INVALID_FRIEND_DECL_WITH_SELF|Cannot mark the module itself as a friend.|
|INVALID_FRIEND_DECL_WITH_MODULES_OUTSIDE_ACCOUNT_ADDRESS|Cannot declare modules outside of account address as friends.|
|INVALID_FRIEND_DECL_WITH_MODULES_IN_DEPENDENCIES|Cannot declare modules that this module depends on as friends.|
|CYCLIC_MODULE_FRIENDSHIP|The updated module introduces a cyclic friendship (i.e., A friends B and B also friends A).|
|INVALID_PHANTOM_TYPE_PARAM_POSITION|A phantom type parameter was used in a non-phantom position.|
|LOOP_MAX_DEPTH_REACHED|Loops are too deeply nested.|
|TYPE_RESOLUTION_FAILURE|Failed to resolve type due to linking being broken after verification.|
|RESOURCE_DOES_NOT_EXIST|We tried to access a resource that does not exist under the account.|
|RESOURCE_ALREADY_EXISTS|We tried to create a resource under an account where that resource already exists.|
|UNKNOWN_STATUS|A reserved status to represent an unknown vm status. This is std::u64::MAX, but we can't pattern match on that, so put the hardcoded value in.|
|
|

## Move Standard Library (stdlib)

|Error |Meaning  |
--- | :---: |
|INVALID_ARGUMENT|Caller specified an invalid argument (http: 400).|
|OUT_OF_RANGE|An input or result of a computation is out of range (http: 400).|
|INVALID_STATE|The system is not in a state where the operation can be performed (http: 400).|
|UNAUTHENTICATED|Request not authenticated due to missing, invalid, or expired auth token (http: 401).|
|PERMISSION_DENIED|The client does not have sufficient permission (http: 403).|
|NOT_FOUND|A specified resource is not found (http: 404).|
|ABORTED|Concurrency conflict, such as read-modify-write conflict (http: 409).|
|ALREADY_EXISTS|The resource that a client tried to create already exists (http: 409).|
|RESOURCE_EXHAUSTED|Out of gas or other forms of quota (http: 429).|
|CANCELLED|Request cancelled by the client (http: 499).|
|INTERNAL|Internal error (http: 500).|
|NOT_IMPLEMENTED|Feature not implemented (http: 501).|
|UNAVAILABLE|The service is currently unavailable. Indicates that a retry could solve the issue (http: 503).|

## Aptos accounts

|Error |Meaning  |
--- | :---: |
|EACCOUNT_ALREADY_EXISTS|Account already exists.|
|EACCOUNT_DOES_NOT_EXIST|Account does not exist.|
|ESEQUENCE_NUMBER_TOO_BIG|Sequence number exceeds the maximum value for a u64.|
|EMALFORMED_AUTHENTICATION_KEY|The provided authentication key has an invalid length.|
|ECANNOT_RESERVED_ADDRESS|Cannot create account because address is reserved.|
|EOUT_OF_GAS|Transaction exceeded its allocated max gas.|
|EWRONG_CURRENT_PUBLIC_KEY|Specified current public key is not correct.|
|EINVALID_PROOF_OF_KNOWLEDGE|Specified proof of knowledge required to prove ownership of a public key is invalid.|
|ENO_CAPABILITY|The caller does not have a digital-signature-based capability to call this function.|
|EINVALID_ACCEPT_ROTATION_CAPABILITY|The caller does not have a valid rotation capability offer from the other account.|
|ENO_VALID_FRAMEWORK_RESERVED_ADDRESS|Address to create is not a valid reserved address for Aptos framework.|
|EINVALID_SCHEME|Specified scheme required to proceed with the smart contract operation - can only be ED25519_SCHEME(0) OR MULTI_ED25519_SCHEME(1).|
|EINVALID_ORIGINATING_ADDRESS|Abort the transaction if the expected originating address is different from the originating address on-chain.|
|ENO_SUCH_SIGNER_CAPABILITY|The signer capability doesn't exist at the given address.|

## Aptos coins

|Error |Meaning  |
--- | :---: |
|ECOIN_INFO_ADDRESS_MISMATCH|Address of account which is used to initialize a coin `CoinType` doesn't match the deployer of module.|
|ECOIN_INFO_ALREADY_PUBLISHED|`CoinType` is already initialized as a coin.|
|ECOIN_INFO_NOT_PUBLISHED|`CoinType` hasn't been initialized as a coin.|
|ECOIN_STORE_ALREADY_PUBLISHED|Account already has `CoinStore` registered for `CoinType`.|
|ECOIN_STORE_NOT_PUBLISHED|Account hasn't registered `CoinStore` for `CoinType`.|
|EINSUFFICIENT_BALANCE|Not enough coins to complete transaction.|
|EDESTRUCTION_OF_NONZERO_TOKEN|Cannot destroy non-zero coins.|
|EZERO_COIN_AMOUNT|Coin amount cannot be zero.|
|EFROZEN|CoinStore is frozen. Coins cannot be deposited or withdrawn.|
|ECOIN_SUPPLY_UPGRADE_NOT_SUPPORTED|Cannot upgrade the total supply of coins to different implementation.|
|ECOIN_NAME_TOO_LONG|Name of the coin is too long.|
|ECOIN_SYMBOL_TOO_LONG|Symbol of the coin is too long.|

## Aptos tokens

|Error |Meaning  |
--- | :---: |
|EALREADY_HAS_BALANCE|The token has balance and cannot be initialized.|
|ECOLLECTIONS_NOT_PUBLISHED|There isn't any collection under this account.|
|ECOLLECTION_NOT_PUBLISHED|Cannot find collection in creator's account.|
|ECOLLECTION_ALREADY_EXISTS|The collection already exists.|
|ECREATE_WOULD_EXCEED_COLLECTION_MAXIMUM|Exceeds the collection's maximal number of token_data.|
|EINSUFFICIENT_BALANCE|Insufficient token balance.|
|EINVALID_TOKEN_MERGE|Cannot merge the two tokens with different token IDs.|
|EMINT_WOULD_EXCEED_TOKEN_MAXIMUM|xceed the token data maximal allowed.|
|ENO_BURN_CAPABILITY|No burn capability.|
|ETOKEN_DATA_ALREADY_EXISTS|TokenData already exists.|
|ETOKEN_DATA_NOT_PUBLISHED|TokenData not published.|
|ETOKEN_STORE_NOT_PUBLISHED|TokenStore doesn't exist.|
|ETOKEN_SPLIT_AMOUNT_LARGER_THAN_TOKEN_AMOUNT|Cannot split token to an amount larger than its amount.|
|EFIELD_NOT_MUTABLE|The field is not mutable.|
|ENO_MUTATE_CAPABILITY|Not authorized to mutate.|
|ENO_TOKEN_IN_TOKEN_STORE|Token not in the token store.|
|EUSER_NOT_OPT_IN_DIRECT_TRANSFER|User didn't opt-in direct transfer.|
|EWITHDRAW_ZERO|Cannot withdraw 0 token.|
|ENFT_NOT_SPLITABLE|Cannot split a token that only has 1 amount.|
|ENO_MINT_CAPABILITY|No mint capability|
|ECOLLECTION_NAME_TOO_LONG|The collection name is too long.|
|ENFT_NAME_TOO_LONG|The NFT name is too long.|
|EURI_TOO_LONG|The URI is too long.|
|ENO_DEPOSIT_TOKEN_WITH_ZERO_AMOUNT|Cannot deposit a token with 0 amount.|
|ENO_BURN_TOKEN_WITH_ZERO_AMOUNT|Cannot burn 0 token.|
|EWITHDRAW_PROOF_EXPIRES|Withdraw proof expires.|
|EOWNER_CANNOT_BURN_TOKEN|Token is not burnable by owner.|
|ECREATOR_CANNOT_BURN_TOKEN|Token is not burnable by creator.|
|ECANNOT_UPDATE_RESERVED_PROPERTY|Reserved fields for token contract. Cannot be updated by user.|
|EURI_TOO_SHORT|URI too short.|
|ETOKEN_OFFER_NOT_EXIST|Token offer doesn't exist.|
