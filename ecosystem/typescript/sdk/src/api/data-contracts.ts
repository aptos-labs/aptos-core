/* eslint-disable */
/* tslint:disable */
/*
 * ---------------------------------------------------------------
 * ## THIS FILE WAS GENERATED VIA SWAGGER-TYPESCRIPT-API        ##
 * ##                                                           ##
 * ## AUTHOR: acacode                                           ##
 * ## SOURCE: https://github.com/acacode/swagger-typescript-api ##
 * ---------------------------------------------------------------
 */

export interface AptosError {
  code: number;
  message: string;

  /**
   * The version of the latest transaction in the ledger.
   *
   */
  aptos_ledger_version?: LedgerVersion;
}

/**
 * Unsigned int64 type value
 * @format uint64
 * @example 32425224034
 */
export type Uint64 = string;

/**
* Hex-encoded 16 bytes Aptos account address.

Prefixed with `0x` and leading zeros are trimmed.

See [doc](https://diem.github.io/move/address.html) for more details.
* @format address
* @example 0xdd
*/
export type Address = string;

/**
* All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
two hex digits per byte.

Different with `Address` type, hex-encoded bytes should not trim any zeros.
* @format hex
* @example 0x88fbd33f54e1126269769780feb24480428179f552e2313fbe571b72e62a1ca1
*/
export type HexEncodedBytes = string;

/**
 * Timestamp in seconds, e.g. transaction expiration timestamp.
 * @format uint64
 * @example 1635447454
 */
export type TimestampSec = string;

/**
 * Timestamp in microseconds, e.g. ledger / block creation timestamp.
 * @format uint64
 * @example 1632507671675208
 */
export type TimestampUsec = string;

/**
 * The version of the latest transaction in the ledger.
 * @format uint64
 * @example 52635485
 */
export type LedgerVersion = string;

/**
* Event key is a global index for an event stream.

It is hex-encoded BCS bytes of `EventHandle` `guid` field value, which is
a combination of a `uint64` creation number and account address
(without trimming leading zeros).

For example, event key `0x00000000000000000000000000000000000000000a550c18`
is combined by the following 2 parts:
  1. `0000000000000000`: `uint64` representation of `0`.
  2. `0000000000000000000000000a550c18`: 16 bytes of account address.
* @format hex
* @example 0x00000000000000000000000000000000000000000a550c18
*/
export type EventKey = string;

/**
* Event `sequence_number` is unique id of an event in an event stream.
Event `sequence_number` starts from 0 for each event key.
* @format uint64
* @example 23
*/
export type EventSequenceNumber = string;

export interface LedgerInfo {
  /**
   * The blockchain chain id.
   *
   * @example 4
   */
  chain_id: number;

  /**
   * The version of the latest transaction in the ledger.
   *
   */
  ledger_version: LedgerVersion;

  /**
   * Timestamp in microseconds, e.g. ledger / block creation timestamp.
   *
   */
  ledger_timestamp: TimestampUsec;
}

/**
 * Core account resource, used for identifying account and transaction execution.
 * @example {"sequence_number":"1","authentication_key":"0x5307b5f4bc67829097a8ba9b43dba3b88261eeccd1f709d9bde240fc100fbb69"}
 */
export interface Account {
  /** Unsigned int64 type value */
  sequence_number: Uint64;

  /**
   * All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
   * two hex digits per byte.
   *
   * Different with `Address` type, hex-encoded bytes should not trim any zeros.
   */
  authentication_key: HexEncodedBytes;
}

/**
 * Account resource is a Move struct value belongs to an account.
 * @example {"type":"0x1::AptosAccount::Balance<0x1::XDX::XDX>","data":{"coin":{"value":"8000000000"}}}
 */
export interface AccountResource {
  /**
   * String representation of an on-chain Move struct type.
   *
   * It is a combination of:
   *   1. `Move module address`, `module name` and `struct name` joined by `::`.
   *   2. `struct generic type parameters` joined by `, `.
   * Examples:
   *   * `0x1::Aptos::Aptos<0x1::XDX::XDX>`
   *   * `0x1::Abc::Abc<vector<u8>, vector<u64>>`
   *   * `0x1::AptosAccount::AccountOperationsCapability`
   * Note:
   *   1. Empty chars should be ignored when comparing 2 struct tag ids.
   *   2. When used in an URL path, should be encoded by url-encoding (AKA percent-encoding).
   * See [doc](https://diem.github.io/move/structs-and-resources.html) for more details.
   */
  type: MoveStructTagId;

