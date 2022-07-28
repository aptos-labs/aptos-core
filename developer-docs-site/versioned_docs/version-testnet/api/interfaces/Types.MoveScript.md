---
id: "Types.MoveScript"
title: "Interface: MoveScript"
sidebar_label: "MoveScript"
custom_edit_url: null
---

[Types](../namespaces/Types.md).MoveScript

## Properties

### abi

• `Optional` **abi**: [`MoveFunction`](Types.MoveFunction.md)

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:842](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L842)

___

### bytecode

• **bytecode**: `string`

All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
two hex digits per byte.

Different with `Address` type, hex-encoded bytes should not trim any zeros.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:841](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L841)
