---
id: "Types.Event"
title: "Interface: Event"
sidebar_label: "Event"
custom_edit_url: null
---

[Types](../namespaces/Types.md).Event

Event `key` and `sequence_number` are global identifier of the event.

Event `sequence_number` starts from 0 for each event key.

Event `type` is the type information of the event `data`, you can use the `type`
to decode the `data` JSON.

**`example`** {"key":"0x00000000000000000000000000000000000000000a550c18","sequence_number":"23","type":"0x1::AptosAccount::CreateAccountEvent","data":{"created":"0xa550c18","role_id":"0"}}

## Properties

### data

• **data**: `any`

Move `bool` type value is serialized into `boolean`.

Move `u8` type value is serialized into `integer`.
Move `u64` and `u128` type value is serialized into `string`.
Move `address` type value(16 bytes Aptos account address) is serialized into
hex-encoded string, which is prefixed with `0x` and leading zeros are trimmed.
For example:
  * `0x1`
  * `0x1668f6be25668c1a17cd8caf6b8d2f25`
Move `vector` type value is serialized into `array`, except `vector<u8>` which is
serialized into hex-encoded string with `0x` prefix.
  * `vector<u64>{255, 255}` => `["255", "255"]`
  * `vector<u8>{255, 255}` => `0xffff`
Move `struct` type value is serialized into `object` that looks like this (except some Move stdlib types, see the following section):
  ```json
  {
    field1_name: field1_value,
    field2_name: field2_value,
    ......
  }
  ```
  `{ "created": "0xa550c18", "role_id": "0" }`
**Special serialization for Move stdlib types:**
* [0x1::ASCII::String](https://github.com/aptos-labs/aptos-core/blob/main/language/move-stdlib/docs/ASCII.md) is serialized into `string`. For example, struct value `0x1::ASCII::String{bytes: b"hello world"}` is serialized as `"hello world"` in JSON.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:968](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L968)

___

### key

• **key**: `string`

Event key is a global index for an event stream.

It is hex-encoded BCS bytes of `EventHandle` `guid` field value, which is
a combination of a `uint64` creation number and account address
(without trimming leading zeros).
For example, event key `0x00000000000000000000000000000000000000000a550c18`
is combined by the following 2 parts:
  1. `0000000000000000`: `uint64` representation of `0`.
  2. `0000000000000000000000000a550c18`: 16 bytes of account address.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:907](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L907)

___

### sequence\_number

• **sequence\_number**: `string`

Event `sequence_number` is unique id of an event in an event stream.
Event `sequence_number` starts from 0 for each event key.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:914](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L914)

___

### type

• **type**: `string`

String representation of an on-chain Move type tag that is exposed in transaction payload.

Values:
  - bool
  - u8
  - u64
  - u128
  - address
  - signer
  - vector: `vector<{non-reference MoveTypeId}>`
  - struct: `{address}::{module_name}::{struct_name}::<{generic types}>`
Vector type value examples:
  * `vector<u8>`
  * `vector<vector<u64>>`
  * `vector<0x1::AptosAccount::Balance<0x1::XDX::XDX>>`
Struct type value examples:
  * `0x1::Aptos::Aptos<0x1::XDX::XDX>`
  * `0x1::Abc::Abc<vector<u8>, vector<u64>>`
  * `0x1::AptosAccount::AccountOperationsCapability`
Note:
  1. Empty chars should be ignored when comparing 2 struct tag ids.
  2. When used in an URL path, should be encoded by url-encoding (AKA percent-encoding).

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:940](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L940)
