---
id: "BCS.Serializer"
title: "Class: Serializer"
sidebar_label: "Serializer"
custom_edit_url: null
---

[BCS](../namespaces/BCS.md).Serializer

## Constructors

### constructor

• **new Serializer**()

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts:10](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts#L10)

## Properties

### buffer

• `Private` **buffer**: `ArrayBuffer`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts:6](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts#L6)

___

### offset

• `Private` **offset**: `number`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts:8](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts#L8)

## Methods

### ensureBufferWillHandleSize

▸ `Private` **ensureBufferWillHandleSize**(`bytes`): `void`

#### Parameters

| Name | Type |
| :------ | :------ |
| `bytes` | `number` |

#### Returns

`void`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts:15](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts#L15)

___

### getBytes

▸ **getBytes**(): `Uint8Array`

Returns the buffered bytes

#### Returns

`Uint8Array`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts:191](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts#L191)

___

### serialize

▸ `Protected` **serialize**(`values`): `void`

#### Parameters

| Name | Type |
| :------ | :------ |
| `values` | `Uint8Array` |

#### Returns

`void`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts:23](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts#L23)

___

### serializeBool

▸ **serializeBool**(`value`): `void`

Serializes a boolean value.

BCS layout for "boolean": One byte. "0x01" for True and "0x00" for False.

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | `boolean` |

#### Returns

`void`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts:85](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts#L85)

___

### serializeBytes

▸ **serializeBytes**(`value`): `void`

Serializes an array of bytes.

BCS layout for "bytes": bytes_length | bytes. bytes_length is the length of the bytes array that is
uleb128 encoded. bytes_length is a u32 integer.

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | `Uint8Array` |

#### Returns

`void`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts:66](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts#L66)

___

### serializeFixedBytes

▸ **serializeFixedBytes**(`value`): `void`

Serializes an array of bytes with known length. Therefore length doesn't need to be
serialized to help deserialization.  When deserializing, the number of
bytes to deserialize needs to be passed in.

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | `Uint8Array` |

#### Returns

`void`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts:76](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts#L76)

___

### serializeStr

▸ **serializeStr**(`value`): `void`

Serializes a string. UTF8 string is supported. Serializes the string's bytes length "l" first,
and then serializes "l" bytes of the string content.

BCS layout for "string": string_length | string_content. string_length is the bytes length of
the string that is uleb128 encoded. string_length is a u32 integer.

**`example`**
```ts
const serializer = new Serializer();
serializer.serializeStr("çå∞≠¢õß∂ƒ∫");
assert(serializer.getBytes() === new Uint8Array([24, 0xc3, 0xa7, 0xc3, 0xa5, 0xe2, 0x88, 0x9e,
0xe2, 0x89, 0xa0, 0xc2, 0xa2, 0xc3, 0xb5, 0xc3, 0x9f, 0xe2, 0x88, 0x82, 0xc6, 0x92, 0xe2, 0x88, 0xab]));
```

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | `string` |

#### Returns

`void`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts:55](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts#L55)

___

### serializeU128

▸ **serializeU128**(`value`): `void`

Serializes a uint128 number.

BCS layout for "uint128": Sixteen bytes. Binary format in little-endian representation.

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | [`AnyNumber`](../namespaces/BCS.md#anynumber) |

#### Returns

`void`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts:162](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts#L162)

___

### serializeU16

▸ **serializeU16**(`value`): `void`

Serializes a uint16 number.

BCS layout for "uint16": Two bytes. Binary format in little-endian representation.

**`example`**
```ts
const serializer = new Serializer();
serializer.serializeU16(4660);
assert(serializer.getBytes() === new Uint8Array([0x34, 0x12]));
```

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | `number` |

#### Returns

`void`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts:115](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts#L115)

___

### serializeU32

▸ **serializeU32**(`value`): `void`

Serializes a uint32 number.

BCS layout for "uint32": Four bytes. Binary format in little-endian representation.

**`example`**
```ts
const serializer = new Serializer();
serializer.serializeU32(305419896);
assert(serializer.getBytes() === new Uint8Array([0x78, 0x56, 0x34, 0x12]));
```

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | `number` |

#### Returns

`void`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts:131](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts#L131)

___

### serializeU32AsUleb128

▸ **serializeU32AsUleb128**(`val`): `void`

Serializes a uint32 number with uleb128.

BCS use uleb128 encoding in two cases: (1) lengths of variable-length sequences and (2) tags of enum values

#### Parameters

| Name | Type |
| :------ | :------ |
| `val` | `number` |

#### Returns

`void`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts:177](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts#L177)

___

### serializeU64

▸ **serializeU64**(`value`): `void`

Serializes a uint64 number.

BCS layout for "uint64": Eight bytes. Binary format in little-endian representation.

**`example`**
```ts
const serializer = new Serializer();
serializer.serializeU64(1311768467750121216);
assert(serializer.getBytes() === new Uint8Array([0x00, 0xEF, 0xCD, 0xAB, 0x78, 0x56, 0x34, 0x12]));
```

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | [`AnyNumber`](../namespaces/BCS.md#anynumber) |

#### Returns

`void`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts:147](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts#L147)

___

### serializeU8

▸ **serializeU8**(`value`): `void`

Serializes a uint8 number.

BCS layout for "uint8": One byte. Binary format in little-endian representation.

#### Parameters

| Name | Type |
| :------ | :------ |
| `value` | `number` |

#### Returns

`void`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts:99](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts#L99)

___

### serializeWithFunction

▸ `Private` **serializeWithFunction**(`fn`, `bytesLength`, `value`): `void`

#### Parameters

| Name | Type |
| :------ | :------ |
| `fn` | (`byteOffset`: `number`, `value`: `number`, `littleEndian?`: `boolean`) => `void` |
| `bytesLength` | `number` |
| `value` | `number` |

#### Returns

`void`

#### Defined in

[ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts:29](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/transaction_builder/bcs/serializer.ts#L29)
