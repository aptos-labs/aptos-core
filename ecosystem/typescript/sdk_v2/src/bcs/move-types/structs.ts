import { Serializable, Serializer } from "../serializer";
import { Deserializable, Deserializer } from "../deserializer";
import { Bool, U128, U16, U256, U32, U64, U8 } from "./primitives";
import { AnyNumber, HexInput } from "../../types";
import { AccountAddress } from "../../core";

export type NonGenericInputs = boolean | number | string | bigint;
export type NonGenerics = Bool | U8 | U16 | U32 | U64 | U128 | U256 | MoveString;

/**
 * This class is the Aptos Typescript SDK representation of a Move `vector<T>`,
 * where `T` represents either a primitive type (`bool`, `u8`, `u64`, ...)
 * or a BCS-serializable struct itself.
 *
 * It is a BCS-serializable, array-like type that contains an array of values of type `T`,
 * where `T` is a class that implements `Serializable`.
 *
 * The purpose of this class is to facilitate easy construction of BCS-serializable
 * Move `vector<T>` types.
 *
 * @example
 * // in Move: `vector<u8> [1, 2, 3, 4];`
 * const vecOfU8s = new Vector([new U8(1), new U8(2), new U8(3), new U8(4)]);
 * // in Move: `std::bcs::to_bytes(vector<u8> [1, 2, 3, 4]);`
 * const bcsBytes = vecOfU8s.toUint8Array();
 *
 * // vector<Option<u8>> [ std::option::some<u8>(1), std::option::some<u8>(2) ];
 * const vecOfOptionU8s = new Vector([
 *    MoveOption.U8(1),
 *    MoveOption.U8(2),
 * ]);
 *
 * // vector<String> [ std::string::utf8(b"hello"), std::string::utf8(b"world") ];
 * const vecOfStrings = new Vector([new MoveString("hello"), new MoveString("world")]);
 * const vecOfStrings2 = Vector.ofStrings(["hello", "world"]);
 *
 * // vector<vector<u8>> [ vector<u8> [1, 2, 3, 4], vector<u8> [5, 6, 7, 8] ];
 * const vecOfVecs = new Vector<Vector<U8>>([
 *   vecOfU8s,
 *   Vector.ofU8s([1, 2, 3, 4]),
 *   Vector.ofU8s([5, 6, 7, 8]),
 * ]);
 *
 * // where MySerializableStruct is a class you've made that implements Serializable
 * const vecOfSerializableValues = new Vector([
 *   new MySerializableStruct("hello", "world"),
 *   new MySerializableStruct("foo", "bar"),
 * ]);
 * @params
 * values: an Array<T> of values where T is a class that implements Serializable
 * @returns a Vector<T> with the values `values`
 */
export class Vector<T extends Serializable> extends Serializable {
  constructor(public values: Array<T>) {
    super();
  }

  /**
   * Factory method to generate a Vector of U8s from an array of numbers.
   *
   * @example
   * const v = Vector.ofU8s([1, 2, 3, 4]);
   * @params values: an array of `numbers` to convert to U8s
   * @returns a Vector<U8>
   */
  static ofU8s(values: Array<number>): Vector<U8> {
    return new Vector<U8>(values.map((v) => new U8(v)));
  }

  /**
   * Factory method to generate a Vector of U16s from an array of numbers.
   *
   * @example
   * const v = Vector.ofU16s([1, 2, 3, 4]);
   * @params values: an array of `numbers` to convert to U16s
   * @returns a Vector<U16>
   */
  static ofU16s(values: Array<number>): Vector<U16> {
    return new Vector<U16>(values.map((v) => new U16(v)));
  }

  /**
   * Factory method to generate a Vector of U32s from an array of numbers.
   *
   * @example
   * const v = Vector.ofU32s([1, 2, 3, 4]);
   * @params values: an array of `numbers` to convert to U32s
   * @returns a Vector<U32>
   */
  static ofU32s(values: Array<number>): Vector<U32> {
    return new Vector<U32>(values.map((v) => new U32(v)));
  }

