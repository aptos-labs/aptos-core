---
id: "HexString"
title: "Class: HexString"
sidebar_label: "HexString"
sidebar_position: 0
custom_edit_url: null
---

A util class for working with hex strings.
Hex strings are strings that are prefixed with `0x`

## Constructors

### constructor

• **new HexString**(`hexString`)

Creates new HexString instance from regular string. If specified string already starts with "0x" prefix,
it will not add another one

**`example`**
```
 const string = "string";
 new HexString(string); // "0xstring"
```

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `hexString` | `string` | String to convert |

#### Defined in

[ecosystem/typescript/sdk/src/hex_string.ts:62](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/hex_string.ts#L62)

## Properties

### hexString

• `Private` `Readonly` **hexString**: `string`

#### Defined in

[ecosystem/typescript/sdk/src/hex_string.ts:13](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/hex_string.ts#L13)

## Methods

### hex

▸ **hex**(): `string`

Getter for inner hexString

#### Returns

`string`

Inner hex string

#### Defined in

[ecosystem/typescript/sdk/src/hex_string.ts:74](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/hex_string.ts#L74)

___

### noPrefix

▸ **noPrefix**(): `string`

Getter for inner hexString without prefix

**`example`**
```
 const hexString = new HexString("string"); // "0xstring"
 hexString.noPrefix(); // "string"
```

#### Returns

`string`

Inner hex string without prefix

#### Defined in

[ecosystem/typescript/sdk/src/hex_string.ts:87](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/hex_string.ts#L87)

___

### toBuffer

▸ **toBuffer**(): `Buffer`

Converts hex string to a Buffer in hex encoding

#### Returns

`Buffer`

Buffer from inner hexString without prefix

#### Defined in

[ecosystem/typescript/sdk/src/hex_string.ts:116](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/hex_string.ts#L116)

___

### toShortString

▸ **toShortString**(): `string`

Trimmes extra zeroes in the begining of a string

**`example`**
```
 new HexString("0x000000string").toShortString(); // result = "0xstring"
```

#### Returns

`string`

Inner hexString without leading zeroes

#### Defined in

[ecosystem/typescript/sdk/src/hex_string.ts:107](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/hex_string.ts#L107)

___

### toString

▸ **toString**(): `string`

Overrides default `toString` method

#### Returns

`string`

Inner hex string

#### Defined in

[ecosystem/typescript/sdk/src/hex_string.ts:95](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/hex_string.ts#L95)

___

### toUint8Array

▸ **toUint8Array**(): `Uint8Array`

Converts hex string to a Uint8Array

#### Returns

`Uint8Array`

Uint8Array from inner hexString without prefix

#### Defined in

[ecosystem/typescript/sdk/src/hex_string.ts:124](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/hex_string.ts#L124)

___

### ensure

▸ `Static` **ensure**(`hexString`): [`HexString`](HexString.md)

Ensures `hexString` is instance of `HexString` class

**`example`**
```
 const regularString = "string";
 const hexString = new HexString("string"); // "0xstring"
 HexString.ensure(regularString); // "0xstring"
 HexString.ensure(hexString); // "0xstring"
```

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `hexString` | [`MaybeHexString`](../modules.md#maybehexstring) | String to check |

#### Returns

[`HexString`](HexString.md)

New HexString if `hexString` is regular string or `hexString` if it is HexString instance

#### Defined in

[ecosystem/typescript/sdk/src/hex_string.ts:45](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/hex_string.ts#L45)

___

### fromBuffer

▸ `Static` **fromBuffer**(`buffer`): [`HexString`](HexString.md)

Creates new hex string from Buffer

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `buffer` | `Buffer` | A buffer to convert |

#### Returns

[`HexString`](HexString.md)

New HexString

#### Defined in

[ecosystem/typescript/sdk/src/hex_string.ts:20](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/hex_string.ts#L20)

___

### fromUint8Array

▸ `Static` **fromUint8Array**(`arr`): [`HexString`](HexString.md)

Creates new hex string from Uint8Array

#### Parameters

| Name | Type | Description |
| :------ | :------ | :------ |
| `arr` | `Uint8Array` | Uint8Array to convert |

#### Returns

[`HexString`](HexString.md)

New HexString

#### Defined in

[ecosystem/typescript/sdk/src/hex_string.ts:29](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/hex_string.ts#L29)
