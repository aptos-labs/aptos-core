---
id: "Types.DeleteResource"
title: "Interface: DeleteResource"
sidebar_label: "DeleteResource"
custom_edit_url: null
---

[Types](../namespaces/Types.md).DeleteResource

Delete account resource change.

## Properties

### address

• **address**: `string`

Hex-encoded 16 bytes Aptos account address.

Prefixed with `0x` and leading zeros are trimmed.
See [doc](https://diem.github.io/move/address.html) for more details.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:697](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L697)

___

### resource

• **resource**: `string`

String representation of an on-chain Move struct type.

It is a combination of:
  1. `Move module address`, `module name` and `struct name` joined by `::`.
  2. `struct generic type parameters` joined by `, `.
Examples:
  * `0x1::Aptos::Aptos<0x1::XDX::XDX>`
  * `0x1::Abc::Abc<vector<u8>, vector<u64>>`
  * `0x1::AptosAccount::AccountOperationsCapability`
Note:
  1. Empty chars should be ignored when comparing 2 struct tag ids.
  2. When used in an URL path, should be encoded by url-encoding (AKA percent-encoding).
See [doc](https://diem.github.io/move/structs-and-resources.html) for more details.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:714](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L714)

___

### state\_key\_hash

• **state\_key\_hash**: `string`

All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
two hex digits per byte.

Different with `Address` type, hex-encoded bytes should not trim any zeros.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:689](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L689)

___

### type

• **type**: `string`

**`example`** delete_resource

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:681](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L681)
