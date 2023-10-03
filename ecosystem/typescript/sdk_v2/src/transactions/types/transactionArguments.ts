import { Serializer, Deserializer, Serializable } from "../../bcs";
import { AccountAddress } from "../../core";

/**
 * Representation of a Transaction Argument that can serialized and deserialized
 */
export abstract class TransactionArgument extends Serializable {
  /**
   * Serialize a Transaction Argument
   */
  abstract serialize(serializer: Serializer): void;

  /**
   * Deserialize a Transaction Argument
   */
  static deserialize(deserializer: Deserializer): TransactionArgument {
    const index = deserializer.deserializeUleb128AsU32();
    /**
     * index is represented in rust as an enum
     * {@link https://github.com/aptos-labs/aptos-core/blob/main/third_party/move/move-core/types/src/transaction_argument.rs#L11}
     */
    switch (index) {
      case 0:
        return TransactionArgumentU8.load(deserializer);
      case 1:
        return TransactionArgumentU64.load(deserializer);
      case 2:
        return TransactionArgumentU128.load(deserializer);
      case 3:
        return TransactionArgumentAddress.load(deserializer);
      case 4:
        return TransactionArgumentU8Vector.load(deserializer);
      case 5:
        return TransactionArgumentBool.load(deserializer);
      case 6:
        return TransactionArgumentU16.load(deserializer);
      case 7:
        return TransactionArgumentU32.load(deserializer);
      case 8:
        return TransactionArgumentU256.load(deserializer);
      default:
        throw new Error(`Unknown variant index for TransactionArgument: ${index}`);
    }
  }
}

export class TransactionArgumentU8 extends TransactionArgument {
  public readonly value: number;

  constructor(value: number) {
    super();
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(0);
    serializer.serializeU8(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentU8 {
    const value = deserializer.deserializeU8();
    return new TransactionArgumentU8(value);
  }
}

export class TransactionArgumentU16 extends TransactionArgument {
  public readonly value: number;

  constructor(value: number) {
    super();
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(6);
    serializer.serializeU16(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentU16 {
    const value = deserializer.deserializeU16();
    return new TransactionArgumentU16(value);
  }
}

export class TransactionArgumentU32 extends TransactionArgument {
  public readonly value: number;

  constructor(value: number) {
    super();
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(7);
    serializer.serializeU32(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentU32 {
    const value = deserializer.deserializeU32();
    return new TransactionArgumentU32(value);
  }
}

export class TransactionArgumentU64 extends TransactionArgument {
  public readonly value: bigint;

  constructor(value: bigint) {
    super();
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(1);
    serializer.serializeU64(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentU64 {
    const value = deserializer.deserializeU64();
    return new TransactionArgumentU64(value);
  }
}

export class TransactionArgumentU128 extends TransactionArgument {
  public readonly value: bigint;

  constructor(value: bigint) {
    super();
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(2);
    serializer.serializeU128(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentU128 {
    const value = deserializer.deserializeU128();
    return new TransactionArgumentU128(value);
  }
}

export class TransactionArgumentU256 extends TransactionArgument {
  public readonly value: bigint;

  constructor(value: bigint) {
    super();
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(8);
    serializer.serializeU256(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentU256 {
    const value = deserializer.deserializeU256();
    return new TransactionArgumentU256(value);
  }
}

export class TransactionArgumentAddress extends TransactionArgument {
  public readonly value: AccountAddress;

  constructor(value: AccountAddress) {
    super();
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(3);
    this.value.serialize(serializer);
  }

  static load(deserializer: Deserializer): TransactionArgumentAddress {
    const value = AccountAddress.deserialize(deserializer);
    return new TransactionArgumentAddress(value);
  }
}

export class TransactionArgumentU8Vector extends TransactionArgument {
  public readonly value: Uint8Array;

  constructor(value: Uint8Array) {
    super();
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(4);
    serializer.serializeBytes(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentU8Vector {
    const value = deserializer.deserializeBytes();
    return new TransactionArgumentU8Vector(value);
  }
}

export class TransactionArgumentBool extends TransactionArgument {
  public readonly value: boolean;

  constructor(value: boolean) {
    super();
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(5);
    serializer.serializeBool(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentBool {
    const value = deserializer.deserializeBool();
    return new TransactionArgumentBool(value);
  }
}
