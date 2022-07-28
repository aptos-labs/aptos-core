---
id: "Types.UserTransactionRequest"
title: "Interface: UserTransactionRequest"
sidebar_label: "UserTransactionRequest"
custom_edit_url: null
---

[Types](../namespaces/Types.md).UserTransactionRequest

## Properties

### expiration\_timestamp\_secs

• **expiration\_timestamp\_secs**: `string`

Timestamp in seconds, e.g. transaction expiration timestamp.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:458](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L458)

___

### gas\_currency\_code

• `Optional` **gas\_currency\_code**: `string`

**`example`** XDX

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:452](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L452)

___

### gas\_unit\_price

• **gas\_unit\_price**: `string`

Unsigned int64 type value

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:449](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L449)

___

### max\_gas\_amount

• **max\_gas\_amount**: `string`

Unsigned int64 type value

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:446](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L446)

___

### payload

• **payload**: [`TransactionPayload`](../namespaces/Types.md#transactionpayload)

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:459](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L459)

___

### sender

• **sender**: `string`

Hex-encoded 16 bytes Aptos account address.

Prefixed with `0x` and leading zeros are trimmed.
See [doc](https://diem.github.io/move/address.html) for more details.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:440](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L440)

___

### sequence\_number

• **sequence\_number**: `string`

Unsigned int64 type value

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:443](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L443)
