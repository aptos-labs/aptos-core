---
id: "Types.DeleteModule"
title: "Interface: DeleteModule"
sidebar_label: "DeleteModule"
custom_edit_url: null
---

[Types](../namespaces/Types.md).DeleteModule

## Properties

### address

• **address**: `string`

Hex-encoded 16 bytes Aptos account address.

Prefixed with `0x` and leading zeros are trimmed.
See [doc](https://diem.github.io/move/address.html) for more details.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:662](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L662)

___

### module

• **module**: `string`

Move module id is a string representation of Move module.

Format: "{address}::{module name}"
`address` should be hex-encoded 16 bytes account address
that is prefixed with `0x` and leading zeros are trimmed.
Module name is case-sensitive.
See [doc](https://diem.github.io/move/modules-and-scripts.html#modules) for more details.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:673](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L673)

___

### state\_key\_hash

• **state\_key\_hash**: `string`

All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
two hex digits per byte.

Different with `Address` type, hex-encoded bytes should not trim any zeros.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:654](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L654)

___

### type

• **type**: `string`

**`example`** delete_module

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:646](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L646)
