import { Hex } from "../core/hex";
import { Deserializable, Deserializer } from "./deserializer";
import { Bool, U128, U16, U256, U32, U64, U8 } from "./serializable_primitives";
import { Serializable, Serializer } from "./serializer";

export type NonGenericInputs = boolean | number | string | bigint;
export type NonGenerics = Bool | U8 | U16 | U32 | U64 | U128 | U256 | MoveString;

export class Vector<T extends Serializable> extends Serializable {
  constructor(public values: Array<T>) {
    super();
  }

  /**
   * Allow for typecasting from an array of primitive inputs to a Vector of the corresponding non-generic class
   *
   * NOTE: This only works with a depth of one. Generics will not work.
   *
   * @example
   * const v = Vector.from([true, false, true], Bool);
   * const v2 = Vector.from([1, 2, 3], U64);
   * const v3 = Vector.from(["abc", "def", "ghi"], MoveString);
   * const v4 = Vector.from([new U64(1), new U64(2), new U64(3)], MoveOption<U64>); // This will NOT work.
   * @params values: an array of primitive values that can be used to create the non-generic Move type T
   * cls: the class to typecast the input values to
   * @returns a Vector of the corresponding class T
   */
  static from<T extends NonGenerics>(values: Array<NonGenericInputs>, cls: new (...args: any[]) => T): Vector<T> {
    return new Vector<T>(values.map((v) => new (cls as any)(v)) as Array<T>);
  }

  /**
   * Allow for typecasting an array of any input to a Vector of the corresponding class,
   * using the second arg as a lambda function.
   *
   * @example
   * const vec = Vector.fromLambda([true, false, undefined], (v) => new MoveOption(v, Bool));
   *
   * // the resulting type below, when serialized, would be vector<vector<Option<bool>>>
   * const vecofVecs = Vector.fromLambda([vec, vec, vec], (v) => new MoveOption(v, Vector));
   * @params values: an array of primitive values that can be used to create the non-generic Move type T
   * cls: the class to typecast the input values to
   * @returns a Vector of the corresponding class T
   */
  static fromLambda<T extends Serializable>(values: Array<any>, f: (...args: any[]) => T): Vector<T> {
    return new Vector<T>(values.map((v) => f(v)) as Array<T>);
  }

  serialize(serializer: Serializer): void {
    serializer.serializeVector(this.values);
  }

  /**
   * Deserialize a Vector of type T, specifically where T is a Serializable and Deserializable type.
   *
   * NOTE: This only works with a depth of one. Generics will not work.
   *
   * NOTE: This will not work with types that aren't of the Serializable class.
   *
   * If you want to use types that merely implement Deserializable,
   * please use the deserializeVector function in the Deserializer class.
   * @example
   * const vec = Vector.deserialize(deserializer, U64);
   * @params deserializer: the Deserializer instance to use, with bytes loaded into it already.
   * cls: the class to typecast the input values to, must be a Serializable and Deserializable type.
   * @returns a Vector of the corresponding class T
   * *
   */
  static deserialize<T extends Serializable>(deserializer: Deserializer, cls: Deserializable<T>): Vector<T> {
    const length = deserializer.deserializeUleb128AsU32();
    const values = new Array<T>();
    for (let i = 0; i < length; i += 1) {
      values.push(cls.deserialize(deserializer));
    }
    return new Vector(values);
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

// NOTES:
// So I actually tried to implement the RotationCapabilityProofOfferChallengeV2 in the unit test,
// and I realized the old way was actually a little more annoying.
// First thing I noticed:
//    the ability to do .toUint8Array() on anything is amazing
// Second thing:
//    having the typed fields is great. It means you don't have to guess what the fields are,
//    it's explicitly a U64 or U8 or whatever. It makes it clear what you're working with in the Move contract
//    but you as the developer don't have to worry about it past the initial class construction.
// Third thing:
//    The ability to serialize and deserialize individual values is really nice. You can now see the individual
//    fields laid out, meaning debugging serialization issues is *much* easier.
//
// The main thing to think about now is if it's acceptable that it doesn't work on *everything* and if we should
// leave the `.from` methods in Vector.
//
// Also named arguments instead of a list of fields in the class: aka
// Serializable will require that each class have a field called MoveFields that is just a dictionary of the fields.
// Then the .serialize() method will just iterate over the fields with Object.keys() or whatever and serialize each
// one of them, since they're all Serializable.
// This makes even *more* sense when you consider that we are trying to move to object args.
//
// The entire thing becomes *very* useful and nice when you start being able to use ABI generated classes to create
// Serializable classes.
// You can just define your ABI in a Move struct/contract, and then generate the Serializable classes from that,
// rather than having to do it by hand.
//
//
// serialize(serializer: Serializer): void {
//   serializer.serialize(this.moduleAddress);
//   serializer.serializeStr(this.moduleName);
//   serializer.serializeStr(this.structName);
//   serializer.serializeStr(this.functionName);
//   serializer.serializeU8(this.chainId);
//   serializer.serializeU64(this.sequenceNumber);
//   serializer.serialize(this.sourceAddress);
//   serializer.serialize(this.recipientAddress);
// }
// I just ran into an issue where I forgot to add `serializer.serializeStr(this.functionName);`
//    This *wouldn't* happen if you had the field structFields and just iterated over them for
//    serialization and deserialization.
//

export class MoveOption<T extends Serializable> extends Serializable {
  private vec: Vector<T>;

  public value: T | undefined;

  constructor(value?: T | NonGenericInputs, cls?: new (...args: any[]) => T) {
    super();
    if (typeof value !== "undefined") {
      if (cls) {
        this.vec = new Vector([new (cls as any)(value)] as T[]);
      } else if (!(value instanceof Serializable)) {
        throw new Error("MoveOption value must be a Serializable object if you do not provide a class to typecast to.");
      } else {
        this.vec = new Vector([value as T]);
      }
    } else {
      this.vec = new Vector([]);
    }

    [this.value] = this.vec.values;
  }

  /**
   * Retrieves the inner value of the MoveOption.
   *
   * This method is inspired by Rust's `Option<T>.unwrap()`.
   * In Rust, attempting to unwrap a `None` value results in a panic.
   *
   * Similarly, this method will throw an error if the value is not present.
   *
   * @example
   * const option = new MoveOption<Bool>(new Bool(true));
   * const value = option.unwrap();  // Returns the Bool instance
   *
   * @throws {Error} Throws an error if the MoveOption does not contain a value.
   *
   * @returns {T} The contained value if present.
   */
  unwrap(): T {
    if (!this.isSome()) {
      throw new Error("Called unwrap on a MoveOption with no value");
    } else {
      return this.value!;
    }
  }

  // Check if the MoveOption has a value.
  isSome(): boolean {
    return this.value !== undefined;
  }

  serialize(serializer: Serializer): void {
    this.vec.serialize(serializer);
  }

  static deserialize<U extends Serializable>(deserializer: Deserializer, cls: Deserializable<U>): MoveOption<U> {
    const vector = Vector.deserialize(deserializer, cls);
    return new MoveOption(vector.values[0]);
  }
}

// TODO: Name MoveObject? Not sure what to call this.
export class MoveObject extends Serializable {
  // this should eventually be value: AccountAddress
  constructor(public value: Hex) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeFixedBytes(this.value.toUint8Array());
  }
}
