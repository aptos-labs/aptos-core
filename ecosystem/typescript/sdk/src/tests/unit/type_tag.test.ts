import {
  objectStructTag,
  StructTag,
  TypeTag,
  TypeTagAddress,
  TypeTagBool,
  TypeTagParser,
  TypeTagParserError,
  TypeTagSigner,
  TypeTagStruct,
  TypeTagU128,
  TypeTagU16,
  TypeTagU256,
  TypeTagU32,
  TypeTagU64,
  TypeTagU8,
  TypeTagVector,
} from "../../aptos_types/type_tag";
import { Deserializer, Serializer } from "../../bcs";

const expectedTypeTag = {
  string: "0x0000000000000000000000000000000000000000000000000000000000000001::some_module::SomeResource",
  address: "0x0000000000000000000000000000000000000000000000000000000000000001",
  module_name: "some_module",
  name: "SomeResource",
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

describe("TypeTagParser", () => {
  test("make sure parseTypeTag throws TypeTagParserError 'Invalid type tag' if invalid format", () => {
    let typeTag = "0x000";
    let parser = new TypeTagParser(typeTag);

    try {
      parser.parseTypeTag();
    } catch (error) {
      expect(error).toBeInstanceOf(TypeTagParserError);
      const typeTagError = error as TypeTagParserError;
      expect(typeTagError.message).toEqual("Invalid type tag.");
    }

    typeTag = "0x1::some_module::SomeResource<0x1>";
    parser = new TypeTagParser(typeTag);
    expect(() => parser.parseTypeTag()).toThrowError("Invalid type tag.");
  });

  test("make sure parseTypeTag works with un-nested type tag", () => {
    const parser = new TypeTagParser(expectedTypeTag.string);
    const result = parser.parseTypeTag() as TypeTagStruct;
    expect(result.value.address.toHexString()).toEqual(expectedTypeTag.address);
    expect(result.value.module_name.value).toEqual(expectedTypeTag.module_name);
    expect(result.value.name.value).toEqual(expectedTypeTag.name);
    expect(result.value.type_args.length).toEqual(0);
  });

  test("make sure parseTypeTag works with nested type tag", () => {
    const typeTag = "0x1::some_module::SomeResource<0x1::some_module::SomeResource, 0x1::some_module::SomeResource>";
    const parser = new TypeTagParser(typeTag);
    const result = parser.parseTypeTag() as TypeTagStruct;
    expect(result.value.address.toHexString()).toEqual(expectedTypeTag.address);
    expect(result.value.module_name.value).toEqual(expectedTypeTag.module_name);
    expect(result.value.name.value).toEqual(expectedTypeTag.name);
    expect(result.value.type_args.length).toEqual(2);

    // make sure the nested type tag is correct
    for (const typeArg of result.value.type_args) {
      const nestedTypeTag = typeArg as TypeTagStruct;
      expect(nestedTypeTag.value.address.toHexString()).toEqual(expectedTypeTag.address);
      expect(nestedTypeTag.value.module_name.value).toEqual(expectedTypeTag.module_name);
      expect(nestedTypeTag.value.name.value).toEqual(expectedTypeTag.name);
      expect(nestedTypeTag.value.type_args.length).toEqual(0);
    }
  });

  describe("parse Object type", () => {
    test("TypeTagParser successfully parses an Object type", () => {
      const typeTag = "0x1::object::Object<T>";
      const parser = new TypeTagParser(typeTag);
      const result = parser.parseTypeTag();
      expect(result instanceof TypeTagAddress).toBeTruthy();
    });

    test("TypeTagParser successfully parses complex Object types", () => {
      const typeTag = "0x1::object::Object<T>";
      const parser = new TypeTagParser(typeTag);
      const result = parser.parseTypeTag();
      expect(result instanceof TypeTagAddress).toBeTruthy();

      const typeTag2 = "0x1::object::Object<0x1::coin::Fun<A, B<C>>>";
      const parser2 = new TypeTagParser(typeTag2);
      const result2 = parser2.parseTypeTag();
      expect(result2 instanceof TypeTagAddress).toBeTruthy();
    });

    test("TypeTagParser does not parse unofficial objects", () => {
      const typeTag = "0x12345::object::Object<T>";
      const parser = new TypeTagParser(typeTag);
      expect(() => parser.parseTypeTag()).toThrowError("Invalid type tag.");
    });

    test("TypeTagParser successfully parses an Option type", () => {
      const typeTag = "0x1::option::Option<u8>";
      const parser = new TypeTagParser(typeTag);
      const result = parser.parseTypeTag();

      if (result instanceof TypeTagStruct) {
        expect(result.value === objectStructTag(new TypeTagU8()));
      } else {
        fail(`Not an option ${result}`);
      }
    });

    test("TypeTagParser successfully parses a strcut with a nested Object type", () => {
      const typeTag = "0x1::some_module::SomeResource<0x1::object::Object<T>>";
      const parser = new TypeTagParser(typeTag);
      const result = parser.parseTypeTag() as TypeTagStruct;
      expect(result.value.address.toHexString()).toEqual(expectedTypeTag.address);
      expect(result.value.module_name.value).toEqual("some_module");
      expect(result.value.name.value).toEqual("SomeResource");
      expect(result.value.type_args[0] instanceof TypeTagAddress).toBeTruthy();
    });

    test("TypeTagParser successfully parses a struct with a nested Object and Struct types", () => {
      const typeTag = "0x1::some_module::SomeResource<0x1::object::Object<T>, 0x1::some_module::SomeResource>";
      const parser = new TypeTagParser(typeTag);
      const result = parser.parseTypeTag() as TypeTagStruct;
      expect(result.value.address.toHexString()).toEqual(expectedTypeTag.address);
      expect(result.value.module_name.value).toEqual("some_module");
      expect(result.value.name.value).toEqual("SomeResource");
      expect(result.value.type_args.length).toEqual(2);
      expect(result.value.type_args[0] instanceof TypeTagAddress).toBeTruthy();
      expect(result.value.type_args[1] instanceof TypeTagStruct).toBeTruthy();
    });
  });

  describe("supports generic types", () => {
    test("throws an error when the type to use is not provided", () => {
      const typeTag = "T0";
      const parser = new TypeTagParser(typeTag);
      expect(() => {
        parser.parseTypeTag();
      }).toThrow("Can't convert generic type since no typeTags were specified.");
    });

    test("successfully parses a generic type tag to the provided type", () => {
      const typeTag = "T0";
      const parser = new TypeTagParser(typeTag, ["bool"]);
      const result = parser.parseTypeTag();
      expect(result instanceof TypeTagBool).toBeTruthy();
    });
  });
});

describe("Deserialize TypeTags", () => {
  test("deserializes a TypeTagBool correctly", () => {
    const serializer = new Serializer();
    const tag = new TypeTagBool();

    tag.serialize(serializer);

    expect(TypeTag.deserialize(new Deserializer(serializer.getBytes()))).toBeInstanceOf(TypeTagBool);
  });

  test("deserializes a TypeTagU8 correctly", () => {
    const serializer = new Serializer();
    const tag = new TypeTagU8();

    tag.serialize(serializer);

    expect(TypeTag.deserialize(new Deserializer(serializer.getBytes()))).toBeInstanceOf(TypeTagU8);
  });

  test("deserializes a TypeTagU16 correctly", () => {
    const serializer = new Serializer();
    const tag = new TypeTagU16();

    tag.serialize(serializer);

    expect(TypeTag.deserialize(new Deserializer(serializer.getBytes()))).toBeInstanceOf(TypeTagU16);
  });

  test("deserializes a TypeTagU32 correctly", () => {
    const serializer = new Serializer();
    const tag = new TypeTagU32();

    tag.serialize(serializer);

    expect(TypeTag.deserialize(new Deserializer(serializer.getBytes()))).toBeInstanceOf(TypeTagU32);
  });

  test("deserializes a TypeTagU64 correctly", () => {
    const serializer = new Serializer();
    const tag = new TypeTagU64();

    tag.serialize(serializer);

    expect(TypeTag.deserialize(new Deserializer(serializer.getBytes()))).toBeInstanceOf(TypeTagU64);
  });

  test("deserializes a TypeTagU128 correctly", () => {
    const serializer = new Serializer();
    const tag = new TypeTagU128();

    tag.serialize(serializer);

    expect(TypeTag.deserialize(new Deserializer(serializer.getBytes()))).toBeInstanceOf(TypeTagU128);
  });

  test("deserializes a TypeTagU256 correctly", () => {
    const serializer = new Serializer();
    const tag = new TypeTagU256();

    tag.serialize(serializer);

    expect(TypeTag.deserialize(new Deserializer(serializer.getBytes()))).toBeInstanceOf(TypeTagU256);
  });

  test("deserializes a TypeTagAddress correctly", () => {
    const serializer = new Serializer();
    const tag = new TypeTagAddress();

    tag.serialize(serializer);

    expect(TypeTag.deserialize(new Deserializer(serializer.getBytes()))).toBeInstanceOf(TypeTagAddress);
  });

  test("deserializes a TypeTagSigner correctly", () => {
    const serializer = new Serializer();
    const tag = new TypeTagSigner();

    tag.serialize(serializer);

    expect(TypeTag.deserialize(new Deserializer(serializer.getBytes()))).toBeInstanceOf(TypeTagSigner);
  });

  test("deserializes a TypeTagVector correctly", () => {
    const serializer = new Serializer();
    const tag = new TypeTagVector(new TypeTagU32());

    tag.serialize(serializer);
    const deserialized = TypeTag.deserialize(new Deserializer(serializer.getBytes())) as TypeTagVector;
    expect(deserialized).toBeInstanceOf(TypeTagVector);
    expect(deserialized.value).toBeInstanceOf(TypeTagU32);
  });

  test("deserializes a TypeTagStruct correctly", () => {
    const serializer = new Serializer();
    const tag = new TypeTagStruct(StructTag.fromString(expectedTypeTag.string));

    tag.serialize(serializer);
    const deserialized = TypeTag.deserialize(new Deserializer(serializer.getBytes())) as TypeTagStruct;
    expect(deserialized).toBeInstanceOf(TypeTagStruct);
    expect(deserialized.value).toBeInstanceOf(StructTag);
    expect(deserialized.value.address.toHexString()).toEqual(expectedTypeTag.address);
    expect(deserialized.value.module_name.value).toEqual("some_module");
    expect(deserialized.value.name.value).toEqual("SomeResource");
    expect(deserialized.value.type_args.length).toEqual(0);
  });
});
