---
id: "Types.AccountResource"
title: "Interface: AccountResource"
sidebar_label: "AccountResource"
custom_edit_url: null
---

[Types](../namespaces/Types.md).AccountResource

Account resource is a Move struct value belongs to an account.

**`example`** {"type":"0x1::AptosAccount::Balance<0x1::XDX::XDX>","data":{"coin":{"value":"8000000000"}}}

## Properties

### data

• **data**: `object`

Account resource data is JSON representation of the Move struct `type`.

Move struct field name and value are serialized as object property name and value.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:161](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L161)

___

### type

• **type**: `string`

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

[ecosystem/typescript/sdk/src/api/data-contracts.ts:154](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L154)
