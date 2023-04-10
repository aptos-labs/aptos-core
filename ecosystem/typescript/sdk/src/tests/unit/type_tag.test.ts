import { StructTag, TypeTagStruct } from "../../aptos_types/type_tag";

const expectedTypeTag = {
  string: "0x0000000000000000000000000000000000000000000000000000000000000001::aptos_coin::AptosCoin",
  address: "0x0000000000000000000000000000000000000000000000000000000000000001",
  module_name: "aptos_coin",
  name: "AptosCoin",
};

describe("StructTag", () => {
  test("make sure StructTag.fromString works with un-nested type tag", () => {
    const structTag = StructTag.fromString(expectedTypeTag.string);
    expect(structTag.address.toHexString()).toEqual(expectedTypeTag.address);
    expect(structTag.module_name.value).toEqual(expectedTypeTag.module_name);
    expect(structTag.name.value).toEqual(expectedTypeTag.name);
    expect(structTag.type_args.length).toEqual(0);
  });

  test("make sure StructTag.fromString works with nested type tag", () => {
    const structTag = StructTag.fromString(
      `${expectedTypeTag.string}<${expectedTypeTag.string}, ${expectedTypeTag.string}>`,
    );
    expect(structTag.address.toHexString()).toEqual(expectedTypeTag.address);
    expect(structTag.module_name.value).toEqual(expectedTypeTag.module_name);
    expect(structTag.name.value).toEqual(expectedTypeTag.name);
    expect(structTag.type_args.length).toEqual(2);

    // make sure the nested type tag is correct
    for (const typeArg of structTag.type_args) {
      const nestedTypeTag = typeArg as TypeTagStruct;
      expect(nestedTypeTag.value.address.toHexString()).toEqual(expectedTypeTag.address);
      expect(nestedTypeTag.value.module_name.value).toEqual(expectedTypeTag.module_name);
      expect(nestedTypeTag.value.name.value).toEqual(expectedTypeTag.name);
      expect(nestedTypeTag.value.type_args.length).toEqual(0);
    }
  });
});
