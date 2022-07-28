---
id: "Types.MoveModuleABI"
title: "Interface: MoveModuleABI"
sidebar_label: "MoveModuleABI"
custom_edit_url: null
---

[Types](../namespaces/Types.md).MoveModuleABI

Move Module ABI is JSON representation of Move module binary interface.

## Properties

### address

• **address**: `string`

Hex-encoded 16 bytes Aptos account address.

Prefixed with `0x` and leading zeros are trimmed.
See [doc](https://diem.github.io/move/address.html) for more details.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:311](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L311)

___

### exposed\_functions

• **exposed\_functions**: [`MoveFunction`](Types.MoveFunction.md)[]

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:316](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L316)

___

### friends

• **friends**: `string`[]

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:315](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L315)

___

### name

• **name**: `string`

**`example`** Aptos

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:314](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L314)

___

### structs

• **structs**: [`MoveStruct`](Types.MoveStruct.md)[]

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:317](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L317)
