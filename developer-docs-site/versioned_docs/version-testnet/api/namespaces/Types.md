---
id: "Types"
title: "Namespace: Types"
sidebar_label: "Types"
sidebar_position: 0
custom_edit_url: null
---

## Enumerations

- [MoveAbility](../enums/Types.MoveAbility.md)

## Interfaces

- [Account](../interfaces/Types.Account.md)
- [AccountResource](../interfaces/Types.AccountResource.md)
- [AptosError](../interfaces/Types.AptosError.md)
- [DeleteModule](../interfaces/Types.DeleteModule.md)
- [DeleteResource](../interfaces/Types.DeleteResource.md)
- [DeleteTableItem](../interfaces/Types.DeleteTableItem.md)
- [DirectWriteSet](../interfaces/Types.DirectWriteSet.md)
- [Ed25519Signature](../interfaces/Types.Ed25519Signature.md)
- [Event](../interfaces/Types.Event.md)
- [LedgerInfo](../interfaces/Types.LedgerInfo.md)
- [ModuleBundlePayload](../interfaces/Types.ModuleBundlePayload.md)
- [MoveFunction](../interfaces/Types.MoveFunction.md)
- [MoveModule](../interfaces/Types.MoveModule.md)
- [MoveModuleABI](../interfaces/Types.MoveModuleABI.md)
- [MoveScript](../interfaces/Types.MoveScript.md)
- [MoveStruct](../interfaces/Types.MoveStruct.md)
- [MoveStructField](../interfaces/Types.MoveStructField.md)
- [MultiAgentSignature](../interfaces/Types.MultiAgentSignature.md)
- [MultiEd25519Signature](../interfaces/Types.MultiEd25519Signature.md)
- [OnChainTransactionInfo](../interfaces/Types.OnChainTransactionInfo.md)
- [Script](../interfaces/Types.Script.md)
- [ScriptFunctionPayload](../interfaces/Types.ScriptFunctionPayload.md)
- [ScriptPayload](../interfaces/Types.ScriptPayload.md)
- [ScriptWriteSet](../interfaces/Types.ScriptWriteSet.md)
- [TableItemRequest](../interfaces/Types.TableItemRequest.md)
- [Token](../interfaces/Types.Token.md)
- [TokenData](../interfaces/Types.TokenData.md)
- [TokenId](../interfaces/Types.TokenId.md)
- [UserTransactionRequest](../interfaces/Types.UserTransactionRequest.md)
- [UserTransactionSignature](../interfaces/Types.UserTransactionSignature.md)
- [WriteModule](../interfaces/Types.WriteModule.md)
- [WriteResource](../interfaces/Types.WriteResource.md)
- [WriteSetPayload](../interfaces/Types.WriteSetPayload.md)
- [WriteTableItem](../interfaces/Types.WriteTableItem.md)

## Type Aliases

### AccountSignature

Ƭ **AccountSignature**: [`Ed25519Signature`](../interfaces/Types.Ed25519Signature.md) \| [`MultiEd25519Signature`](../interfaces/Types.MultiEd25519Signature.md)

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:1032](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L1032)

___

### Address

Ƭ **Address**: `string`

Hex-encoded 16 bytes Aptos account address.

Prefixed with `0x` and leading zeros are trimmed.

See [doc](https://diem.github.io/move/address.html) for more details.

**`format`** address

**`example`** 0xdd

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:39](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L39)

___

### BlockMetadataTransaction

Ƭ **BlockMetadataTransaction**: { `id`: [`HexEncodedBytes`](Types.md#hexencodedbytes) ; `previous_block_votes`: [`Address`](Types.md#address)[] ; `proposer`: [`Address`](Types.md#address) ; `round`: [`Uint64`](Types.md#uint64) ; `timestamp`: [`TimestampUsec`](Types.md#timestampusec) ; `type`: `string`  } & [`OnChainTransactionInfo`](../interfaces/Types.OnChainTransactionInfo.md)

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:547](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L547)

___

### EventKey

Ƭ **EventKey**: `string`

Event key is a global index for an event stream.

It is hex-encoded BCS bytes of `EventHandle` `guid` field value, which is
a combination of a `uint64` creation number and account address
(without trimming leading zeros).

For example, event key `0x00000000000000000000000000000000000000000a550c18`
is combined by the following 2 parts:
1. `0000000000000000`: `uint64` representation of `0`.
2. `0000000000000000000000000a550c18`: 16 bytes of account address.

**`format`** hex

**`example`** 0x00000000000000000000000000000000000000000a550c18

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:86](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L86)

