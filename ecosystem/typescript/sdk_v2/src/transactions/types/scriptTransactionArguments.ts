import { Serializer, Deserializer, Serializable } from "../../bcs";
import { AccountAddress } from "../../core";
import { AnyNumber, HexInput, ScriptTransactionArgumentVariants, Uint16, Uint32, Uint8 } from "../../types";
import { U8, U16, U32, U64, U128, U256, Bool } from "../../bcs/serializable/move-primitives";
import { MoveVector } from "../../bcs/serializable/move-structs";
/**
 * Representation of a Script Transaction Argument that can be serialized and deserialized
 */
export abstract class ScriptTransactionArgument extends Serializable {
  /**
   * Serialize a Script Transaction Argument
   */
  abstract serialize(serializer: Serializer): void;

  /**
   * Deserialize a Script Transaction Argument
   */
  static deserialize(deserializer: Deserializer): ScriptTransactionArgument {
    // index enum variant
    const index = deserializer.deserializeUleb128AsU32();
    switch (index) {
      case ScriptTransactionArgumentVariants.U8:
        return ScriptTransactionArgumentU8.load(deserializer);
      case ScriptTransactionArgumentVariants.U64:
        return ScriptTransactionArgumentU64.load(deserializer);
      case ScriptTransactionArgumentVariants.U128:
        return ScriptTransactionArgumentU128.load(deserializer);
      case ScriptTransactionArgumentVariants.Address:
        return ScriptTransactionArgumentAddress.load(deserializer);
      case ScriptTransactionArgumentVariants.U8Vector:
        return ScriptTransactionArgumentU8Vector.load(deserializer);
      case ScriptTransactionArgumentVariants.Bool:
        return ScriptTransactionArgumentBool.load(deserializer);
      case ScriptTransactionArgumentVariants.U16:
        return ScriptTransactionArgumentU16.load(deserializer);
      case ScriptTransactionArgumentVariants.U32:
        return ScriptTransactionArgumentU32.load(deserializer);
      case ScriptTransactionArgumentVariants.U256:
        return ScriptTransactionArgumentU256.load(deserializer);
      default:
        throw new Error(`Unknown variant index for ScriptTransactionArgument: ${index}`);
    }
  }

  static fromMovePrimitive(
    arg: U8 | U16 | U32 | U64 | U128 | U256 | Bool | MoveVector<U8> | AccountAddress,
  ): ScriptTransactionArgument {
    if (arg instanceof U8) {
      return new ScriptTransactionArgumentU8(arg.value);
    }
    if (arg instanceof U16) {
      return new ScriptTransactionArgumentU16(arg.value);
    }
    if (arg instanceof U32) {
      return new ScriptTransactionArgumentU32(arg.value);
    }
    if (arg instanceof U64) {
      return new ScriptTransactionArgumentU64(arg.value);
    }
    if (arg instanceof U128) {
      return new ScriptTransactionArgumentU128(arg.value);
    }
    if (arg instanceof U256) {
      return new ScriptTransactionArgumentU256(arg.value);
    }
    if (arg instanceof Bool) {
      return new ScriptTransactionArgumentBool(arg.value);
    }
    if (arg instanceof AccountAddress) {
      return new ScriptTransactionArgumentAddress(arg);
    }
    if (arg instanceof MoveVector) {
      const allArgsU8 = arg.values.every((v) => v instanceof U8);
      if (allArgsU8) {
        return new ScriptTransactionArgumentU8Vector(arg.values.map((v) => v.value));
      }
      throw new Error("Unsupported vector type");
    }
    throw new Error("Unsupported argument type");
  }
}

export class ScriptTransactionArgumentU8 extends ScriptTransactionArgument {
  public readonly value: U8;

  constructor(value: Uint8) {
    super();
    this.value = new U8(value);
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(ScriptTransactionArgumentVariants.U8);
    serializer.serialize(this.value);
  }

