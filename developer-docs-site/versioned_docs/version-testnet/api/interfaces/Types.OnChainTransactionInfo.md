---
id: "Types.OnChainTransactionInfo"
title: "Interface: OnChainTransactionInfo"
sidebar_label: "OnChainTransactionInfo"
custom_edit_url: null
---

[Types](../namespaces/Types.md).OnChainTransactionInfo

## Properties

### accumulator\_root\_hash

• **accumulator\_root\_hash**: `string`

All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
two hex digits per byte.

Different with `Address` type, hex-encoded bytes should not trim any zeros.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:539](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L539)

___

### changes

• **changes**: [`WriteSetChange`](../namespaces/Types.md#writesetchange)[]

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:540](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L540)

___

### event\_root\_hash

• **event\_root\_hash**: `string`

All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
two hex digits per byte.

Different with `Address` type, hex-encoded bytes should not trim any zeros.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:515](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L515)

___

### gas\_used

• **gas\_used**: `string`

Unsigned int64 type value

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:518](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L518)

___

### hash

• **hash**: `string`

All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
two hex digits per byte.

Different with `Address` type, hex-encoded bytes should not trim any zeros.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:499](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L499)

___

### state\_root\_hash

• **state\_root\_hash**: `string`

All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
two hex digits per byte.

Different with `Address` type, hex-encoded bytes should not trim any zeros.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:507](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L507)

___

### success

• **success**: `boolean`

Transaction execution result (success: true, failure: false).
See `vm_status` for human readable error message from Aptos VM.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:525](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L525)

___

### version

• **version**: `string`

Unsigned int64 type value

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:491](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L491)

___

### vm\_status

• **vm\_status**: `string`

Human readable transaction execution result message from Aptos VM.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:531](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L531)