___

### EventSequenceNumber

Ƭ **EventSequenceNumber**: `string`

Event `sequence_number` is unique id of an event in an event stream.
Event `sequence_number` starts from 0 for each event key.

**`format`** uint64

**`example`** 23

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:94](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L94)

___

### GenesisTransaction

Ƭ **GenesisTransaction**: { `events`: [`Event`](../interfaces/Types.Event.md)[] ; `payload`: [`WriteSetPayload`](../interfaces/Types.WriteSetPayload.md) ; `type`: `string`  } & [`OnChainTransactionInfo`](../interfaces/Types.OnChainTransactionInfo.md)

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:556](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L556)

___

### HexEncodedBytes

Ƭ **HexEncodedBytes**: `string`

All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
two hex digits per byte.

Different with `Address` type, hex-encoded bytes should not trim any zeros.

**`format`** hex

**`example`** 0x88fbd33f54e1126269769780feb24480428179f552e2313fbe571b72e62a1ca1

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:49](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L49)

___

### LedgerVersion

Ƭ **LedgerVersion**: `string`

The version of the latest transaction in the ledger.

**`format`** uint64

**`example`** 52635485

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:70](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L70)

___

### MoveModuleId

Ƭ **MoveModuleId**: `string`

Move module id is a string representation of Move module.

Format: "{address}::{module name}"

`address` should be hex-encoded 16 bytes account address
that is prefixed with `0x` and leading zeros are trimmed.

Module name is case-sensitive.

See [doc](https://diem.github.io/move/modules-and-scripts.html#modules) for more details.

**`example`** 0x1::Aptos

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:431](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L431)

___

### MoveStructTagId

Ƭ **MoveStructTagId**: `string`

String representation of an on-chain Move struct type.

It is a combination of:
1. `Move module address`, `module name` and `struct name` joined by `::`.
2. `struct generic type parameters` joined by `, `.

Examples:
`0x1::Aptos::Aptos<0x1::XDX::XDX>`
`0x1::Abc::Abc<vector<u8>, vector<u64> />`
`0x1::AptosAccount::AccountOperationsCapability`

Note:
1. Empty chars should be ignored when comparing 2 struct tag ids.
2. When used in an URL path, should be encoded by url-encoding (AKA percent-encoding).

See [doc](https://diem.github.io/move/structs-and-resources.html) for more details.

**`format`** move_type

**`pattern`** ^0x[0-9a-zA-Z:_< />]+$

**`example`** 0x1::AptosAccount::Balance<0x1::XUS::XUS>

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:283](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L283)

___

### MoveTypeId

Ƭ **MoveTypeId**: `string`

String representation of an on-chain Move type identifier defined by the Move language.

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
`vector<u8>`
`vector<vector<u64>>`
`vector<0x1::AptosAccount::Balance<0x1::XDX::XDX>>`

Struct type value examples:
`0x1::Aptos::Aptos<0x1::XDX::XDX>`
`0x1::Abc::Abc<vector<u8>, vector<u64>>`
`0x1::AptosAccount::AccountOperationsCapability`

Reference type value examples:
`&signer`
`&mut address`
`&mut vector<u8>`

Generic type parameter value example, the following is `0x1::TransactionFee::TransactionFee` JSON representation:

{
"name": "TransactionFee",
"is_native": false,
"abilities": ["key"],
"generic_type_params": [
{"constraints": [], "is_phantom": true}
],
"fields": [
{ "name": "balance", "type": "0x1::Aptos::Aptos<T0 />" },
{ "name": "preburn", "type": "0x1::Aptos::Preburn<T0 />" }
]
}