  /**
   * Factory method to generate a Vector of U64s from an array of numbers or bigints.
   *
   * @example
   * const v = Vector.ofU64s([1, 2, 3, 4]);
   * @params values: an array of numbers of type `number | bigint` to convert to U64s
   * @returns a Vector<U64>
   */
  static ofU64s(values: Array<AnyNumber>): Vector<U64> {
    return new Vector<U64>(values.map((v) => new U64(v)));
  }

  /**
   * Factory method to generate a Vector of U128s from an array of numbers or bigints.
   *
   * @example
   * const v = Vector.ofU128s([1, 2, 3, 4]);
   * @params values: an array of numbers of type `number | bigint` to convert to U128s
   * @returns a Vector<U128>
   */
  static ofU128s(values: Array<AnyNumber>): Vector<U128> {
    return new Vector<U128>(values.map((v) => new U128(v)));
  }

  /**
   * Factory method to generate a Vector of U256s from an array of numbers or bigints.
   *
   * @example
   * const v = Vector.ofU256s([1, 2, 3, 4]);
   * @params values: an array of numbers of type `number | bigint` to convert to U256s
   * @returns a Vector<U256>
   */
  static ofU256s(values: Array<AnyNumber>): Vector<U256> {
    return new Vector<U256>(values.map((v) => new U256(v)));
  }

  /**
   * Factory method to generate a Vector of Bools from an array of booleans.
   *
   * @example
   * const v = Vector.ofBools([true, false, true, false]);
   * @params values: an array of `numbers` to convert to Bools
   * @returns a Vector<Bool>
   */
  static ofBools(values: Array<boolean>): Vector<Bool> {
    return new Vector<Bool>(values.map((v) => new Bool(v)));
  }

