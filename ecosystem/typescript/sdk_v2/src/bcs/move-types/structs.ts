import { Serializable, Serializer } from "../serializer";
import { Deserializable, Deserializer } from "../deserializer";
import { Bool, U128, U16, U256, U32, U64, U8 } from "./primitives";
import { AnyNumber, HexInput } from "../../types";
import { AccountAddress } from "../../core";

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
 * const vecOfU8s = new MoveVector<U8>([new U8(1), new U8(2), new U8(3), new U8(4)]);
 * // in Move: `std::bcs::to_bytes(vector<u8> [1, 2, 3, 4]);`
 * const bcsBytes = vecOfU8s.toUint8Array();
 *
 * // vector<vector<u8>> [ vector<u8> [1], vector<u8> [1, 2, 3, 4], vector<u8> [5, 6, 7, 8] ];
 * const vecOfVecs = new MoveVector<MoveVector<U8>>([
 *   new MoveVector<U8>([new U8(1)]),
 *   MoveVector.U8([1, 2, 3, 4]),
 *   MoveVector.U8([5, 6, 7, 8]),
 * ]);
 *
 * // vector<Option<u8>> [ std::option::some<u8>(1), std::option::some<u8>(2) ];
 * const vecOfOptionU8s = new MoveVector<MoveOption<U8>>([
 *    MoveOption.U8(1),
 *    MoveOption.U8(2),
 * ]);
 *
 * // vector<String> [ std::string::utf8(b"hello"), std::string::utf8(b"world") ];
 * const vecOfStrings = new MoveVector([new MoveString("hello"), new MoveString("world")]);
 * const vecOfStrings2 = MoveVector.String(["hello", "world"]);
 *
 * // where MySerializableStruct is a class you've made that implements Serializable
 * const vecOfSerializableValues = new MoveVector<MySerializableStruct>([
 *   new MySerializableStruct("hello", "world"),
 *   new MySerializableStruct("foo", "bar"),
 * ]);
 * @params
 * values: an Array<T> of values where T is a class that implements Serializable
 * @returns a `MoveVector<T>` with the values `values`
 */
export class MoveVector<T extends Serializable> extends Serializable {
  public values: Array<T>;

  constructor(values: Array<T>) {
    super();
    this.values = values;
  }

  /**
   * Factory method to generate a MoveVector of U8s from an array of numbers.
   *
   * @example
   * const v = MoveVector.U8([1, 2, 3, 4]);
   * @params values: an array of `numbers` to convert to U8s
   * @returns a `MoveVector<U8>`
   */
  static U8(values: Array<number>): MoveVector<U8> {
    return new MoveVector<U8>(values.map((v) => new U8(v)));
  }

  /**
   * Factory method to generate a MoveVector of U16s from an array of numbers.
   *
   * @example
   * const v = MoveVector.U16([1, 2, 3, 4]);
   * @params values: an array of `numbers` to convert to U16s
   * @returns a `MoveVector<U16>`
   */
  static U16(values: Array<number>): MoveVector<U16> {
    return new MoveVector<U16>(values.map((v) => new U16(v)));
  }

  /**
   * Factory method to generate a MoveVector of U32s from an array of numbers.
   *
   * @example
   * const v = MoveVector.U32([1, 2, 3, 4]);
   * @params values: an array of `numbers` to convert to U32s
   * @returns a `MoveVector<U32>`
   */
  static U32(values: Array<number>): MoveVector<U32> {
    return new MoveVector<U32>(values.map((v) => new U32(v)));
  }

  /**
   * Factory method to generate a MoveVector of U64s from an array of numbers or bigints.
   *
   * @example
   * const v = MoveVector.U64([1, 2, 3, 4]);
   * @params values: an array of numbers of type `number | bigint` to convert to U64s
   * @returns a `MoveVector<U64>`
   */
  static U64(values: Array<AnyNumber>): MoveVector<U64> {
    return new MoveVector<U64>(values.map((v) => new U64(v)));
  }

  /**
   * Factory method to generate a MoveVector of U128s from an array of numbers or bigints.
   *
   * @example
   * const v = MoveVector.U128([1, 2, 3, 4]);
   * @params values: an array of numbers of type `number | bigint` to convert to U128s
   * @returns a `MoveVector<U128>`
   */
  static U128(values: Array<AnyNumber>): MoveVector<U128> {
    return new MoveVector<U128>(values.map((v) => new U128(v)));
  }

  /**
   * Factory method to generate a MoveVector of U256s from an array of numbers or bigints.
   *
   * @example
   * const v = MoveVector.U256([1, 2, 3, 4]);
   * @params values: an array of numbers of type `number | bigint` to convert to U256s
   * @returns a `MoveVector<U256>`
   */
  static U256(values: Array<AnyNumber>): MoveVector<U256> {
    return new MoveVector<U256>(values.map((v) => new U256(v)));
  }

