/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

/**
 * This is a JSON representation of some data within an account resource. More specifically,
 * it is a map of strings to arbitrary JSON values / objects, where the keys are top level
 * fields within the given resource.
 *
 * To clarify, you might query for 0x1::account::Account and see the example data.
 *
 * Move `bool` type value is serialized into `boolean`.
 *
 * Move `u8` type value is serialized into `integer`.
 *
 * Move `u64` and `u128` type value is serialized into `string`.
 *
 * Move `address` type value (32 byte Aptos account address) is serialized into a HexEncodedBytes string.
 * For example:
 * - `0x1`
 * - `0x1668f6be25668c1a17cd8caf6b8d2f25`
 *
 * Move `vector` type value is serialized into `array`, except `vector<u8>` which is serialized into a
 * HexEncodedBytes string with `0x` prefix.
 * For example:
 * - `vector<u64>{255, 255}` => `["255", "255"]`
 * - `vector<u8>{255, 255}` => `0xffff`
 *
 * Move `struct` type value is serialized into `object` that looks like this (except some Move stdlib types, see the following section):
 * ```json
 * {
     * field1_name: field1_value,
     * field2_name: field2_value,
     * ......
     * }
     * ```
     *
     * For example:
     * `{ "created": "0xa550c18", "role_id": "0" }`
     *
     * **Special serialization for Move stdlib types**:
     * - [0x1::string::String](https://github.com/aptos-labs/aptos-core/blob/main/language/move-stdlib/docs/ascii.md)
     * is serialized into `string`. For example, struct value `0x1::string::String{bytes: b"Hello World!"}`
     * is serialized as `"Hello World!"` in JSON.
     *
     */
    export type MoveStructValue = {
    };

