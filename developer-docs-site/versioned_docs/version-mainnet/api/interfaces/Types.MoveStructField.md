---
id: "Types.MoveStructField"
title: "Interface: MoveStructField"
sidebar_label: "MoveStructField"
custom_edit_url: null
---

[Types](../namespaces/Types.md).MoveStructField

**`example`** {"name":"value","type":"u64"}

## Properties

### name

• **name**: `string`

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:335](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L335)

___

### type

• **type**: `string`

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

[ecosystem/typescript/sdk/src/api/data-contracts.ts:390](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L390)
