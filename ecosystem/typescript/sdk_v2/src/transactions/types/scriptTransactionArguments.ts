import { Serializer, Deserializer, Serializable } from "../../bcs";
import { AccountAddress } from "../../core";
import { ScriptTransactionArgumentVariants } from "../../types";

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
}

export class ScriptTransactionArgumentU8 extends ScriptTransactionArgument {
  public readonly value: number;

  constructor(value: number) {
    super();
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(ScriptTransactionArgumentVariants.U8);
    serializer.serializeU8(this.value);
  }

  static load(deserializer: Deserializer): ScriptTransactionArgumentU8 {
    const value = deserializer.deserializeU8();
    return new ScriptTransactionArgumentU8(value);
  }
}

export class ScriptTransactionArgumentU16 extends ScriptTransactionArgument {
  public readonly value: number;

  constructor(value: number) {
    super();
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(ScriptTransactionArgumentVariants.U16);
    serializer.serializeU16(this.value);
  }

  static load(deserializer: Deserializer): ScriptTransactionArgumentU16 {
    const value = deserializer.deserializeU16();
    return new ScriptTransactionArgumentU16(value);
  }
}

export class ScriptTransactionArgumentU32 extends ScriptTransactionArgument {
  public readonly value: number;

  constructor(value: number) {
    super();
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(ScriptTransactionArgumentVariants.U32);
    serializer.serializeU32(this.value);
  }

  static load(deserializer: Deserializer): ScriptTransactionArgumentU32 {
    const value = deserializer.deserializeU32();
    return new ScriptTransactionArgumentU32(value);
  }
}

export class ScriptTransactionArgumentU64 extends ScriptTransactionArgument {
  public readonly value: bigint;

  constructor(value: bigint) {
    super();
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(ScriptTransactionArgumentVariants.U64);
    serializer.serializeU64(this.value);
  }

  static load(deserializer: Deserializer): ScriptTransactionArgumentU64 {
    const value = deserializer.deserializeU64();
    return new ScriptTransactionArgumentU64(value);
  }
}

export class ScriptTransactionArgumentU128 extends ScriptTransactionArgument {
  public readonly value: bigint;

  constructor(value: bigint) {
    super();
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(ScriptTransactionArgumentVariants.U128);
    serializer.serializeU128(this.value);
  }

  static load(deserializer: Deserializer): ScriptTransactionArgumentU128 {
    const value = deserializer.deserializeU128();
    return new ScriptTransactionArgumentU128(value);
  }
}

export class ScriptTransactionArgumentU256 extends ScriptTransactionArgument {
  public readonly value: bigint;

  constructor(value: bigint) {
    super();
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(ScriptTransactionArgumentVariants.U256);
    serializer.serializeU256(this.value);
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
    this.value.serialize(serializer);
  }

  static load(deserializer: Deserializer): ScriptTransactionArgumentAddress {
    const value = AccountAddress.deserialize(deserializer);
    return new ScriptTransactionArgumentAddress(value);
  }
}

export class ScriptTransactionArgumentU8Vector extends ScriptTransactionArgument {
  public readonly value: Uint8Array;

  constructor(value: Uint8Array) {
    super();
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(ScriptTransactionArgumentVariants.U8Vector);
    serializer.serializeBytes(this.value);
  }

  static load(deserializer: Deserializer): ScriptTransactionArgumentU8Vector {
    const value = deserializer.deserializeBytes();
    return new ScriptTransactionArgumentU8Vector(value);
  }
}

export class ScriptTransactionArgumentBool extends ScriptTransactionArgument {
  public readonly value: boolean;

  constructor(value: boolean) {
    super();
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(ScriptTransactionArgumentVariants.Bool);
    serializer.serializeBool(this.value);
  }

  static load(deserializer: Deserializer): ScriptTransactionArgumentBool {
    const value = deserializer.deserializeBool();
    return new ScriptTransactionArgumentBool(value);
  }
}
