---
id: "Types.Account"
title: "Interface: Account"
sidebar_label: "Account"
custom_edit_url: null
---

[Types](../namespaces/Types.md).Account

Core account resource, used for identifying account and transaction execution.

**`example`** {"sequence_number":"1","authentication_key":"0x5307b5f4bc67829097a8ba9b43dba3b88261eeccd1f709d9bde240fc100fbb69"}

## Properties

### authentication\_key

• **authentication\_key**: `string`

All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
two hex digits per byte.

Different with `Address` type, hex-encoded bytes should not trim any zeros.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:131](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L131)

___

### sequence\_number

• **sequence\_number**: `string`

Unsigned int64 type value

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:123](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L123)