  /**
   * Account resource data is JSON representation of the Move struct `type`.
   *
   * Move struct field name and value are serialized as object property name and value.
   */
  data: object;
}

/**
* String representation of an on-chain Move type tag that is exposed in transaction payload.

Values:
  - bool
  - u8
  - u64
  - u128
  - address
  - signer
  - vector: `vector<{non-reference MoveTypeId}>`
  - struct: `{address}::{module_name}::{struct_name}::<{generic types}>`

Vector type value examples:
  * `vector<u8>`
  * `vector<vector<u64>>`
  * `vector<0x1::AptosAccount::Balance<0x1::XDX::XDX>>`

Struct type value examples:
  * `0x1::Aptos::Aptos<0x1::XDX::XDX>`
  * `0x1::Abc::Abc<vector<u8>, vector<u64>>`
  * `0x1::AptosAccount::AccountOperationsCapability`

Note:
  1. Empty chars should be ignored when comparing 2 struct tag ids.
  2. When used in an URL path, should be encoded by url-encoding (AKA percent-encoding).
* @pattern ^(bool|u8|u64|u128|address|signer|vector<.+>|0x[0-9a-zA-Z:_<, >]+)$
* @example 0x1::XUS::XUS
*/
export type MoveTypeTagId = string;

/**
* String representation of an on-chain Move type identifier defined by the Move language.

Values:
  - bool
  - u8
  - u64
  - u128
  - address
  - signer
  - vector: `vector<{non-reference MoveTypeId}>`
  - struct: `{address}::{module_name}::{struct_name}::<{generic types}>`
  - reference: immutable `&` and mutable `&mut` references.
  - generic_type_parameter: it is always start with `T` and following an index number,
    which is the position of the generic type parameter in the `struct` or
    `function` generic type parameters definition.

Vector type value examples:
  * `vector<u8>`
  * `vector<vector<u64>>`
  * `vector<0x1::AptosAccount::Balance<0x1::XDX::XDX>>`

Struct type value examples:
  * `0x1::Aptos::Aptos<0x1::XDX::XDX>`
  * `0x1::Abc::Abc<vector<u8>, vector<u64>>`
  * `0x1::AptosAccount::AccountOperationsCapability`

Reference type value examples:
  * `&signer`
  * `&mut address`
  * `&mut vector<u8>`

Generic type parameter value example, the following is `0x1::TransactionFee::TransactionFee` JSON representation:

    {
        "name": "TransactionFee",
        "is_native": false,
        "abilities": ["key"],
        "generic_type_params": [
            {"constraints": [], "is_phantom": true}
        ],
        "fields": [
            { "name": "balance", "type": "0x1::Aptos::Aptos<T0>" },
            { "name": "preburn", "type": "0x1::Aptos::Preburn<T0>" }
        ]
    }

It's Move source code:

    module AptosFramework::TransactionFee {
        struct TransactionFee<phantom CoinType> has key {
            balance: Aptos<CoinType>,
            preburn: Preburn<CoinType>,
        }
    }

The `T0` in the above JSON representation is the generic type place holder for
the `CoinType` in the Move source code.

Note:
  1. Empty chars should be ignored when comparing 2 struct tag ids.
  2. When used in an URL path, should be encoded by url-encoding (AKA percent-encoding).
* @pattern ^(bool|u8|u64|u128|address|signer|vector<.+>|0x[0-9a-zA-Z:_<, >]+|^&(mut )?.+$|T\d+)$
* @example 0x1::AptosAccount::Balance<0x1::XUS::XUS>
*/
export type MoveTypeId = string;

/**
* String representation of an on-chain Move struct type.

It is a combination of:
  1. `Move module address`, `module name` and `struct name` joined by `::`.
  2. `struct generic type parameters` joined by `, `.

Examples:
  * `0x1::Aptos::Aptos<0x1::XDX::XDX>`
  * `0x1::Abc::Abc<vector<u8>, vector<u64>>`
  * `0x1::AptosAccount::AccountOperationsCapability`

Note:
  1. Empty chars should be ignored when comparing 2 struct tag ids.
  2. When used in an URL path, should be encoded by url-encoding (AKA percent-encoding).

See [doc](https://diem.github.io/move/structs-and-resources.html) for more details.
* @format move_type
* @pattern ^0x[0-9a-zA-Z:_<>]+$
* @example 0x1::AptosAccount::Balance<0x1::XUS::XUS>
*/
export type MoveStructTagId = string;

export interface MoveModule {
  /**
   * All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
   * two hex digits per byte.
   *
   * Different with `Address` type, hex-encoded bytes should not trim any zeros.
   */
  bytecode: HexEncodedBytes;

