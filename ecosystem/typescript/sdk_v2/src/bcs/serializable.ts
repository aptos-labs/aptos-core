import { from } from "form-data";
import { Deserializable, Deserializer, Serializer } from ".";
import { AnyNumber, Uint16, Uint32, Uint64, Uint128, Uint256 } from "./types";
import { HexInput } from "../types";
import { Hex } from "../core";

// Before, we had TypeTags with enum variant indexes, but since the primitives here aren't enums in Rust,
// they aren't actually serialized like this with the native Rust BCS/Serde implementation.
//
// To emulate the Rust-like BCS serialization, we can use classes that implement Serializable, and then
// serialize them by fetching the runtime metadata, stored initially at runtime with the Serialize() decorator.
// This will facilitate serializing vectors and nested objects/types.
// Normally this wouldn't be possible because typescript doesn't have runtime reflection, but we can use the Serialize() decorator for this.
//
// To ensure type safety, always extend Serializable whenever you use the @Serialize() decorator, or you will likely get runtime errors.
// 
// The bcsTypeRegistry is the metadata storage for all runtime type reflection of serialize and deserialize functions.
const bcsTypeRegistry = new Map<string, { serialize?: Function, deserialize?: Function }>();

// This is the decorator that will register the serialize functions for each class that implements Serializable.
export function Serialize(): ClassDecorator {
  return (target: Function) => {
    const entry = bcsTypeRegistry.get(target.name) || {};
    entry.serialize = target.prototype.serialize;
    bcsTypeRegistry.set(target.name, entry);
  };
}

// This is the decorator that will register the deserialize functions for each class that implements Deserializable.
export function Deserialize(): ClassDecorator {
  return (target: Function) => {
    const entry = bcsTypeRegistry.get(target.name) || {};
    entry.deserialize = target.prototype.deserialize;
    bcsTypeRegistry.set(target.name, entry);
  };
}

// Instead of enums, we can use classes that implement Serializable, to facilitate composable serialization.
// This lets us serialize vectors and nested objects/types very easily. They will all implement the
// Serializable interface, so we can just call serialize on them.
// This abstract class handles the consistency of each subclass, whereas the decorators will handle the extra concerns like type registration.
// To ensure type safety, always extend a class with Serializable whenever you use the @Serialize() decorator, or you will likely get runtime errors.
export abstract class Serializable {
  abstract serialize(serializer: Serializer): void;

  toUint8Array(): Uint8Array {
    const serializer = new Serializer();
    this.serialize(serializer);
    return serializer.toUint8Array();
  }
}

@Serialize()
@Deserialize()
export class Bool extends Serializable {
  constructor(public value: boolean) {
    super();
  }

  serialize(serializer: Serializer): void {
      serializer.serializeBool(this.value);
  }

  static deserialize(deserializer: Deserializer): Bool {
      return new Bool(deserializer.deserializeBool());
  }
}


export class Vector<T extends Serializable> extends Serializable {
  constructor(public value: Array<T>) {
    super();
  }

  serialize(serializer: Serializer): void {
      serializer.serializeVector(this.value);
  }

  static deserialize<U extends Serializable>(deserializer: Deserializer, cls: Deserializable<U>): Vector<U> {
      const values = new Array<U>();
      const length = deserializer.deserializeUleb128AsU32();
      for (let i = 0; i < length; i++) {
          values.push(cls.deserialize(deserializer));
      }
      return new Vector<U>(values);
  }
}

export class U8 extends Serializable {
  constructor(public value: number) {
    super();
  }

  serialize(serializer: Serializer): void {
      serializer.serializeU8(this.value);
  }

  static deserialize(deserializer: Deserializer): U8 {
      return new U8(deserializer.deserializeU8());
  }
}

export class U16 extends Serializable {
  constructor(public value: number) {
    super();
  }

  serialize(serializer: Serializer): void {
      serializer.serializeU16(this.value);
  }

  static deserialize(deserializer: Deserializer): U16 {
      return new U16(deserializer.deserializeU16());
  }
}

export class U32 extends Serializable {
  constructor(public value: number) {
    super();
  }

  serialize(serializer: Serializer): void {
      serializer.serializeU32(this.value);
  }

  static deserialize(deserializer: Deserializer): U32 {
      return new U32(deserializer.deserializeU32());
  }
}

export class U64 extends Serializable {
  value: bigint;

  constructor(value: AnyNumber) {
    super();
    this.value = BigInt(value);
  }

  serialize(serializer: Serializer): void {
      serializer.serializeU64(this.value);
  }

  static deserialize(deserializer: Deserializer): U64 {
      return new U64(deserializer.deserializeU64());
  }
}

export class U128 extends Serializable {
  value: bigint;

  constructor(value: AnyNumber) {
    super();
    this.value = BigInt(value);
  }

  serialize(serializer: Serializer): void {
      serializer.serializeU128(this.value);
  }

  static deserialize(deserializer: Deserializer): U128 {
      return new U128(deserializer.deserializeU128());
  }
}

export class U256 extends Serializable {
  value: bigint;

  constructor(value: AnyNumber) {
    super();
    this.value = BigInt(value);
  }

  serialize(serializer: Serializer): void {
      serializer.serializeU256(this.value);
  }

  static deserialize(deserializer: Deserializer): U256 {
      return new U256(deserializer.deserializeU256());
  }
}

export class MoveString extends Serializable {
  constructor(public value: string) {
    super();
  }

  serialize(serializer: Serializer): void {
      serializer.serializeStr(this.value);
  }

  static deserialize(deserializer: Deserializer): MoveString {
      return new MoveString(deserializer.deserializeStr());
  }
}

export class MoveOption<T extends Serializable> extends Serializable {
  constructor(public value: T | null) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(this.value ? 1 : 0);
    if (this.value) {
      this.value.serialize(serializer);
    }
  }

  static deserialize<T extends Serializable>(deserializer: Deserializer): MoveOption<T> | null {
    const isSome = deserializer.deserializeUleb128AsU32() === 1;
    if (isSome) {
      const 
      const value = deserializer.deserialize(T);
      return new MoveOption<U>(value);
    } else {
      return null;
    }
  }
}

// TODO: Name MoveObject? Not sure what to call this.
export class AptosObject extends Serializable {
  // this should eventually be value: AccountAddress
  constructor(public value: Hex) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeFixedBytes(this.value.toUint8Array());
  }
}
