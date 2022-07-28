---
id: "TxnBuilderTypes.ModuleId"
title: "Class: ModuleId"
sidebar_label: "ModuleId"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).ModuleId

## Constructors

### constructor

• **new ModuleId**(`address`, `name`)

Full name of a module.

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `address` | [`AccountAddress`](TxnBuilderTypes.AccountAddress.md) | The account address. |
| `name` | [`Identifier`](TxnBuilderTypes.Identifier.md) | The name of the module under the account at "address". |

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:235](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L235)

## Properties

### address

• `Readonly` **address**: [`AccountAddress`](TxnBuilderTypes.AccountAddress.md)

___

### name

• `Readonly` **name**: [`Identifier`](TxnBuilderTypes.Identifier.md)

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:251](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L251)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`ModuleId`](TxnBuilderTypes.ModuleId.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`ModuleId`](TxnBuilderTypes.ModuleId.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:256](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L256)

___

### fromStr

▸ `Static` **fromStr**(`moduleId`): [`ModuleId`](TxnBuilderTypes.ModuleId.md)

Converts a string literal to a ModuleId

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `moduleId` | `string` | String literal in format "AcountAddress::ModuleName",   e.g. "0x01::Coin" |

#### Returns

[`ModuleId`](TxnBuilderTypes.ModuleId.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:243](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L243)