  /**
   * Move Module ABI is JSON representation of Move module binary interface.
   *
   */
  abi?: MoveModuleABI;
}

/**
 * Move Module ABI is JSON representation of Move module binary interface.
 */
export interface MoveModuleABI {
  /**
   * Hex-encoded 16 bytes Aptos account address.
   *
   * Prefixed with `0x` and leading zeros are trimmed.
   * See [doc](https://diem.github.io/move/address.html) for more details.
   */
  address: Address;

  /** @example Aptos */
  name: string;
  friends: MoveModuleId[];
  exposed_functions: MoveFunction[];
  structs: MoveStruct[];
}

/**
 * @example {"name":"Balance","is_native":false,"abilities":["key"],"generic_type_params":[{"constraints":[],"is_phantom":true}],"fields":[{"name":"coin","type":"0x1::Aptos::Aptos<T0>"}]}
 */
export interface MoveStruct {
  name: string;
  is_native: boolean;
  abilities: MoveAbility[];
  generic_type_params: { constraints: MoveAbility[]; is_phantom: boolean }[];
  fields: MoveStructField[];
}

/**
 * @example {"name":"value","type":"u64"}
 */
export interface MoveStructField {
  name: string;

  /**
   * String representation of an on-chain Move type identifier defined by the Move language.
   *
   * Values:
   *   - bool
   *   - u8
   *   - u64
   *   - u128
   *   - address
   *   - signer
   *   - vector: `vector<{non-reference MoveTypeId}>`
   *   - struct: `{address}::{module_name}::{struct_name}::<{generic types}>`
   *   - reference: immutable `&` and mutable `&mut` references.
   *   - generic_type_parameter: it is always start with `T` and following an index number,
   *     which is the position of the generic type parameter in the `struct` or
   *     `function` generic type parameters definition.
   * Vector type value examples:
   *   * `vector<u8>`
   *   * `vector<vector<u64>>`
   *   * `vector<0x1::AptosAccount::Balance<0x1::XDX::XDX>>`
   * Struct type value examples:
   *   * `0x1::Aptos::Aptos<0x1::XDX::XDX>`
   *   * `0x1::Abc::Abc<vector<u8>, vector<u64>>`
   *   * `0x1::AptosAccount::AccountOperationsCapability`
   * Reference type value examples:
   *   * `&signer`
   *   * `&mut address`
   *   * `&mut vector<u8>`
   * Generic type parameter value example, the following is `0x1::TransactionFee::TransactionFee` JSON representation:
   *     {
   *         "name": "TransactionFee",
   *         "is_native": false,
   *         "abilities": ["key"],
   *         "generic_type_params": [
   *             {"constraints": [], "is_phantom": true}
   *         ],
   *         "fields": [
   *             { "name": "balance", "type": "0x1::Aptos::Aptos<T0>" },
   *             { "name": "preburn", "type": "0x1::Aptos::Preburn<T0>" }
   *         ]
   *     }
   * It's Move source code:
   *     module AptosFramework::TransactionFee {
   *         struct TransactionFee<phantom CoinType> has key {
   *             balance: Aptos<CoinType>,
   *             preburn: Preburn<CoinType>,
   *         }
   * The `T0` in the above JSON representation is the generic type place holder for
   * the `CoinType` in the Move source code.
   * Note:
   *   1. Empty chars should be ignored when comparing 2 struct tag ids.
   *   2. When used in an URL path, should be encoded by url-encoding (AKA percent-encoding).
   */
  type: MoveTypeId;
}

/**
 * @example {"name":"peer_to_peer_with_metadata","visibility":"script","generic_type_params":[{"constraints":[]}],"params":["signer","address","u64","vector<u8>","vector<u8>"],"return":[]}
 */
export interface MoveFunction {
  /** Move function name */
  name: string;
  visibility: "public" | "script" | "friend";
  generic_type_params: { constraints: MoveAbility[] }[];
  params: MoveTypeId[];
  return: MoveTypeId[];
}

/**
* Abilities are a typing feature in Move that control what actions are permissible for values of a given type.

See [doc](https://diem.github.io/move/abilities.html) for more details.
* @example key
*/
export enum MoveAbility {
  Copy = "copy",
  Drop = "drop",
  Store = "store",
  Key = "key",
}

/**
* Move module id is a string representation of Move module.

Format: "{address}::{module name}"

`address` should be hex-encoded 16 bytes account address
that is prefixed with `0x` and leading zeros are trimmed.

Module name is case-sensitive.

See [doc](https://diem.github.io/move/modules-and-scripts.html#modules) for more details.
* @example 0x1::Aptos
*/
export type MoveModuleId = string;