It's Move source code:

module AptosFramework::TransactionFee {
struct TransactionFee<phantom CoinType /> has key {
balance: Aptos<CoinType />,
preburn: Preburn<CoinType />,
}
}

The `T0` in the above JSON representation is the generic type place holder for
the `CoinType` in the Move source code.

Note:
1. Empty chars should be ignored when comparing 2 struct tag ids.
2. When used in an URL path, should be encoded by url-encoding (AKA percent-encoding).

**`pattern`** ^(bool|u8|u64|u128|address|signer|vector<.+>|0x[0-9a-zA-Z:_<, >]+|^&(mut )?.+$|T\d+)$

**`example`** 0x1::AptosAccount::Balance<0x1::XUS::XUS>

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:260](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L260)

___

### MoveTypeTagId

Ƭ **MoveTypeTagId**: `string`

String representation of an on-chain Move type tag that is exposed in transaction payload.

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
`vector<u8>`
`vector<vector<u64>>`
`vector<0x1::AptosAccount::Balance<0x1::XDX::XDX>>`

Struct type value examples:
`0x1::Aptos::Aptos<0x1::XDX::XDX>`
`0x1::Abc::Abc<vector<u8>, vector<u64>>`
`0x1::AptosAccount::AccountOperationsCapability`

Note:
1. Empty chars should be ignored when comparing 2 struct tag ids.
2. When used in an URL path, should be encoded by url-encoding (AKA percent-encoding).

**`pattern`** ^(bool|u8|u64|u128|address|signer|vector<.+>|0x[0-9a-zA-Z:_<, >]+)$

**`example`** 0x1::XUS::XUS

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:193](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L193)

___

### MoveValue

Ƭ **MoveValue**: `any`

Move `bool` type value is serialized into `boolean`.

Move `u8` type value is serialized into `integer`.

Move `u64` and `u128` type value is serialized into `string`.

Move `address` type value(16 bytes Aptos account address) is serialized into
hex-encoded string, which is prefixed with `0x` and leading zeros are trimmed.

For example:
`0x1`
`0x1668f6be25668c1a17cd8caf6b8d2f25`

Move `vector` type value is serialized into `array`, except `vector<u8>` which is
serialized into hex-encoded string with `0x` prefix.

For example:
`vector<u64>{255, 255}` => `["255", "255"]`
`vector<u8>{255, 255}` => `0xffff`

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

*Special serialization for Move stdlib types:**

[0x1::ASCII::String](https://github.com/aptos-labs/aptos-core/blob/main/language/move-stdlib/docs/ASCII.md) is serialized into `string`. For example, struct value `0x1::ASCII::String{bytes: b"hello world"}` is serialized as `"hello world"` in JSON.

**`example`** 3344000000

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:884](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L884)

___

### OnChainTransaction

Ƭ **OnChainTransaction**: [`GenesisTransaction`](Types.md#genesistransaction) \| [`UserTransaction`](Types.md#usertransaction) \| [`BlockMetadataTransaction`](Types.md#blockmetadatatransaction) \| [`StateCheckpointTransaction`](Types.md#statecheckpointtransaction)

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:483](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L483)

___

### PendingTransaction

Ƭ **PendingTransaction**: { `hash`: [`HexEncodedBytes`](Types.md#hexencodedbytes) ; `type`: `string`  } & [`UserTransactionRequest`](../interfaces/Types.UserTransactionRequest.md) & [`UserTransactionSignature`](../interfaces/Types.UserTransactionSignature.md)

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:480](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L480)

___

### ScriptFunctionId

Ƭ **ScriptFunctionId**: `string`

Script function id is string representation of a script function defined on-chain.

Format: `{address}::{module name}::{function name}`

Both `module name` and `function name` are case-sensitive.

**`example`** 0x1::PaymentScripts::peer_to_peer_with_metadata

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:591](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L591)

___

### StateCheckpointTransaction

