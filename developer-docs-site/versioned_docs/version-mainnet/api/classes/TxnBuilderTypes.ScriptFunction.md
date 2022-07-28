---
id: "TxnBuilderTypes.ScriptFunction"
title: "Class: ScriptFunction"
sidebar_label: "ScriptFunction"
custom_edit_url: null
---

[TxnBuilderTypes](../namespaces/TxnBuilderTypes.md).ScriptFunction

## Constructors

### constructor

• **new ScriptFunction**(`module_name`, `function_name`, `ty_args`, `args`)

Contains the payload to run a function within a module.

**`example`**
A coin transfer function has one type argument "CoinType".
```
public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
```

**`example`**
A coin transfer function has three arugments "from", "to" and "amount".
```
public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
```

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `module_name` | [`ModuleId`](TxnBuilderTypes.ModuleId.md) | Fullly qualified module name. ModuleId consists of account address and module name. |
| `function_name` | [`Identifier`](TxnBuilderTypes.Identifier.md) | The function to run. |
| `ty_args` | [`Seq`](../namespaces/BCS.md#seq)<[`TypeTag`](TxnBuilderTypes.TypeTag.md)\> | Type arguments that move function requires. |
| `args` | [`Seq`](../namespaces/BCS.md#seq)<`Uint8Array`\> | Arugments to the move function. |

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:137](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L137)

## Properties

### args

• `Readonly` **args**: [`Seq`](../namespaces/BCS.md#seq)<`Uint8Array`\>

___

### function\_name

• `Readonly` **function\_name**: [`Identifier`](TxnBuilderTypes.Identifier.md)

___

### module\_name

• `Readonly` **module\_name**: [`ModuleId`](TxnBuilderTypes.ModuleId.md)

___

### ty\_args

• `Readonly` **ty\_args**: [`Seq`](../namespaces/BCS.md#seq)<[`TypeTag`](TxnBuilderTypes.TypeTag.md)\>

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

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:168](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L168)

___

### deserialize

▸ `Static` **deserialize**(`deserializer`): [`ScriptFunction`](TxnBuilderTypes.ScriptFunction.md)

#### Parameters

| Name | Type |
| :------ | :------ |
| `deserializer` | [`Deserializer`](BCS.Deserializer.md) |

#### Returns

[`ScriptFunction`](TxnBuilderTypes.ScriptFunction.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:179](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L179)

___

### natual

▸ `Static` **natual**(`module`, `func`, `ty_args`, `args`): [`ScriptFunction`](TxnBuilderTypes.ScriptFunction.md)

**`example`**
A coin transfer function has one type argument "CoinType".
```
public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
```

**`example`**
A coin transfer function has three arugments "from", "to" and "amount".
```
public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
```

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `module` | `string` | Fully qualified module name in format "AccountAddress::ModuleName" e.g. "0x1::Coin" |
| `func` | `string` | Function name |
| `ty_args` | [`Seq`](../namespaces/BCS.md#seq)<[`TypeTag`](TxnBuilderTypes.TypeTag.md)\> | Type arguments that move function requires. |
| `args` | [`Seq`](../namespaces/BCS.md#seq)<`Uint8Array`\> | Arugments to the move function. |

#### Returns

[`ScriptFunction`](TxnBuilderTypes.ScriptFunction.md)

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts:164](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/aptos_types/transaction.ts#L164)
