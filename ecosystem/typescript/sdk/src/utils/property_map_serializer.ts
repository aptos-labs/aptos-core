import { Bytes, Serializer } from "../bcs";
import { TypeTagParser } from "../transaction_builder";
import { serializeArg } from "../transaction_builder/builder_utils";

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
