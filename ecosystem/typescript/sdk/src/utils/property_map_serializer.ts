import assert from "assert";
import {
  AnyNumber,
  bcsSerializeBool,
  bcsSerializeStr,
  bcsSerializeU128,
  bcsSerializeU8,
  bcsSerializeUint64,
  Bytes,
} from "../bcs";
import { TypeTagParser } from "../transaction_builder";
import { TypeTagAddress, TypeTagBool, TypeTagStruct, TypeTagU128, TypeTagU64, TypeTagU8 } from "../aptos_types";

export function getPropertyValueRaw(values: Array<string>, types: Array<string>): Array<Bytes> {
  assert(values.length === types.length);
  const results = new Array<Bytes>();
  types.forEach((typ, index) => {
    const typeTag = new TypeTagParser(typ).parseTypeTag();
    if (typeTag instanceof TypeTagBool) {
      const res: boolean = JSON.parse(values[index]);
      results.push(bcsSerializeBool(res));
    } else if (typeTag instanceof TypeTagStruct && (typeTag as TypeTagStruct).isStringTypeTag()) {
      results.push(bcsSerializeStr(values[index]));
    } else if (typeTag instanceof TypeTagU8) {
      const res: number = JSON.parse(values[index]);
      results.push(bcsSerializeU8(res));
    } else if (typeTag instanceof TypeTagU64) {
      const res: AnyNumber = BigInt(values[index]);
      results.push(bcsSerializeUint64(res));
    } else if (typeTag instanceof TypeTagU128) {
      const res: AnyNumber = BigInt(values[index]);
      results.push(bcsSerializeU128(res));
    } else if (typeTag instanceof TypeTagAddress) {
      results.push(bcsSerializeStr(values[index]));
    } else {
      results.push(bcsSerializeStr(values[index]));
    }
  });
  return results;
}
