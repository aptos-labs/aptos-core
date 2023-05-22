import { Bytes, Deserializer, Serializer } from "../bcs";
import { serializeArg } from "../transaction_builder/builder_utils";
import {
  stringStructTag,
  TypeTag,
  TypeTagAddress,
  TypeTagBool,
  TypeTagParser,
  TypeTagStruct,
  TypeTagU128,
  TypeTagU64,
  TypeTagU8,
} from "../aptos_types";
import { HexString } from "./hex_string";

export class PropertyValue {
  type: string;

  value: any;

  constructor(type: string, value: string) {
    this.type = type;
    this.value = value;
  }
}

export class PropertyMap {
  data: { [key: string]: PropertyValue };

  constructor() {
    this.data = {};
  }

  setProperty(key: string, value: PropertyValue) {
    this.data[key] = value;
  }
}

export function getPropertyType(typ: string): TypeTag {
  let typeTag: TypeTag;
  if (typ === "string" || typ === "String") {
    typeTag = new TypeTagStruct(stringStructTag);
  } else {
    typeTag = new TypeTagParser(typ).parseTypeTag();
  }
  return typeTag;
}

export function getPropertyValueRaw(values: Array<string>, types: Array<string>): Array<Bytes> {
  if (values.length !== types.length) {
    throw new Error("Length of property values and types not match");
  }

  const results = new Array<Bytes>();
  types.forEach((typ, index) => {
    try {
      const typeTag = getPropertyType(typ);
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

export function getSinglePropertyValueRaw(value: string, type: string): Uint8Array {
  if (!value || !type) {
    throw new Error("value or type can not be empty");
  }

  try {
    const typeTag = getPropertyType(type);
    const serializer = new Serializer();
    serializeArg(value, typeTag, serializer);
    return serializer.getBytes();
  } catch (error) {
    // if not support type, just use the raw string bytes
    return new TextEncoder().encode(value);
  }
}

export function deserializePropertyMap(rawPropertyMap: any): PropertyMap {
  const entries = rawPropertyMap.map.data;
  const pm = new PropertyMap();
  entries.forEach((prop: any) => {
    const { key } = prop;
    const val: string = prop.value.value;
    const typ: string = prop.value.type;
    const typeTag = getPropertyType(typ);
    const newValue = deserializeValueBasedOnTypeTag(typeTag, val);
    const pv = new PropertyValue(typ, newValue);
    pm.setProperty(key, pv);
  });
  return pm;
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
    res = HexString.fromUint8Array(de.deserializeFixedBytes(32)).hex();
  } else if (tag instanceof TypeTagStruct && (tag as TypeTagStruct).isStringTypeTag()) {
    res = de.deserializeStr();
  } else {
    res = val;
  }
  return res;
}