Ƭ **StateCheckpointTransaction**: { `timestamp`: [`TimestampUsec`](Types.md#timestampusec) ; `type`: `string`  } & [`OnChainTransactionInfo`](../interfaces/Types.OnChainTransactionInfo.md)

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:558](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L558)

___

### SubmitTransactionRequest

Ƭ **SubmitTransactionRequest**: [`UserTransactionRequest`](../interfaces/Types.UserTransactionRequest.md) & [`UserTransactionSignature`](../interfaces/Types.UserTransactionSignature.md)

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:478](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L478)

___

### TimestampSec

Ƭ **TimestampSec**: `string`

Timestamp in seconds, e.g. transaction expiration timestamp.

**`format`** uint64

**`example`** 1635447454

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:56](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L56)

___

### TimestampUsec

Ƭ **TimestampUsec**: `string`

Timestamp in microseconds, e.g. ledger / block creation timestamp.

**`format`** uint64

**`example`** 1632507671675208

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:63](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L63)

___

### Transaction

Ƭ **Transaction**: [`PendingTransaction`](Types.md#pendingtransaction) \| [`GenesisTransaction`](Types.md#genesistransaction) \| [`UserTransaction`](Types.md#usertransaction) \| [`BlockMetadataTransaction`](Types.md#blockmetadatatransaction) \| [`StateCheckpointTransaction`](Types.md#statecheckpointtransaction)

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:471](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L471)

___

### TransactionPayload

Ƭ **TransactionPayload**: [`ScriptFunctionPayload`](../interfaces/Types.ScriptFunctionPayload.md) \| [`ScriptPayload`](../interfaces/Types.ScriptPayload.md) \| [`ModuleBundlePayload`](../interfaces/Types.ModuleBundlePayload.md) \| [`WriteSetPayload`](../interfaces/Types.WriteSetPayload.md)

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:560](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L560)

___

### TransactionSignature

Ƭ **TransactionSignature**: [`Ed25519Signature`](../interfaces/Types.Ed25519Signature.md) \| [`MultiEd25519Signature`](../interfaces/Types.MultiEd25519Signature.md) \| [`MultiAgentSignature`](../interfaces/Types.MultiAgentSignature.md)

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:971](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L971)

___

### Uint64

Ƭ **Uint64**: `string`

Unsigned int64 type value

**`format`** uint64

**`example`** 32425224034

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:28](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L28)

___

### UserCreateSigningMessageRequest

Ƭ **UserCreateSigningMessageRequest**: [`UserTransactionRequest`](../interfaces/Types.UserTransactionRequest.md) & { `secondary_signers?`: [`Address`](Types.md#address)[]  }

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:462](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L462)

___

### UserTransaction

Ƭ **UserTransaction**: { `events`: [`Event`](../interfaces/Types.Event.md)[] ; `timestamp`: [`TimestampUsec`](Types.md#timestampusec) ; `type`: `string`  } & [`UserTransactionRequest`](../interfaces/Types.UserTransactionRequest.md) & [`UserTransactionSignature`](../interfaces/Types.UserTransactionSignature.md) & [`OnChainTransactionInfo`](../interfaces/Types.OnChainTransactionInfo.md)

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:543](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L543)

___

### WriteSet

Ƭ **WriteSet**: [`ScriptWriteSet`](../interfaces/Types.ScriptWriteSet.md) \| [`DirectWriteSet`](../interfaces/Types.DirectWriteSet.md)

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:613](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L613)

___

### WriteSetChange

Ƭ **WriteSetChange**: [`DeleteModule`](../interfaces/Types.DeleteModule.md) \| [`DeleteResource`](../interfaces/Types.DeleteResource.md) \| [`DeleteTableItem`](../interfaces/Types.DeleteTableItem.md) \| [`WriteModule`](../interfaces/Types.WriteModule.md) \| [`WriteResource`](../interfaces/Types.WriteResource.md) \| [`WriteTableItem`](../interfaces/Types.WriteTableItem.md)

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:636](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L636)
