import { deserializeValueBasedOnTypeTag, getPropertyValueRaw } from "./property_map_serde";
import {
  bcsSerializeBool,
  bcsSerializeStr,
  bcsSerializeU128,
  bcsSerializeU8,
  bcsSerializeUint64,
  bcsToBytes,
  Bytes,
} from "../bcs";
import { AccountAddress } from "../aptos_types";
import { HexString } from "../hex_string";
import { TypeTagParser } from "../transaction_builder";

test("test property_map_serializer", () => {
  function isSame(array1: Bytes, array2: Bytes): boolean {
    return (
      array1.length == array2.length &&
      array1.every((element, index) => {
        return element === array2[index];
      })
    );
  }
  let values = ["false", "10", "18446744073709551615", "340282366920938463463374607431768211455", "hello", "0x1"];
  let types = ["bool", "u8", "u64", "u128", "0x1::string::String", "address"];
  let newValues = getPropertyValueRaw(values, types);
  expect(isSame(newValues[0], bcsSerializeBool(false))).toBe(true);
  expect(isSame(newValues[1], bcsSerializeU8(10))).toBe(true);
  expect(isSame(newValues[2], bcsSerializeUint64(18446744073709551615n))).toBe(true);
  expect(isSame(newValues[3], bcsSerializeU128(340282366920938463463374607431768211455n))).toBe(true);
  expect(isSame(newValues[4], bcsSerializeStr(values[4]))).toBe(true);
  expect(isSame(newValues[5], bcsToBytes(AccountAddress.fromHex(new HexString("0x1"))))).toBe(true);
});

test("test propertymap deserializer", () => {
  function toHexString(data: Bytes): string {
    return new Buffer(data).toString("hex");
  }
  let values = [
    "false",
    "10",
    "18446744073709551615",
    "340282366920938463463374607431768211455",
    "hello",
    "0x0000000000000000000000000000000000000000000000000000000000000001",
  ];
  let types = ["bool", "u8", "u64", "u128", "0x1::string::String", "address"];
  let newValues = getPropertyValueRaw(values, types);
  for (let i = 0; i < values.length; i++) {
    expect(deserializeValueBasedOnTypeTag(new TypeTagParser(types[i]).parseTypeTag(), toHexString(newValues[i]))).toBe(
      values[i],
    );
  }
});