export interface UserTransactionRequest {
  /**
   * Hex-encoded 16 bytes Aptos account address.
   *
   * Prefixed with `0x` and leading zeros are trimmed.
   * See [doc](https://diem.github.io/move/address.html) for more details.
   */
  sender: Address;

  /** Unsigned int64 type value */
  sequence_number: Uint64;

  /** Unsigned int64 type value */
  max_gas_amount: Uint64;

  /** Unsigned int64 type value */
  gas_unit_price: Uint64;

  /** @example XDX */
  gas_currency_code: string;

  /**
   * Timestamp in seconds, e.g. transaction expiration timestamp.
   *
   */
  expiration_timestamp_secs: TimestampSec;
  payload: TransactionPayload;
}

/**
 * This schema is used for appending `signature` field to another schema.
 */
export interface UserTransactionSignature {
  signature: TransactionSignature;
}

export type Transaction = PendingTransaction | GenesisTransaction | UserTransaction | BlockMetadataTransaction;

export type SubmitTransactionRequest = UserTransactionRequest & UserTransactionSignature;

export type PendingTransaction = { type: string; hash: HexEncodedBytes } & UserTransactionRequest &
  UserTransactionSignature;

export type OnChainTransaction = GenesisTransaction | UserTransaction | BlockMetadataTransaction;

export interface OnChainTransactionInfo {
  /** Unsigned int64 type value */
  version: Uint64;

  /**
   * All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
   * two hex digits per byte.
   *
   * Different with `Address` type, hex-encoded bytes should not trim any zeros.
   */
  hash: HexEncodedBytes;

  /**
   * All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
   * two hex digits per byte.
   *
   * Different with `Address` type, hex-encoded bytes should not trim any zeros.
   */
  state_root_hash: HexEncodedBytes;

  /**
   * All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
   * two hex digits per byte.
   *
   * Different with `Address` type, hex-encoded bytes should not trim any zeros.
   */
  event_root_hash: HexEncodedBytes;

  /** Unsigned int64 type value */
  gas_used: Uint64;

  /**
   * Transaction execution result (success: true, failure: false).
   * See `vm_status` for human readable error message from Aptos VM.
   *
   */
  success: boolean;

  /**
   * Human readable transaction execution result message from Aptos VM.
   *
   */
  vm_status: string;

  /**
   * All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
   * two hex digits per byte.
   *
   * Different with `Address` type, hex-encoded bytes should not trim any zeros.
   */
  accumulator_root_hash: HexEncodedBytes;
  changes: WriteSetChange[];
}

export type UserTransaction = { type: string; events: Event[]; timestamp: TimestampUsec } & UserTransactionRequest &
  UserTransactionSignature &
  OnChainTransactionInfo;

export type BlockMetadataTransaction = {
  type: string;
  id: HexEncodedBytes;
  round: Uint64;
  previous_block_votes: Address[];
  proposer: Address;
  timestamp: TimestampUsec;
} & OnChainTransactionInfo;

export type GenesisTransaction = { type: string; events: Event[]; payload: WriteSetPayload } & OnChainTransactionInfo;

export type TransactionPayload = ScriptFunctionPayload | ScriptPayload | ModuleBundlePayload | WriteSetPayload;

/**
 * @example {"type":"script_function_payload","function":"0x1::PaymentScripts::peer_to_peer_with_metadata","type_arguments":["0x1::XDX::XDX"],"arguments":["0x1668f6be25668c1a17cd8caf6b8d2f25","2021000000","0x","0x"]}
 */
export interface ScriptFunctionPayload {
  type: string;

  /**
   * Script function id is string representation of a script function defined on-chain.
   *
   * Format: `{address}::{module name}::{function name}`
   * Both `module name` and `function name` are case-sensitive.
   */
  function: ScriptFunctionId;

  /** Generic type arguments required by the script function. */
  type_arguments: MoveTypeTagId[];

  /** The script function arguments. */
  arguments: MoveValue[];
}

/**
* Script function id is string representation of a script function defined on-chain.

Format: `{address}::{module name}::{function name}`

Both `module name` and `function name` are case-sensitive.
* @example 0x1::PaymentScripts::peer_to_peer_with_metadata
*/
export type ScriptFunctionId = string;

export interface ScriptPayload {
  /** @example script_payload */
  type: string;
  code: MoveScript;
  type_arguments: MoveTypeTagId[];
  arguments: MoveValue[];
}

