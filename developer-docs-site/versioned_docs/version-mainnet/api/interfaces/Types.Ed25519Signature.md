---
id: "Types.Ed25519Signature"
title: "Interface: Ed25519Signature"
sidebar_label: "Ed25519Signature"
custom_edit_url: null
---

[Types](../namespaces/Types.md).Ed25519Signature

Please refer to https://github.com/aptos-labs/aptos-core/tree/main/specifications/crypto#signature-and-verification for
more details.

## Properties

### public\_key

• **public\_key**: `string`

All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
two hex digits per byte.

Different with `Address` type, hex-encoded bytes should not trim any zeros.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:987](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L987)

___

### signature

• **signature**: `string`

All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
two hex digits per byte.

Different with `Address` type, hex-encoded bytes should not trim any zeros.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:995](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L995)

___

### type

• **type**: `string`

**`example`** ed25519_signature

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:979](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L979)
