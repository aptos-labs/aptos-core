import { Deserializer, Seq, Serializer, deserializeVector, serializeVector } from "../bcs";
import { AccountAddress } from "./account_address";
import { Identifier } from "./identifier";

export abstract class TypeTag {
  abstract serialize(serializer: Serializer): void;

  static deserialize(deserializer: Deserializer): TypeTag {
    const index = deserializer.deserializeVariantIndex();
    switch (index) {
      case 0:
        return TypeTagVariantbool.load(deserializer);
      case 1:
        return TypeTagVariantu8.load(deserializer);
      case 2:
        return TypeTagVariantu64.load(deserializer);
      case 3:
        return TypeTagVariantu128.load(deserializer);
      case 4:
        return TypeTagVariantaddress.load(deserializer);
      case 5:
        return TypeTagVariantsigner.load(deserializer);
      case 6:
        return TypeTagVariantvector.load(deserializer);
      case 7:
        return TypeTagVariantstruct.load(deserializer);
      default:
        throw new Error(`Unknown variant index for TypeTag: ${index}`);
    }
  }
}

export class TypeTagVariantbool extends TypeTag {
  constructor() {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeVariantIndex(0);
  }

  static load(deserializer: Deserializer): TypeTagVariantbool {
    return new TypeTagVariantbool();
  }
}

export class TypeTagVariantu8 extends TypeTag {
  constructor() {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeVariantIndex(1);
  }

  static load(deserializer: Deserializer): TypeTagVariantu8 {
    return new TypeTagVariantu8();
  }
}

export class TypeTagVariantu64 extends TypeTag {
  constructor() {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeVariantIndex(2);
  }

  static load(deserializer: Deserializer): TypeTagVariantu64 {
    return new TypeTagVariantu64();
  }
}

export class TypeTagVariantu128 extends TypeTag {
  constructor() {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeVariantIndex(3);
  }

  static load(deserializer: Deserializer): TypeTagVariantu128 {
    return new TypeTagVariantu128();
  }
}

export class TypeTagVariantaddress extends TypeTag {
  constructor() {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeVariantIndex(4);
  }

  static load(deserializer: Deserializer): TypeTagVariantaddress {
    return new TypeTagVariantaddress();
  }
}

export class TypeTagVariantsigner extends TypeTag {
  constructor() {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeVariantIndex(5);
  }

  static load(deserializer: Deserializer): TypeTagVariantsigner {
    return new TypeTagVariantsigner();
  }
}

export class TypeTagVariantvector extends TypeTag {
  constructor(public readonly value: TypeTag) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeVariantIndex(6);
    this.value.serialize(serializer);
  }

  static load(deserializer: Deserializer): TypeTagVariantvector {
    const value = TypeTag.deserialize(deserializer);
    return new TypeTagVariantvector(value);
  }
}

export class TypeTagVariantstruct extends TypeTag {
  constructor(public readonly value: StructTag) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeVariantIndex(7);
    this.value.serialize(serializer);
  }

  static load(deserializer: Deserializer): TypeTagVariantstruct {
    const value = StructTag.deserialize(deserializer);
    return new TypeTagVariantstruct(value);
  }
}

export class StructTag {
  constructor(
    public readonly address: AccountAddress,
    public readonly module_name: Identifier,
    public readonly name: Identifier,
    public readonly type_args: Seq<TypeTag>,
  ) {}

  /**
   * Converts a string literal to a StructTag
   * @param structTag String literal in format "AcountAddress::ModuleName::ResourceName",
   *   e.g. "0x01::TestCoin::TestCoin"
   * @returns
   */
  static fromString(structTag: string): StructTag {
    // Type args are not supported in string literal
    if (structTag.includes("<")) {
      throw new Error("Not implemented");
    }

    const parts = structTag.split("::");
    if (parts.length !== 3) {
      throw new Error("Invalid struct tag string literal.");
    }

    return new StructTag(AccountAddress.fromHex(parts[0]), new Identifier(parts[1]), new Identifier(parts[2]), []);
  }

  serialize(serializer: Serializer): void {
    this.address.serialize(serializer);
    this.module_name.serialize(serializer);
    this.name.serialize(serializer);
    serializeVector<TypeTag>(this.type_args, serializer);
  }

  static deserialize(deserializer: Deserializer): StructTag {
    const address = AccountAddress.deserialize(deserializer);
    const module_name = Identifier.deserialize(deserializer);
    const name = Identifier.deserialize(deserializer);
    const type_args = deserializeVector(deserializer, TypeTag);
    return new StructTag(address, module_name, name, type_args);
  }
}
