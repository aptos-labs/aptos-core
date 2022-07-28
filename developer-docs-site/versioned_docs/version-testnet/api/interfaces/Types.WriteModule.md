---
id: "Types.WriteModule"
title: "Interface: WriteModule"
sidebar_label: "WriteModule"
custom_edit_url: null
---

[Types](../namespaces/Types.md).WriteModule

Write move module

## Properties

### address

• **address**: `string`

Hex-encoded 16 bytes Aptos account address.

Prefixed with `0x` and leading zeros are trimmed.
See [doc](https://diem.github.io/move/address.html) for more details.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:757](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L757)

___

### data

• **data**: [`MoveModule`](Types.MoveModule.md)

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:758](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L758)

___

### state\_key\_hash

• **state\_key\_hash**: `string`

All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
two hex digits per byte.

Different with `Address` type, hex-encoded bytes should not trim any zeros.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:749](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L749)

___

### type

• **type**: `string`

**`example`** write_module

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:741](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L741)