export interface ModuleBundlePayload {
  /** @example module_bundle_payload */
  type: string;
  modules: MoveModule[];
}

export interface WriteSetPayload {
  /** @example write_set_payload */
  type: string;
  write_set: WriteSet;
}

export type WriteSet = ScriptWriteSet | DirectWriteSet;

export interface ScriptWriteSet {
  /** @example script_write_set */
  type: string;

  /**
   * Hex-encoded 16 bytes Aptos account address.
   *
   * Prefixed with `0x` and leading zeros are trimmed.
   * See [doc](https://diem.github.io/move/address.html) for more details.
   */
  execute_as: Address;
  script: Script;
}

export interface DirectWriteSet {
  /** @example direct_write_set */
  type: string;
  changes: WriteSetChange[];
  events: Event[];
}

export type WriteSetChange =
  | DeleteModule
  | DeleteResource
  | DeleteTableItem
  | WriteModule
  | WriteResource
  | WriteTableItem;

export interface DeleteModule {
  /** @example delete_module */
  type: string;

  /**
   * All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
   * two hex digits per byte.
   *
   * Different with `Address` type, hex-encoded bytes should not trim any zeros.
   */
  state_key_hash: HexEncodedBytes;

  /**
   * Hex-encoded 16 bytes Aptos account address.
   *
   * Prefixed with `0x` and leading zeros are trimmed.
   * See [doc](https://diem.github.io/move/address.html) for more details.
   */
  address: Address;

  /**
   * Move module id is a string representation of Move module.
   *
   * Format: "{address}::{module name}"
   * `address` should be hex-encoded 16 bytes account address
   * that is prefixed with `0x` and leading zeros are trimmed.
   * Module name is case-sensitive.
   * See [doc](https://diem.github.io/move/modules-and-scripts.html#modules) for more details.
   */
  module: MoveModuleId;
}

/**
 * Delete account resource change.
 */
export interface DeleteResource {
  /** @example delete_resource */
  type: string;

  /**
   * All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
   * two hex digits per byte.
   *
   * Different with `Address` type, hex-encoded bytes should not trim any zeros.
   */
  state_key_hash: HexEncodedBytes;

  /**
   * Hex-encoded 16 bytes Aptos account address.
   *
   * Prefixed with `0x` and leading zeros are trimmed.
   * See [doc](https://diem.github.io/move/address.html) for more details.
   */
  address: Address;

  /**
   * String representation of an on-chain Move struct type.
   *
   * It is a combination of:
   *   1. `Move module address`, `module name` and `struct name` joined by `::`.
   *   2. `struct generic type parameters` joined by `, `.
   * Examples:
   *   * `0x1::Aptos::Aptos<0x1::XDX::XDX>`
   *   * `0x1::Abc::Abc<vector<u8>, vector<u64>>`
   *   * `0x1::AptosAccount::AccountOperationsCapability`
   * Note:
   *   1. Empty chars should be ignored when comparing 2 struct tag ids.
   *   2. When used in an URL path, should be encoded by url-encoding (AKA percent-encoding).
   * See [doc](https://diem.github.io/move/structs-and-resources.html) for more details.
   */
  resource: MoveStructTagId;
}

/**
 * Delete table item change.
 */
export interface DeleteTableItem {
  /** @example delete_table_item */
  type: string;

  /**
   * All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
   * two hex digits per byte.
   *
   * Different with `Address` type, hex-encoded bytes should not trim any zeros.
   */
  state_key_hash: HexEncodedBytes;

  /** Table item deletion */
  data: { handle: HexEncodedBytes; key: HexEncodedBytes };
}

/**
 * Write move module
 */
export interface WriteModule {
  /** @example write_module */
  type: string;

  /**
   * All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
   * two hex digits per byte.
   *
   * Different with `Address` type, hex-encoded bytes should not trim any zeros.
   */
  state_key_hash: HexEncodedBytes;

  /**
   * Hex-encoded 16 bytes Aptos account address.
   *
   * Prefixed with `0x` and leading zeros are trimmed.
   * See [doc](https://diem.github.io/move/address.html) for more details.
   */
  address: Address;
  data: MoveModule;
}

/**
 * Write account resource
 */
export interface WriteResource {
  /** @example write_resource */
  type: string;

  /**
   * All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
   * two hex digits per byte.
   *
   * Different with `Address` type, hex-encoded bytes should not trim any zeros.
   */
  state_key_hash: HexEncodedBytes;

  /**
   * Hex-encoded 16 bytes Aptos account address.
   *
   * Prefixed with `0x` and leading zeros are trimmed.
   * See [doc](https://diem.github.io/move/address.html) for more details.
   */
  address: Address;

