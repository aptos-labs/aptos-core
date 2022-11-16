import { Bytes, Deserializer, Serializer } from "../bcs";
import { TypeTagParser } from "../transaction_builder";
import { serializeArg } from "../transaction_builder/builder_utils";
import { PropertyMap } from "../token_types";
import {
  TypeTag,
  TypeTagAddress,
  TypeTagBool,
  TypeTagStruct,
  TypeTagU128,
  TypeTagU64,
  TypeTagU8,
} from "../aptos_types";
import { HexString } from "../hex_string";

export function getPropertyValueRaw(values: Array<string>, types: Array<string>): Array<Bytes> {
  if (values.length !== types.length) {
    throw new Error("Length of property values and types not match");
  }

  const results = new Array<Bytes>();
  types.forEach((typ, index) => {
    try {
      const typeTag = new TypeTagParser(typ).parseTypeTag();
      const serializer = new Serializer();
      serializeArg(values[index], typeTag, serializer);
      results.push(serializer.getBytes());
    } catch (error) {
      // if not support type, just use the raw string bytes
      results.push(new TextEncoder().encode(values[index]));
    }
  });
  return results;
}

export function deserializePropertyMap(propertyMap: PropertyMap) {
  const entries = propertyMap.map.data;
  entries.forEach((prop) => {
    const val: string = prop.value.value;
    const typ: string = prop.value.type;
    const typeTag = new TypeTagParser(typ).parseTypeTag();
    const newValue = deserializeValueBasedOnTypeTag(typeTag, val);
    // eslint-disable-next-line no-param-reassign
    prop.value.value = newValue;
  });
}

export function deserializeValueBasedOnTypeTag(tag: TypeTag, val: string): string {
  const de = new Deserializer(new HexString(val).toUint8Array());
  let res: string = "";
  if (tag instanceof TypeTagU8) {
    res = de.deserializeU8().toString();
  } else if (tag instanceof TypeTagU64) {
    res = de.deserializeU64().toString();
  } else if (tag instanceof TypeTagU128) {
    res = de.deserializeU128().toString();
  } else if (tag instanceof TypeTagBool) {
    res = de.deserializeBool() ? "true" : "false";
  } else if (tag instanceof TypeTagAddress) {
    res = Buffer.from(de.deserializeFixedBytes(32)).toString("hex");
  } else if (tag instanceof TypeTagStruct && (tag as TypeTagStruct).isStringTypeTag()) {
    res = de.deserializeStr();
  } else {
    res = val;
  }
  return res;
}
