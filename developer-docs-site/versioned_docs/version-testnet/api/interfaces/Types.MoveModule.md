---
id: "Types.MoveModule"
title: "Interface: MoveModule"
sidebar_label: "MoveModule"
custom_edit_url: null
---

[Types](../namespaces/Types.md).MoveModule

## Properties

### abi

• `Optional` **abi**: [`MoveModuleABI`](Types.MoveModuleABI.md)

Move Module ABI is JSON representation of Move module binary interface.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:298](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L298)

___

### bytecode

• **bytecode**: `string`

All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
two hex digits per byte.

Different with `Address` type, hex-encoded bytes should not trim any zeros.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:292](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L292)
