import assert from "assert";
import { bcsSerializeBool, bcsSerializeStr, bcsSerializeU128, bcsSerializeU8, bcsSerializeUint64, Bytes } from "../bcs";

export function getPropertyValueRaw(values: Array<string>, types: Array<string>): Array<Bytes> {
  assert(values.length === types.length);
  const results = new Array<Bytes>();
  types.forEach((typ, index) => {
    if (typ === "bool") {
      const res: boolean = JSON.parse(values[index]);
      results.push(bcsSerializeBool(res));
    } else if (typ === "0x1::string::String") {
      results.push(bcsSerializeStr(values[index]));
    } else if (typ === "u8") {
      const res: number = JSON.parse(values[index]);
      results.push(bcsSerializeU8(res));
    } else if (typ === "u64") {
      const res: number = JSON.parse(values[index]);
      results.push(bcsSerializeUint64(res));
    } else if (typ === "u128") {
      const res: number = JSON.parse(values[index]);
      results.push(bcsSerializeU128(res));
    } else if (typ === "address") {
      results.push(bcsSerializeStr(values[index]));
    } else {
      results.push(bcsSerializeStr(values[index]));
    }
  });
  return results;
}