  static load(deserializer: Deserializer): ScriptTransactionArgumentU8 {
    const value = deserializer.deserializeU8();
    return new ScriptTransactionArgumentU8(value);
  }
}

export class ScriptTransactionArgumentU16 extends ScriptTransactionArgument {
  public readonly value: U16;

  constructor(value: Uint16) {
    super();
    this.value = new U16(value);
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(ScriptTransactionArgumentVariants.U16);
    serializer.serialize(this.value);
  }

  static load(deserializer: Deserializer): ScriptTransactionArgumentU16 {
    const value = deserializer.deserializeU16();
    return new ScriptTransactionArgumentU16(value);
  }
}

export class ScriptTransactionArgumentU32 extends ScriptTransactionArgument {
  public readonly value: U32;

  constructor(value: Uint32) {
    super();
    this.value = new U32(value);
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(ScriptTransactionArgumentVariants.U32);
    serializer.serialize(this.value);
  }

  static load(deserializer: Deserializer): ScriptTransactionArgumentU32 {
    const value = deserializer.deserializeU32();
    return new ScriptTransactionArgumentU32(value);
  }
}

export class ScriptTransactionArgumentU64 extends ScriptTransactionArgument {
  public readonly value: U64;

  constructor(value: AnyNumber) {
    super();
    this.value = new U64(value);
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(ScriptTransactionArgumentVariants.U64);
    serializer.serialize(this.value);
  }

  static load(deserializer: Deserializer): ScriptTransactionArgumentU64 {
    const value = deserializer.deserializeU64();
    return new ScriptTransactionArgumentU64(value);
  }
}

export class ScriptTransactionArgumentU128 extends ScriptTransactionArgument {
  public readonly value: U128;

  constructor(value: AnyNumber) {
    super();
    this.value = new U128(value);
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(ScriptTransactionArgumentVariants.U128);
    serializer.serialize(this.value);
  }

  static load(deserializer: Deserializer): ScriptTransactionArgumentU128 {
    const value = deserializer.deserializeU128();
    return new ScriptTransactionArgumentU128(value);
  }
}

export class ScriptTransactionArgumentU256 extends ScriptTransactionArgument {
  public readonly value: U256;

  constructor(value: AnyNumber) {
    super();
    this.value = new U256(value);
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(ScriptTransactionArgumentVariants.U256);
    serializer.serialize(this.value);
  }

  static load(deserializer: Deserializer): ScriptTransactionArgumentU256 {
    const value = deserializer.deserializeU256();
    return new ScriptTransactionArgumentU256(value);
  }
}

export class ScriptTransactionArgumentAddress extends ScriptTransactionArgument {
  public readonly value: AccountAddress;

  constructor(value: AccountAddress) {
    super();
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(ScriptTransactionArgumentVariants.Address);
    serializer.serialize(this.value);
  }

  static load(deserializer: Deserializer): ScriptTransactionArgumentAddress {
    const value = AccountAddress.deserialize(deserializer);
    return new ScriptTransactionArgumentAddress(value);
  }
}

export class ScriptTransactionArgumentU8Vector extends ScriptTransactionArgument {
  public readonly value: MoveVector<U8>;

  constructor(values: Array<number> | HexInput) {
    super();
    this.value = MoveVector.U8(values);
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(ScriptTransactionArgumentVariants.U8Vector);
    serializer.serialize(this.value);
  }

  static load(deserializer: Deserializer): ScriptTransactionArgumentU8Vector {
    const value = deserializer.deserializeBytes();
    return new ScriptTransactionArgumentU8Vector(value);
  }
}

export class ScriptTransactionArgumentBool extends ScriptTransactionArgument {
  public readonly value: Bool;

  constructor(value: boolean) {
    super();
    this.value = new Bool(value);
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(ScriptTransactionArgumentVariants.Bool);
    serializer.serialize(this.value);
  }

  static load(deserializer: Deserializer): ScriptTransactionArgumentBool {
    const value = deserializer.deserializeBool();
    return new ScriptTransactionArgumentBool(value);
  }
}
