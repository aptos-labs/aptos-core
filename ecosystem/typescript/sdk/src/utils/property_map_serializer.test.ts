import { getPropertyValueRaw } from "./property_map_serializer";
import { bcsSerializeBool, bcsSerializeStr, bcsSerializeU128, bcsSerializeU8, bcsSerializeUint64, Bytes } from "../bcs";
import assert from "assert";

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
  assert(isSame(newValues[0], bcsSerializeBool(false)));
  assert(isSame(newValues[1], bcsSerializeU8(10)));
  assert(isSame(newValues[2], bcsSerializeUint64(18446744073709551615n)));
  assert(isSame(newValues[3], bcsSerializeU128(340282366920938463463374607431768211455n)));
  assert(isSame(newValues[4], bcsSerializeStr(values[4])));
  assert(isSame(newValues[5], bcsSerializeStr(values[5])));
});