  /**
   * Factory method to generate a MoveVector of Bools from an array of booleans.
   *
   * @example
   * const v = MoveVector.Bool([true, false, true, false]);
   * @params values: an array of `numbers` to convert to Bools
   * @returns a `MoveVector<Bool>`
   */
  static Bool(values: Array<boolean>): MoveVector<Bool> {
    return new MoveVector<Bool>(values.map((v) => new Bool(v)));
  }

  /**
   * Factory method to generate a MoveVector of MoveStrings from an array of strings.
   *
   * @example
   * const v = MoveVector.String(["hello", "world"]);
   * @params values: an array of `numbers` to convert to MoveStrings
   * @returns a `MoveVector<MoveString>`
   */
  static String(values: Array<string>): MoveVector<MoveString> {
    return new MoveVector<MoveString>(values.map((v) => new MoveString(v)));
  }

  serialize(serializer: Serializer): void {
    serializer.serializeVector(this.values);
  }

  /**
   * Deserialize a MoveVector of type T, specifically where T is a Serializable and Deserializable type.
   *
   * NOTE: This only works with a depth of one. Generics will not work.
   *
   * NOTE: This will not work with types that aren't of the Serializable class.
   *
   * If you want to use types that merely implement Deserializable,
   * please use the deserializeVector function in the Deserializer class.
   * @example
   * const vec = MoveVector.deserialize(deserializer, U64);
   * @params deserializer: the Deserializer instance to use, with bytes loaded into it already.
   * cls: the class to typecast the input values to, must be a Serializable and Deserializable type.
   * @returns a MoveVector of the corresponding class T
   * *
   */
  static deserialize<T extends Serializable>(deserializer: Deserializer, cls: Deserializable<T>): MoveVector<T> {
    const length = deserializer.deserializeUleb128AsU32();
    const values = new Array<T>();
    for (let i = 0; i < length; i += 1) {
      values.push(cls.deserialize(deserializer));
    }
    return new MoveVector(values);
  }
}

export class MoveString extends Serializable {
  public value: string;

  constructor(value: string) {
    super();
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeStr(this.value);
  }

  static deserialize(deserializer: Deserializer): MoveString {
    return new MoveString(deserializer.deserializeStr());
  }
}

export class MoveOption<T extends Serializable> extends Serializable {
  private vec: MoveVector<T>;

  public readonly value?: T;

  constructor(value?: T | null) {
    super();
    if (typeof value !== "undefined" && value !== null) {
      this.vec = new MoveVector([value]);
    } else {
      this.vec = new MoveVector([]);
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
      return this.vec.values[0];
    }
  }

  // Check if the MoveOption has a value.
  isSome(): boolean {
    return this.vec.values.length === 1;
  }

  serialize(serializer: Serializer): void {
    // serialize 0 or 1
    // if 1, serialize the value
    this.vec.serialize(serializer);
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
  static U8(value?: number | null): MoveOption<U8> {
    return new MoveOption<U8>(value !== null && value !== undefined ? new U8(value) : undefined);
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
  static U16(value?: number | null): MoveOption<U16> {
    return new MoveOption<U16>(value !== null && value !== undefined ? new U16(value) : undefined);
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
  static U32(value?: number | null): MoveOption<U32> {
    return new MoveOption<U32>(value !== null && value !== undefined ? new U32(value) : undefined);
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
  static U64(value?: AnyNumber | null): MoveOption<U64> {
    return new MoveOption<U64>(value !== null && value !== undefined ? new U64(value) : undefined);
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
  static U128(value?: AnyNumber | null): MoveOption<U128> {
    return new MoveOption<U128>(value !== null && value !== undefined ? new U128(value) : undefined);
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
  static U256(value?: AnyNumber | null): MoveOption<U256> {
    return new MoveOption<U256>(value !== null && value !== undefined ? new U256(value) : undefined);
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
  static Bool(value?: boolean | null): MoveOption<Bool> {
    return new MoveOption<Bool>(value !== null && value !== undefined ? new Bool(value) : undefined);
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
  static String(value?: string | null): MoveOption<MoveString> {
    return new MoveOption<MoveString>(value !== null && value !== undefined ? new MoveString(value) : undefined);
  }

  static deserialize<U extends Serializable>(deserializer: Deserializer, cls: Deserializable<U>): MoveOption<U> {
    const vector = MoveVector.deserialize(deserializer, cls);
    return new MoveOption(vector.values[0]);
  }
}

export class MoveObject extends Serializable {
  public value: AccountAddress;

  constructor(value: HexInput | AccountAddress) {
    super();

    if (value instanceof AccountAddress) {
      this.value = value;
    } else {
      this.value = AccountAddress.fromHexInput({ input: value });
    }
  }

  serialize(serializer: Serializer): void {
    serializer.serialize(this.value);
  }

  static deserialize(deserializer: Deserializer): MoveObject {
    const address = deserializer.deserialize(AccountAddress);
    return new MoveObject(address);
  }
}