  /**
   * Factory method to generate a Vector of MoveStrings from an array of strings.
   *
   * @example
   * const v = Vector.ofStrings(["hello", "world"]);
   * @params values: an array of `numbers` to convert to MoveStrings
   * @returns a Vector<MoveString>
   */
  static ofStrings(values: Array<string>): Vector<MoveString> {
    return new Vector<MoveString>(values.map((v) => new MoveString(v)));
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

export class MoveOption<T extends Serializable> extends Serializable {
  private vec: Vector<T>;

  public value: T | undefined;

  constructor(value?: T) {
    super();
    if (typeof value !== "undefined") {
      this.vec = new Vector([value]);
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
    // serialize 0 or 1
    // if 1, serialize the value

    if (this.vec) {
      this.vec.serialize(serializer);
    }
  }

  /**
   * Factory method to generate a MoveOption<U8> from a `number` or `undefined`.
   *
   * @example
   * MoveOption.U8(1).isSome() === true;
   * MoveOption.U8().isSome() === false;
   * MoveOption.U8(undefined).isSome() === false;
   * @params value: the value used to fill the MoveOption. If `value` is undefined
   * the resulting MoveOption's .isSome() method will return false.
   * @returns a MoveOption<U8> with an inner value `value`
   */
  static U8(value?: number): MoveOption<U8> {
    return new MoveOption<U8>(value !== undefined ? new U8(value) : undefined);
  }

  /**
   * Factory method to generate a MoveOption<U16> from a `number` or `undefined`.
   *
   * @example
   * MoveOption.U16(1).isSome() === true;
   * MoveOption.U16().isSome() === false;
   * MoveOption.U16(undefined).isSome() === false;
   * @params value: the value used to fill the MoveOption. If `value` is undefined
   * the resulting MoveOption's .isSome() method will return false.
   * @returns a MoveOption<U16> with an inner value `value`
   */
  static U16(value?: number): MoveOption<U16> {
    return new MoveOption<U16>(value !== undefined ? new U16(value) : undefined);
  }

  /**
   * Factory method to generate a MoveOption<U32> from a `number` or `undefined`.
   *
   * @example
   * MoveOption.U32(1).isSome() === true;
   * MoveOption.U32().isSome() === false;
   * MoveOption.U32(undefined).isSome() === false;
   * @params value: the value used to fill the MoveOption. If `value` is undefined
   * the resulting MoveOption's .isSome() method will return false.
   * @returns a MoveOption<U32> with an inner value `value`
   */
  static U32(value?: number): MoveOption<U32> {
    return new MoveOption<U32>(value !== undefined ? new U32(value) : undefined);
  }

  /**
   * Factory method to generate a MoveOption<U64> from a `number` or a `bigint` or `undefined`.
   *
   * @example
   * MoveOption.U64(1).isSome() === true;
   * MoveOption.U64().isSome() === false;
   * MoveOption.U64(undefined).isSome() === false;
   * @params value: the value used to fill the MoveOption. If `value` is undefined
   * the resulting MoveOption's .isSome() method will return false.
   * @returns a MoveOption<U64> with an inner value `value`
   */
  static U64(value?: AnyNumber): MoveOption<U64> {
    return new MoveOption<U64>(value !== undefined ? new U64(value) : undefined);
  }

  /**
   * Factory method to generate a MoveOption<U128> from a `number` or a `bigint` or `undefined`.
   *
   * @example
   * MoveOption.U128(1).isSome() === true;
   * MoveOption.U128().isSome() === false;
   * MoveOption.U128(undefined).isSome() === false;
   * @params value: the value used to fill the MoveOption. If `value` is undefined
   * the resulting MoveOption's .isSome() method will return false.
   * @returns a MoveOption<U128> with an inner value `value`
   */
  static U128(value?: AnyNumber): MoveOption<U128> {
    return new MoveOption<U128>(value !== undefined ? new U128(value) : undefined);
  }

  /**
   * Factory method to generate a MoveOption<U256> from a `number` or a `bigint` or `undefined`.
   *
   * @example
   * MoveOption.U256(1).isSome() === true;
   * MoveOption.U256().isSome() === false;
   * MoveOption.U256(undefined).isSome() === false;
   * @params value: the value used to fill the MoveOption. If `value` is undefined
   * the resulting MoveOption's .isSome() method will return false.
   * @returns a MoveOption<U256> with an inner value `value`
   */
  static U256(value?: AnyNumber): MoveOption<U256> {
    return new MoveOption<U256>(value !== undefined ? new U256(value) : undefined);
  }

  /**
   * Factory method to generate a MoveOption<Bool> from a `boolean` or `undefined`.
   *
   * @example
   * MoveOption.Bool(true).isSome() === true;
   * MoveOption.Bool().isSome() === false;
   * MoveOption.Bool(undefined).isSome() === false;
   * @params value: the value used to fill the MoveOption. If `value` is undefined
   * the resulting MoveOption's .isSome() method will return false.
   * @returns a MoveOption<Bool> with an inner value `value`
   */
  static Bool(value?: boolean): MoveOption<Bool> {
    return new MoveOption<Bool>(value !== undefined ? new Bool(value) : undefined);
  }

  /**
   * Factory method to generate a MoveOption<MoveString> from a `string` or `undefined`.
   *
   * @example
   * MoveOption.String("hello").isSome() === true;
   * MoveOption.String("").isSome() === true;
   * MoveOption.String().isSome() === false;
   * MoveOption.String(undefined).isSome() === false;
   * @params value: the value used to fill the MoveOption. If `value` is undefined
   * the resulting MoveOption's .isSome() method will return false.
   * @returns a MoveOption<MoveString> with an inner value `value`
   */
  static String(value?: string): MoveOption<MoveString> {
    return new MoveOption<MoveString>(value !== undefined ? new MoveString(value) : undefined);
  }

  static deserialize<U extends Serializable>(deserializer: Deserializer, cls: Deserializable<U>): MoveOption<U> {
    const vector = Vector.deserialize(deserializer, cls);
    return new MoveOption(vector.values[0]);
  }
}

export class MoveObject extends Serializable {
  value: AccountAddress;

  constructor(value: HexInput) {
    super();

    this.value = AccountAddress.fromHexInput({ input: value });
  }

  serialize(serializer: Serializer): void {
    serializer.serialize(this.value);
  }
}