  /** Account resource is a Move struct value belongs to an account. */
  data: AccountResource;
}

/**
 * Write table item
 */
export interface WriteTableItem {
  /** @example write_table_item */
  type: string;

  /**
   * All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
   * two hex digits per byte.
   *
   * Different with `Address` type, hex-encoded bytes should not trim any zeros.
   */
  state_key_hash: HexEncodedBytes;

  /** Table item write */
  data: { handle: HexEncodedBytes; key: HexEncodedBytes; value: HexEncodedBytes };
}

export interface Script {
  code: MoveScript;
  type_arguments: MoveTypeTagId[];
  arguments: MoveValue[];
}

export interface MoveScript {
  /**
   * All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
   * two hex digits per byte.
   *
   * Different with `Address` type, hex-encoded bytes should not trim any zeros.
   */
  bytecode: HexEncodedBytes;
  abi?: MoveFunction;
}

/**
* Move `bool` type value is serialized into `boolean`.

Move `u8` type value is serialized into `integer`.

Move `u64` and `u128` type value is serialized into `string`.

Move `address` type value(16 bytes Aptos account address) is serialized into
hex-encoded string, which is prefixed with `0x` and leading zeros are trimmed.

For example:
  * `0x1`
  * `0x1668f6be25668c1a17cd8caf6b8d2f25`

Move `vector` type value is serialized into `array`, except `vector<u8>` which is
serialized into hex-encoded string with `0x` prefix.

For example:
  * `vector<u64>{255, 255}` => `["255", "255"]`
  * `vector<u8>{255, 255}` => `0xffff`

Move `struct` type value is serialized into `object` that looks like this (except some Move stdlib types, see the following section):

  ```json
  {
    field1_name: field1_value,
    field2_name: field2_value,
    ......
  }
  ```

For example:
  `{ "created": "0xa550c18", "role_id": "0" }`

**Special serialization for Move stdlib types:**

* [0x1::ASCII::String](https://github.com/aptos-labs/aptos-core/blob/main/language/move-stdlib/docs/ASCII.md) is serialized into `string`. For example, struct value `0x1::ASCII::String{bytes: b"hello world"}` is serialized as `"hello world"` in JSON.
* @example 3344000000
*/
export type MoveValue = any;

/**
* Event `key` and `sequence_number` are global identifier of the event.

Event `sequence_number` starts from 0 for each event key.

Event `type` is the type information of the event `data`, you can use the `type`
to decode the `data` JSON.
* @example {"key":"0x00000000000000000000000000000000000000000a550c18","sequence_number":"23","type":"0x1::AptosAccount::CreateAccountEvent","data":{"created":"0xa550c18","role_id":"0"}}
*/
export interface Event {
  /**
   * Event key is a global index for an event stream.
   *
   * It is hex-encoded BCS bytes of `EventHandle` `guid` field value, which is
   * a combination of a `uint64` creation number and account address
   * (without trimming leading zeros).
   * For example, event key `0x00000000000000000000000000000000000000000a550c18`
   * is combined by the following 2 parts:
   *   1. `0000000000000000`: `uint64` representation of `0`.
   *   2. `0000000000000000000000000a550c18`: 16 bytes of account address.
   */
  key: EventKey;

  /**
   * Event `sequence_number` is unique id of an event in an event stream.
   * Event `sequence_number` starts from 0 for each event key.
   *
   */
  sequence_number: EventSequenceNumber;

  /**
   * String representation of an on-chain Move type tag that is exposed in transaction payload.
   *
   * Values:
   *   - bool
   *   - u8
   *   - u64
   *   - u128
   *   - address
   *   - signer
   *   - vector: `vector<{non-reference MoveTypeId}>`
   *   - struct: `{address}::{module_name}::{struct_name}::<{generic types}>`
   * Vector type value examples:
   *   * `vector<u8>`
   *   * `vector<vector<u64>>`
   *   * `vector<0x1::AptosAccount::Balance<0x1::XDX::XDX>>`
   * Struct type value examples:
   *   * `0x1::Aptos::Aptos<0x1::XDX::XDX>`
   *   * `0x1::Abc::Abc<vector<u8>, vector<u64>>`
   *   * `0x1::AptosAccount::AccountOperationsCapability`
   * Note:
   *   1. Empty chars should be ignored when comparing 2 struct tag ids.
   *   2. When used in an URL path, should be encoded by url-encoding (AKA percent-encoding).
   */
  type: MoveTypeTagId;

