// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file is where we apply a number of traits that allow us to use these
//! traits with Poem. For more information on how these macros work, see the
//! documentation within `crates/aptos-openapi`.
//!
//! In some cases we use these derives because the underlying types are not
//! expressible via OpenAPI, e.g. Address. In other cases, we use them because
//! we do not want to use the default serialization of the types, but instead
//! serialize them as strings, e.g. EntryFunctionId.
//!
//! For potential future improvements here, see:
//! <https://github.com/aptos-labs/aptos-core/issues/2319>

// READ ME: You'll see that some of the examples (specifically those for hex
// strings) have a space at the end. This is necessary to make sure the UI
// displays the example value correctly. See more here:
// https://github.com/aptos-labs/aptos-core/pull/2703

use crate::{
    move_types::{MoveAbility, MoveStructValue},
    Address, AssetType, EntryFunctionId, HashValue, HexEncodedBytes, IdentifierWrapper,
    MoveModuleId, MoveStructTag, MoveType, StateKeyWrapper, U128, U256, U64,
};
use aptos_openapi::{impl_poem_parameter, impl_poem_type};
use indoc::indoc;
use serde_json::json;

impl_poem_type!(
    Address,
    "string",
    (
        example = Some(serde_json::Value::String(
            "0x88fbd33f54e1126269769780feb24480428179f552e2313fbe571b72e62a1ca1".to_string()
        )),
        format = Some("hex"),
        description = Some(indoc! {"
            A hex encoded 32 byte Aptos account address.

            This is represented in a string as a 64 character hex string, sometimes
            shortened by stripping leading 0s, and adding a 0x.

            For example, address 0x0000000000000000000000000000000000000000000000000000000000000001 is represented as 0x1.
        "})
    )
);

impl_poem_type!(
    AssetType,
    "string",
    (
        example = Some(serde_json::Value::String(
            "0x1::aptos_coin::AptosCoin".to_string()
        )),
        format = Some("hex"),
        description = Some(indoc! {"
            A hex encoded 32 byte Aptos account address or a struct tag.

            This is represented in a string as a 64 character hex string, sometimes
            shortened by stripping leading 0s, and adding a 0x or
            Format: `{address}::{module name}::{struct name}`
        "})
    )
);

impl_poem_type!(
    EntryFunctionId,
    "string",
    (
        example = Some(serde_json::Value::String(
            "0x1::aptos_coin::transfer".to_string()
        )),
        description = Some(indoc! {"
          Entry function id is string representation of a entry function defined on-chain.

          Format: `{address}::{module name}::{function name}`

          Both `module name` and `function name` are case-sensitive.
  "})
    )
);

impl_poem_type!(HashValue, "string", ());

impl_poem_type!(
    HexEncodedBytes,
    "string",
    (
        example = Some(serde_json::Value::String(
            "0x88fbd33f54e1126269769780feb24480428179f552e2313fbe571b72e62a1ca1".to_string()
        )),
        format = Some("hex"),
        description = Some(indoc! {"
            All bytes (Vec<u8>) data is represented as hex-encoded string prefixed with `0x` and fulfilled with
            two hex digits per byte.

            Unlike the `Address` type, HexEncodedBytes will not trim any zeros.
        "})
    )
);

impl_poem_type!(IdentifierWrapper, "string", ());

impl_poem_type!(MoveAbility, "string", ());

impl_poem_type!(
    MoveModuleId,
    "string",
    (
        example = Some(serde_json::Value::String("0x1::aptos_coin".to_string())),
        description = Some(indoc! {"
          Move module id is a string representation of Move module.

          Format: `{address}::{module name}`

          `address` should be hex-encoded 32 byte account address that is prefixed with `0x`.

          Module name is case-sensitive.
    "})
    )
);

impl_poem_type!(
    MoveStructTag,
    "string",
    (
        example = Some(serde_json::Value::String(
            "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>".to_string()
        )),
        pattern = Some("^0x[0-9a-zA-Z:_<>]+$".to_string()),
        description = Some(indoc! {"
        String representation of a MoveStructTag (on-chain Move struct type). This exists so you
        can specify MoveStructTags as path / query parameters, e.g. for get_events_by_event_handle.

        It is a combination of:
          1. `move_module_address`, `module_name` and `struct_name`, all joined by `::`
          2. `struct generic type parameters` joined by `, `

        Examples:
          * `0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>`
          * `0x1::account::Account`

        Note:
          1. Empty chars should be ignored when comparing 2 struct tag ids.
          2. When used in an URL path, should be encoded by url-encoding (AKA percent-encoding).

        See [doc](https://aptos.dev/concepts/accounts) for more details.
      "})
    )
);

impl_poem_type!(
    MoveStructValue,
    "object",
    (
        example = Some(json!({
            "authentication_key": "0x0000000000000000000000000000000000000000000000000000000000000001",
            "coin_register_events": {
              "counter": "0",
              "guid": {
                "id": {
                  "addr": "0x1",
                  "creation_num": "0"
                }
              }
            },
            "self_address": "0x1",
            "sequence_number": "0"
        })),
        description = Some(indoc! {"
            This is a JSON representation of some data within an account resource. More specifically,
            it is a map of strings to arbitrary JSON values / objects, where the keys are top level
            fields within the given resource.

            To clarify, you might query for 0x1::account::Account and see the example data.

            Move `bool` type value is serialized into `boolean`.

            Move `u8`, `u16` and `u32` type value is serialized into `integer`.

            Move `u64`, `u128` and `u256` type value is serialized into `string`.

            Move `address` type value (32 byte Aptos account address) is serialized into a HexEncodedBytes string.
            For example:
              - `0x1`
              - `0x1668f6be25668c1a17cd8caf6b8d2f25`

            Move `vector` type value is serialized into `array`, except `vector<u8>` which is serialized into a
            HexEncodedBytes string with `0x` prefix.
            For example:
              - `vector<u64>{255, 255}` => `[\"255\", \"255\"]`
              - `vector<u8>{255, 255}` => `0xffff`

            Move `struct` type value is serialized into `object` that looks like this (except some Move stdlib types, see the following section):
              ```json
              {
                field1_name: field1_value,
                field2_name: field2_value,
                ......
              }
              ```

            For example:
              `{ \"created\": \"0xa550c18\", \"role_id\": \"0\" }`

            **Special serialization for Move stdlib types**:
              - [0x1::string::String](https://github.com/aptos-labs/aptos-core/blob/main/third_party/move/move-stdlib/docs/ascii.md)
                is serialized into `string`. For example, struct value `0x1::string::String{bytes: b\"Hello World!\"}`
                is serialized as `\"Hello World!\"` in JSON.
        "})
    )
);

impl_poem_type!(
    MoveType,
    "string",
    (
        pattern =
            Some("^(bool|u8|u64|u128|address|signer|vector<.+>|0x[0-9a-zA-Z:_<, >]+)$".to_string()),
        description = Some(indoc! {"
            String representation of an on-chain Move type tag that is exposed in transaction payload.
                Values:
                  - bool
                  - u8
                  - u16
                  - u32
                  - u64
                  - u128
                  - u256
                  - address
                  - signer
                  - vector: `vector<{non-reference MoveTypeId}>`
                  - struct: `{address}::{module_name}::{struct_name}::<{generic types}>`

                Vector type value examples:
                  - `vector<u8>`
                  - `vector<vector<u64>>`
                  - `vector<0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>>`

                Struct type value examples:
                  - `0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>
                  - `0x1::account::Account`

                Note:
                  1. Empty chars should be ignored when comparing 2 struct tag ids.
                  2. When used in an URL path, should be encoded by url-encoding (AKA percent-encoding).
    "})
    )
);

impl_poem_type!(
    StateKeyWrapper,
    "string",
    (
        example = Some(serde_json::Value::String("0000000000000000000000000000000000000000000000000000000000000000012f0000000000000000000000000000000000000000000000000000000000000000010d7374616b696e675f70726f7879".to_string())),
        description = Some(indoc! {"
          Representation of a StateKey as a hex string. This is used for cursor based pagination.
        "})
    )
);

impl_poem_type!(
    U64,
    "string",
    (
        example = Some(serde_json::Value::String("32425224034".to_string())),
        format = Some("uint64"),
        description = Some(indoc! {"
        A string containing a 64-bit unsigned integer.

        We represent u64 values as a string to ensure compatibility with languages such
        as JavaScript that do not parse u64s in JSON natively.
    "})
    )
);

impl_poem_type!(
    U128,
    "string",
    (
        example = Some(serde_json::Value::String(
            "340282366920938463463374607431768211454".to_string()
        )),
        format = Some("uint128"),
        description = Some(indoc! {"
        A string containing a 128-bit unsigned integer.

        We represent u128 values as a string to ensure compatibility with languages such
        as JavaScript that do not parse u128s in JSON natively.
    "})
    )
);

impl_poem_type!(
    U256,
    "string",
    (
        example = Some(serde_json::Value::String(
            "340282366920938463463374607431768211454".to_string()
        )),
        format = Some("uint256"),
        description = Some(indoc! {"
      A string containing a 256-bit unsigned integer.

      We represent u256 values as a string to ensure compatibility with languages such
      as JavaScript that do not parse u256s in JSON natively.
  "})
    )
);

impl_poem_parameter!(
    Address,
    AssetType,
    HashValue,
    IdentifierWrapper,
    HexEncodedBytes,
    MoveStructTag,
    StateKeyWrapper,
    U64,
    U128
);
