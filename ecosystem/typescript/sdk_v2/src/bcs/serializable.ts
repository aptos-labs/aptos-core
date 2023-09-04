import { from } from "form-data";
import { Deserializer, Serializer } from ".";
import { AnyNumber, Uint16, Uint32, Uint64, Uint128, Uint256 } from "./types";
import { HexInput } from "../types";
import { Hex } from "../core";
import { assert } from "console";


// TODO: Evaluate if using `undefined` as a primitive for auto-typecasting Options makes sense. It may result in unexpected behavior.
export type SerializableOrPrimitive = Serializable | boolean | number | string | bigint | Uint8Array | undefined | null;

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


export interface Deserializable<T> {
  deserialize(deserializer: Deserializer): T;
}

export class Vector<T extends Serializable> extends Serializable {
  constructor(public values: Array<T>) {
    super();
  }

  /**
  * Allow for typecasting from an array of primitives or Serializable values to a Vector of the corresponding class
  * NOTE: This does not have compile-time typechecking, so the compiler will not give you errors if you pass in the wrong type.
  * NOTE: This only works with a depth of one. Do not use generics in the second argument. Vector.from([...], SomeSerializableType<T>) will not work.
  * @example
  * const v = Vector.from([true, false, true], Bool);
  * const v2 = Vector.from([1, 2, 3], U64);
  * const v3 = Vector.from(["abc", "def", "ghi"], MoveString);
  * @params values: an array of primitive or Serializable values
  * cls: the class to typecast the primitive or Serializable values to
  * @returns a Vector of the corresponding class T
  */
  static from<T extends Serializable>(values: Array<SerializableOrPrimitive>, cls: new (...args: any[]) => T): Vector<T> {
    return new Vector<T>(values.map((v) => new (cls as any)(v)) as Array<T>);
  }

  serialize(serializer: Serializer): void {
    serializer.serializeVector(this.values);
  }

  static Bool(values: Array<boolean>): Vector<Bool> {
    return new Vector<Bool>(values.map((v) => new Bool(v)));
  }

  static U8(values: Array<number>): Vector<U8> {
    return new Vector<U8>(values.map((v) => new U8(v)));
  }

  static U16(values: Array<number>): Vector<U16> {
    return new Vector<U16>(values.map((v) => new U16(v)));
  }

  static U32(values: Array<number>): Vector<U32> {
    return new Vector<U32>(values.map((v) => new U32(v)));
  }

  static U64(values: Array<AnyNumber>): Vector<U64> {
    return new Vector<U64>(values.map((v) => new U64(v)));
  }

  static U128(values: Array<AnyNumber>): Vector<U128> {
    return new Vector<U128>(values.map((v) => new U128(v)));
  }

  static U256(values: Array<AnyNumber>): Vector<U256> {
    return new Vector<U256>(values.map((v) => new U256(v)));
  }

  static String(values: Array<string>): Vector<MoveString> {
    return new Vector<MoveString>(values.map((v) => new MoveString(v)));
  }

  static Option<T extends Serializable>(values: Array<SerializableOrPrimitive>, cls: new (...args: any[]) => T): Vector<Option<T>> {
    return new Vector<Option<T>>(values.map((v) => new Option<T>(v, cls)));
  }

  static Object(values: Array<HexInput>): Vector<AptosObject> {
    return new Vector<AptosObject>(values.map((v) => new AptosObject(Hex.fromHexInput({hexInput: v}))));
  }


  static deserialize<U extends Serializable>(deserializer: Deserializer, cls: Deserializable<U>): Vector<U> {
    const values = new Array<U>();
    for (let i = 0; i < length; i++) {
      values.push(cls.deserialize(deserializer));
    }
    return new Vector<U>(values);
  }
}

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


// NEXT STEP:

// Let's remove the cls values from all constructors and maybe `from` functions, or at least separate primitives from serializable values.
// It's interfering with the composability of the serializable values.
// Ultimately we want to focus on that composability, so we can serialize nested objects and vectors easily.
// 
// I think for the most part what you've done is good.
// You just need to
// 1. make it compatible with TypeTag type stuff, either convert those or convert this? most likely convert those
// 2. Make Vectors/Options work with StructTags in the old sdk

export class Option<T extends Serializable> extends Serializable {
  private vec: Vector<T>;

  constructor(value?: T | SerializableOrPrimitive, cls?: new (...args: any[]) => T) {
    super();
    if (typeof value !== 'undefined') {
      if (cls) {
        this.vec = Vector.from([value], cls);
      } else {
        this.vec = new Vector([value as T]);
      }
    } else {
      this.vec = new Vector([]);
    }
  }

  // Check if the Option has a value.
  isSome(): boolean {
    return this.vec.values.length === 1;
  }

  // Get the value, if it exists.
  get(): T | undefined {
    return this.isSome() ? this.vec.values[0] : undefined;
  }

  serialize(serializer: Serializer): void {
    this.vec.serialize(serializer);
  }

  static from<T extends Serializable>(value: SerializableOrPrimitive, cls: new (...args: any[]) => T): Option<T> {
    const vector = Vector.from([value], cls);
    return new Option(vector.values[0]);
  }

  static deserialize<U extends Serializable>(deserializer: Deserializer, cls: Deserializable<U>): Option<U> {
    const vector = Vector.deserialize(deserializer, cls);
    return new Option(vector.values[0]);
  }
}


export class MoveOption<T extends Serializable> extends Serializable {
  public value: Vector<T>;

  constructor(value?: T, cls?: new (...args: any[]) => T) {
    super();
    if (typeof value !== 'undefined') {
      if (typeof cls === 'undefined') {
        this.value = new Vector([value]);
      } else {
        this.value = Vector.from([value], cls);
      }
    } else {
      this.value = new Vector<T>([]);
    }
  }

  serialize(serializer: Serializer): void {
    this.value.serialize(serializer);
  }

  static deserialize<U extends Serializable>(deserializer: Deserializer, cls: Deserializable<U>): MoveOption<U> | null {
    const length = deserializer.deserializeUleb128AsU32();
    const isSome = length === 1;
    if (isSome) {
      const value = cls.deserialize(deserializer);
      return new MoveOption<U>(value);
    } else {
      assert(length === 0, "Invalid MoveOption length, expected 0 or 1, got " + length);
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
