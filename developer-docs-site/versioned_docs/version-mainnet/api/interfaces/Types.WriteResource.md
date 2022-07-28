---
id: "Types.WriteResource"
title: "Interface: WriteResource"
sidebar_label: "WriteResource"
custom_edit_url: null
---

[Types](../namespaces/Types.md).WriteResource

Write account resource

## Properties

### address

• **address**: `string`

Hex-encoded 16 bytes Aptos account address.

Prefixed with `0x` and leading zeros are trimmed.
See [doc](https://diem.github.io/move/address.html) for more details.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:782](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L782)

___

### data

• **data**: [`AccountResource`](Types.AccountResource.md)

Account resource is a Move struct value belongs to an account.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:785](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L785)

___

### state\_key\_hash

• **state\_key\_hash**: `string`

All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
two hex digits per byte.

Different with `Address` type, hex-encoded bytes should not trim any zeros.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:774](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L774)

___

### type

• **type**: `string`

**`example`** write_resource

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:766](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L766)