  /**
   * Move `bool` type value is serialized into `boolean`.
   *
   * Move `u8` type value is serialized into `integer`.
   * Move `u64` and `u128` type value is serialized into `string`.
   * Move `address` type value(16 bytes Aptos account address) is serialized into
   * hex-encoded string, which is prefixed with `0x` and leading zeros are trimmed.
   * For example:
   *   * `0x1`
   *   * `0x1668f6be25668c1a17cd8caf6b8d2f25`
   * Move `vector` type value is serialized into `array`, except `vector<u8>` which is
   * serialized into hex-encoded string with `0x` prefix.
   *   * `vector<u64>{255, 255}` => `["255", "255"]`
   *   * `vector<u8>{255, 255}` => `0xffff`
   * Move `struct` type value is serialized into `object` that looks like this (except some Move stdlib types, see the following section):
   *   ```json
   *   {
   *     field1_name: field1_value,
   *     field2_name: field2_value,
   *     ......
   *   }
   *   ```
   *   `{ "created": "0xa550c18", "role_id": "0" }`
   * **Special serialization for Move stdlib types:**
   * * [0x1::ASCII::String](https://github.com/aptos-labs/aptos-core/blob/main/language/move-stdlib/docs/ASCII.md) is serialized into `string`. For example, struct value `0x1::ASCII::String{bytes: b"hello world"}` is serialized as `"hello world"` in JSON.
   */
  data: MoveValue;
}

export type TransactionSignature = Ed25519Signature | MultiEd25519Signature | MultiAgentSignature;

/**
* Please refer to https://github.com/aptos-labs/aptos-core/tree/main/specifications/crypto#signature-and-verification for
more details.
*/
export interface Ed25519Signature {
  /** @example ed25519_signature */
  type: string;

  /**
   * All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
   * two hex digits per byte.
   *
   * Different with `Address` type, hex-encoded bytes should not trim any zeros.
   */
  public_key: HexEncodedBytes;

  /**
   * All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
   * two hex digits per byte.
   *
   * Different with `Address` type, hex-encoded bytes should not trim any zeros.
   */
  signature: HexEncodedBytes;
}

/**
 * Multi ed25519 signature, please refer to https://github.com/aptos-labs/aptos-core/tree/main/specifications/crypto#multi-signatures for more details.
 */
export interface MultiEd25519Signature {
  /** @example multi_ed25519_signature */
  type: string;

  /** all public keys of the sender account */
  public_keys: HexEncodedBytes[];

  /** signatures created based on the `threshold` */
  signatures: HexEncodedBytes[];

  /** The threshold of the multi ed25519 account key. */
  threshold: number;

  /**
   * All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
   * two hex digits per byte.
   *
   * Different with `Address` type, hex-encoded bytes should not trim any zeros.
   */
  bitmap: HexEncodedBytes;
}

/**
 * Multi agent signature, please refer to TBD.
 */
export interface MultiAgentSignature {
  /** @example multi_agent_signature */
  type: string;
  sender: AccountSignature;
  secondary_signer_addresses: Address[];
  secondary_signers: AccountSignature[];
}

export type AccountSignature = Ed25519Signature | MultiEd25519Signature;

export interface TableItemRequest {
  /**
   * String representation of an on-chain Move type identifier defined by the Move language.
   *
   * Values:
   *   - bool
   *   - u8
   *   - u64
   *   - u128
   *   - address
   *   - signer
   *   - vector: `vector<{non-reference MoveTypeId}>`
   *   - struct: `{address}::{module_name}::{struct_name}::<{generic types}>`
   *   - reference: immutable `&` and mutable `&mut` references.
   *   - generic_type_parameter: it is always start with `T` and following an index number,
   *     which is the position of the generic type parameter in the `struct` or
   *     `function` generic type parameters definition.
   * Vector type value examples:
   *   * `vector<u8>`
   *   * `vector<vector<u64>>`
   *   * `vector<0x1::AptosAccount::Balance<0x1::XDX::XDX>>`
   * Struct type value examples:
   *   * `0x1::Aptos::Aptos<0x1::XDX::XDX>`
   *   * `0x1::Abc::Abc<vector<u8>, vector<u64>>`
   *   * `0x1::AptosAccount::AccountOperationsCapability`
   * Reference type value examples:
   *   * `&signer`
   *   * `&mut address`
   *   * `&mut vector<u8>`
   * Generic type parameter value example, the following is `0x1::TransactionFee::TransactionFee` JSON representation:
   *     {
   *         "name": "TransactionFee",
   *         "is_native": false,
   *         "abilities": ["key"],
   *         "generic_type_params": [
   *             {"constraints": [], "is_phantom": true}
   *         ],
   *         "fields": [
   *             { "name": "balance", "type": "0x1::Aptos::Aptos<T0>" },
   *             { "name": "preburn", "type": "0x1::Aptos::Preburn<T0>" }
   *         ]
   *     }
   * It's Move source code:
   *     module AptosFramework::TransactionFee {
   *         struct TransactionFee<phantom CoinType> has key {
   *             balance: Aptos<CoinType>,
   *             preburn: Preburn<CoinType>,
   *         }
   * The `T0` in the above JSON representation is the generic type place holder for
   * the `CoinType` in the Move source code.
   * Note:
   *   1. Empty chars should be ignored when comparing 2 struct tag ids.
   *   2. When used in an URL path, should be encoded by url-encoding (AKA percent-encoding).
   */
  key_type: MoveTypeId;

