---
id: "Types.MultiEd25519Signature"
title: "Interface: MultiEd25519Signature"
sidebar_label: "MultiEd25519Signature"
custom_edit_url: null
---

[Types](../namespaces/Types.md).MultiEd25519Signature

Multi ed25519 signature, please refer to https://github.com/aptos-labs/aptos-core/tree/main/specifications/crypto#multi-signatures for more details.

## Properties

### bitmap

• **bitmap**: `string`

All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
two hex digits per byte.

Different with `Address` type, hex-encoded bytes should not trim any zeros.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:1018](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L1018)

___

### public\_keys

• **public\_keys**: `string`[]

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:1004](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L1004)

___

### signatures

• **signatures**: `string`[]

signatures created based on the `threshold`

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:1007](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L1007)

___

### threshold

• **threshold**: `number`

The threshold of the multi ed25519 account key.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:1010](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L1010)

___

### type

• **type**: `string`

**`example`** multi_ed25519_signature

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:1003](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L1003)
