---
id: "TxnBuilderTypes.AccountAddress"
title: "Class: AccountAddress"
sidebar_label: "AccountAddress"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).AccountAddress

## Constructors

### constructor

• **new AccountAddress**(`address`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `address` | `Uint8Array` |

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/account_address.ts:9](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/account_address.ts#L9)

## Properties

### address

• `Readonly` **address**: `Uint8Array`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/account_address.ts:7](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/account_address.ts#L7)

___

### LENGTH

▪ `Static` `Readonly` **LENGTH**: `number` = `32`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/account_address.ts:5](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/account_address.ts#L5)

## Methods

### serialize

▸ **serialize**(`serializer`): `void`

#### Parameters

| Name | Type |
| :------ | :------ |
| `serializer` | [`Serializer`](BCS.Serializer.md) |

#### Returns

`void`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/account_address.ts:45](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/account_address.ts#L45)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`AccountAddress`](TxnBuilderTypes.AccountAddress.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`AccountAddress`](TxnBuilderTypes.AccountAddress.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/account_address.ts:49](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/account_address.ts#L49)

___

### fromHex

▸ `Static` **fromHex**(`addr`): [`AccountAddress`](TxnBuilderTypes.AccountAddress.md)

Creates AccountAddress from a hex string.

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `addr` | [`MaybeHexString`](../modules.md#maybehexstring) | Hex string can be with a prefix or without a prefix,   e.g. '0x1aa' or '1aa'. Hex string will be left padded with 0s if too short. |

#### Returns

[`AccountAddress`](TxnBuilderTypes.AccountAddress.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/account_address.ts:21](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/account_address.ts#L21)
