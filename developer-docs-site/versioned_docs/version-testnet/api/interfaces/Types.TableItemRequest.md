---
id: "Types.TableItemRequest"
title: "Interface: TableItemRequest"
sidebar_label: "TableItemRequest"
custom_edit_url: null
---

[Types](../namespaces/Types.md).TableItemRequest

## Properties

### key

• **key**: `any`

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

[ecosystem/typescript/sdk/src/api/data-contracts.ts:1171](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L1171)

___

### key\_type

• **key\_type**: `string`

String representation of an on-chain Move type identifier defined by the Move language.

Values:
  - bool
  - u8
  - u64
  - u128
  - address
  - signer
  - vector: `vector<{non-reference MoveTypeId}>`
  - struct: `{address}::{module_name}::{struct_name}::<{generic types}>`
  - reference: immutable `&` and mutable `&mut` references.
  - generic_type_parameter: it is always start with `T` and following an index number,
    which is the position of the generic type parameter in the `struct` or
    `function` generic type parameters definition.
Vector type value examples:
  * `vector<u8>`
  * `vector<vector<u64>>`
  * `vector<0x1::AptosAccount::Balance<0x1::XDX::XDX>>`
Struct type value examples:
  * `0x1::Aptos::Aptos<0x1::XDX::XDX>`
  * `0x1::Abc::Abc<vector<u8>, vector<u64>>`
  * `0x1::AptosAccount::AccountOperationsCapability`
Reference type value examples:
  * `&signer`
  * `&mut address`
  * `&mut vector<u8>`
Generic type parameter value example, the following is `0x1::TransactionFee::TransactionFee` JSON representation:
    {
        "name": "TransactionFee",
        "is_native": false,
        "abilities": ["key"],
        "generic_type_params": [
            {"constraints": [], "is_phantom": true}
        ],
        "fields": [
            { "name": "balance", "type": "0x1::Aptos::Aptos<T0 />" },
            { "name": "preburn", "type": "0x1::Aptos::Preburn<T0 />" }
        ]
    }
It's Move source code:
    module AptosFramework::TransactionFee {
        struct TransactionFee<phantom CoinType /> has key {
            balance: Aptos<CoinType />,
            preburn: Preburn<CoinType />,
        }
The `T0` in the above JSON representation is the generic type place holder for
the `CoinType` in the Move source code.
Note:
  1. Empty chars should be ignored when comparing 2 struct tag ids.
  2. When used in an URL path, should be encoded by url-encoding (AKA percent-encoding).

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:1088](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L1088)

___

### value\_type

• **value\_type**: `string`

String representation of an on-chain Move type identifier defined by the Move language.

Values:
  - bool
  - u8
  - u64
  - u128
  - address
  - signer
  - vector: `vector<{non-reference MoveTypeId}>`
  - struct: `{address}::{module_name}::{struct_name}::<{generic types}>`
  - reference: immutable `&` and mutable `&mut` references.
  - generic_type_parameter: it is always start with `T` and following an index number,
    which is the position of the generic type parameter in the `struct` or
    `function` generic type parameters definition.
Vector type value examples:
  * `vector<u8>`
  * `vector<vector<u64>>`
  * `vector<0x1::AptosAccount::Balance<0x1::XDX::XDX>>`
Struct type value examples:
  * `0x1::Aptos::Aptos<0x1::XDX::XDX>`
  * `0x1::Abc::Abc<vector<u8>, vector<u64>>`
  * `0x1::AptosAccount::AccountOperationsCapability`
Reference type value examples:
  * `&signer`
  * `&mut address`
  * `&mut vector<u8>`
Generic type parameter value example, the following is `0x1::TransactionFee::TransactionFee` JSON representation:
    {
        "name": "TransactionFee",
        "is_native": false,
        "abilities": ["key"],
        "generic_type_params": [
            {"constraints": [], "is_phantom": true}
        ],
        "fields": [
            { "name": "balance", "type": "0x1::Aptos::Aptos<T0 />" },
            { "name": "preburn", "type": "0x1::Aptos::Preburn<T0 />" }
        ]
    }
It's Move source code:
    module AptosFramework::TransactionFee {
        struct TransactionFee<phantom CoinType /> has key {
            balance: Aptos<CoinType />,
            preburn: Preburn<CoinType />,
        }
The `T0` in the above JSON representation is the generic type place holder for
the `CoinType` in the Move source code.
Note:
  1. Empty chars should be ignored when comparing 2 struct tag ids.
  2. When used in an URL path, should be encoded by url-encoding (AKA percent-encoding).

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:1143](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L1143)
