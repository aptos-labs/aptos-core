---
id: "BCS.Deserializer"
title: "Class: Deserializer"
sidebar_label: "Deserializer"
custom_edit_url: null
---

[BCS](../namespaces/BCS.md).Deserializer

## Constructors

### constructor

• **new Deserializer**(`data`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `data` | `Uint8Array` |

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts:10](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts#L10)

## Properties

### buffer

• `Private` **buffer**: `ArrayBuffer`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts:6](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts#L6)

___

### offset

• `Private` **offset**: `number`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts:8](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts#L8)

## Methods

### deserializeBool

▸ **deserializeBool**(): `boolean`

Deserializes a boolean value.

BCS layout for "boolean": One byte. "0x01" for True and "0x00" for False.

#### Returns

`boolean`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts:71](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts#L71)

___

### deserializeBytes

▸ **deserializeBytes**(): `Uint8Array`

Deserializes an array of bytes.

BCS layout for "bytes": bytes_length | bytes. bytes_length is the length of the bytes array that is
uleb128 encoded. bytes_length is a u32 integer.

#### Returns

`Uint8Array`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts:53](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts#L53)

___

### deserializeFixedBytes

▸ **deserializeFixedBytes**(`len`): `Uint8Array`

Deserializes an array of bytes. The number of bytes to read is already known.

#### Parameters

| Name | Type |
| :------ | :------ |
| `len` | `number` |

#### Returns

`Uint8Array`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts:62](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts#L62)

___

### deserializeStr

▸ **deserializeStr**(): `string`

Deserializes a string. UTF8 string is supported. Reads the string's bytes length "l" first,
and then reads "l" bytes of content. Decodes the byte array into a string.

BCS layout for "string": string_length | string_content. string_length is the bytes length of
the string that is uleb128 encoded. string_length is a u32 integer.

**`example`**
```ts
const deserializer = new Deserializer(new Uint8Array([24, 0xc3, 0xa7, 0xc3, 0xa5, 0xe2, 0x88, 0x9e,
0xe2, 0x89, 0xa0, 0xc2, 0xa2, 0xc3, 0xb5, 0xc3, 0x9f, 0xe2, 0x88, 0x82, 0xc6, 0x92, 0xe2, 0x88, 0xab]));
assert(deserializer.deserializeStr() === "çå∞≠¢õß∂ƒ∫");
```

#### Returns

`string`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts:41](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts#L41)

___

### deserializeU128

▸ **deserializeU128**(): `bigint`

Deserializes a uint128 number.

BCS layout for "uint128": Sixteen bytes. Binary format in little-endian representation.

#### Returns

`bigint`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts:139](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts#L139)

___

### deserializeU16

▸ **deserializeU16**(): `number`

Deserializes a uint16 number.

BCS layout for "uint16": Two bytes. Binary format in little-endian representation.

**`example`**
```ts
const deserializer = new Deserializer(new Uint8Array([0x34, 0x12]));
assert(deserializer.deserializeU16() === 4660);
```

#### Returns

`number`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts:98](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts#L98)

___

### deserializeU32

▸ **deserializeU32**(): `number`

Deserializes a uint32 number.

BCS layout for "uint32": Four bytes. Binary format in little-endian representation.

**`example`**
```ts
const deserializer = new Deserializer(new Uint8Array([0x78, 0x56, 0x34, 0x12]));
assert(deserializer.deserializeU32() === 305419896);
```

#### Returns

`number`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts:112](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts#L112)

___

### deserializeU64

▸ **deserializeU64**(): `bigint`

Deserializes a uint64 number.

BCS layout for "uint64": Eight bytes. Binary format in little-endian representation.

**`example`**
```ts
const deserializer = new Deserializer(new Uint8Array([0x00, 0xEF, 0xCD, 0xAB, 0x78, 0x56, 0x34, 0x12]));
assert(deserializer.deserializeU64() === 1311768467750121216);
```

#### Returns

`bigint`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts:126](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts#L126)

___

### deserializeU8

▸ **deserializeU8**(): `number`

Deserializes a uint8 number.

BCS layout for "uint8": One byte. Binary format in little-endian representation.

#### Returns

`number`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts:84](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts#L84)

___

### deserializeUleb128AsU32

▸ **deserializeUleb128AsU32**(): `number`

Deserializes a uleb128 encoded uint32 number.

BCS use uleb128 encoding in two cases: (1) lengths of variable-length sequences and (2) tags of enum values

#### Returns

`number`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts:152](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts#L152)

___

### read

▸ `Private` **read**(`length`): `ArrayBuffer`

#### Parameters

| Name | Type |
| :------ | :------ |
| `length` | `number` |

#### Returns

`ArrayBuffer`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts:17](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/deserializer.ts#L17)
