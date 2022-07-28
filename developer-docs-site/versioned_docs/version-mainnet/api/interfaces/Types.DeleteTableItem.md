---
id: "Types.DeleteTableItem"
title: "Interface: DeleteTableItem"
sidebar_label: "DeleteTableItem"
custom_edit_url: null
---

[Types](../namespaces/Types.md).DeleteTableItem

Delete table item change.

## Properties

### data

• **data**: `Object`

Table item deletion

#### Type declaration

| Name | Type |
| :------ | :------ |
| `handle` | `string` |
| `key` | `string` |

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:733](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L733)

___

### state\_key\_hash

• **state\_key\_hash**: `string`

All bytes data are represented as hex-encoded string prefixed with `0x` and fulfilled with
two hex digits per byte.

Different with `Address` type, hex-encoded bytes should not trim any zeros.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:730](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L730)

___

### type

• **type**: `string`

**`example`** delete_table_item

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:722](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L722)