  /**
   * String representation of an on-chain Move type identifier defined by the Move language.
   *
   * Values:
   *   - bool
   *   - u8
   *   - u64
   *   - u128
   *   - address
   *   - signer
   *   - vector: `vector<{non-reference MoveTypeId}>`
   *   - struct: `{address}::{module_name}::{struct_name}::<{generic types}>`
   *   - reference: immutable `&` and mutable `&mut` references.
   *   - generic_type_parameter: it is always start with `T` and following an index number,
   *     which is the position of the generic type parameter in the `struct` or
   *     `function` generic type parameters definition.
   * Vector type value examples:
   *   * `vector<u8>`
   *   * `vector<vector<u64>>`
   *   * `vector<0x1::AptosAccount::Balance<0x1::XDX::XDX>>`
   * Struct type value examples:
   *   * `0x1::Aptos::Aptos<0x1::XDX::XDX>`
   *   * `0x1::Abc::Abc<vector<u8>, vector<u64>>`
   *   * `0x1::AptosAccount::AccountOperationsCapability`
   * Reference type value examples:
   *   * `&signer`
   *   * `&mut address`
   *   * `&mut vector<u8>`
   * Generic type parameter value example, the following is `0x1::TransactionFee::TransactionFee` JSON representation:
   *     {
   *         "name": "TransactionFee",
   *         "is_native": false,
   *         "abilities": ["key"],
   *         "generic_type_params": [
   *             {"constraints": [], "is_phantom": true}
   *         ],
   *         "fields": [
   *             { "name": "balance", "type": "0x1::Aptos::Aptos<T0>" },
   *             { "name": "preburn", "type": "0x1::Aptos::Preburn<T0>" }
   *         ]
   *     }
   * It's Move source code:
   *     module AptosFramework::TransactionFee {
   *         struct TransactionFee<phantom CoinType> has key {
   *             balance: Aptos<CoinType>,
   *             preburn: Preburn<CoinType>,
   *         }
   * The `T0` in the above JSON representation is the generic type place holder for
   * the `CoinType` in the Move source code.
   * Note:
   *   1. Empty chars should be ignored when comparing 2 struct tag ids.
   *   2. When used in an URL path, should be encoded by url-encoding (AKA percent-encoding).
   */
  value_type: MoveTypeId;

  /**
   * Move `bool` type value is serialized into `boolean`.
   *
   * Move `u8` type value is serialized into `integer`.
   * Move `u64` and `u128` type value is serialized into `string`.
   * Move `address` type value(16 bytes Aptos account address) is serialized into
   * hex-encoded string, which is prefixed with `0x` and leading zeros are trimmed.
   * For example:
   *   * `0x1`
   *   * `0x1668f6be25668c1a17cd8caf6b8d2f25`
   * Move `vector` type value is serialized into `array`, except `vector<u8>` which is
   * serialized into hex-encoded string with `0x` prefix.
   *   * `vector<u64>{255, 255}` => `["255", "255"]`
   *   * `vector<u8>{255, 255}` => `0xffff`
   * Move `struct` type value is serialized into `object` that looks like this (except some Move stdlib types, see the following section):
   *   ```json
   *   {
   *     field1_name: field1_value,
   *     field2_name: field2_value,
   *     ......
   *   }
   *   ```
   *   `{ "created": "0xa550c18", "role_id": "0" }`
   * **Special serialization for Move stdlib types:**
   * * [0x1::ASCII::String](https://github.com/aptos-labs/aptos-core/blob/main/language/move-stdlib/docs/ASCII.md) is serialized into `string`. For example, struct value `0x1::ASCII::String{bytes: b"hello world"}` is serialized as `"hello world"` in JSON.
   */
  key: MoveValue;
}
