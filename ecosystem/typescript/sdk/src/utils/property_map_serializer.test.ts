import { getPropertyValueRaw } from "./property_map_serializer";
import { bcsSerializeBool, bcsSerializeStr, bcsSerializeU128, bcsSerializeU8, bcsSerializeUint64, Bytes } from "../bcs";
import assert from "assert";

test("test property_map_serializer", () => {
  var is_same: (array1: Bytes, array2: Bytes) => boolean = function (array1, array2): boolean {
    return (
      array1.length == array2.length &&
      array1.every(function (element, index) {
        return element === array2[index];
      })
    );
  };
  let values = ["false", "10", "1", "30", "hello", "0x1"];
  let types = ["bool", "u8", "u64", "u128", "0x1::string::String", "address"];
  let newValues = getPropertyValueRaw(values, types);
  assert(is_same(newValues[0], bcsSerializeBool(false)));
  assert(is_same(newValues[1], bcsSerializeU8(10)));
  assert(is_same(newValues[2], bcsSerializeUint64(1)));
  assert(is_same(newValues[3], bcsSerializeU128(30)));
  assert(is_same(newValues[4], bcsSerializeStr(values[4])));
  assert(is_same(newValues[5], bcsSerializeStr(values[5])));
});
