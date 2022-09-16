// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { bytesToHex, hexToBytes } from "@noble/hashes/utils";
import { HexEncodedBytes } from "./generated";

// eslint-disable-next-line no-use-before-define
export type MaybeHexString = HexString | string | HexEncodedBytes;

/**
 * A util class for working with hex strings.
 * Hex strings are strings that are prefixed with `0x`
 */
export class HexString {
  /// We want to make sure this hexString has the `0x` hex prefix
  private readonly hexString: string;

  /**
   * Creates new hex string from Buffer
   * @param buffer A buffer to convert
   * @returns New HexString
   */
  static fromBuffer(buffer: Uint8Array): HexString {
    return HexString.fromUint8Array(buffer);
  }

  /**
   * Creates new hex string from Uint8Array
   * @param arr Uint8Array to convert
   * @returns New HexString
   */
  static fromUint8Array(arr: Uint8Array): HexString {
    return new HexString(bytesToHex(arr));
  }

  /**
   * Ensures `hexString` is instance of `HexString` class
   * @param hexString String to check
   * @returns New HexString if `hexString` is regular string or `hexString` if it is HexString instance
   * @example
   * ```
   *  const regularString = "string";
   *  const hexString = new HexString("string"); // "0xstring"
   *  HexString.ensure(regularString); // "0xstring"
   *  HexString.ensure(hexString); // "0xstring"
   * ```
   */
  static ensure(hexString: MaybeHexString): HexString {
    if (typeof hexString === "string") {
      return new HexString(hexString);
    }
    return hexString;
  }

  /**
   * Creates new HexString instance from regular string. If specified string already starts with "0x" prefix,
   * it will not add another one
   * @param hexString String to convert
   * @example
   * ```
   *  const string = "string";
   *  new HexString(string); // "0xstring"
   * ```
   */
  constructor(hexString: string | HexEncodedBytes) {
    if (hexString.startsWith("0x")) {
      this.hexString = hexString;
    } else {
      this.hexString = `0x${hexString}`;
    }
  }

  /**
   * Getter for inner hexString
   * @returns Inner hex string
   */
  hex(): string {
    return this.hexString;
  }

  /**
   * Getter for inner hexString without prefix
   * @returns Inner hex string without prefix
   * @example
   * ```
   *  const hexString = new HexString("string"); // "0xstring"
   *  hexString.noPrefix(); // "string"
   * ```
   */
  noPrefix(): string {
    return this.hexString.slice(2);
  }

  /**
   * Overrides default `toString` method
   * @returns Inner hex string
   */
  toString(): string {
    return this.hex();
  }

  /**
   * Trimmes extra zeroes in the begining of a string
   * @returns Inner hexString without leading zeroes
   * @example
   * ```
   *  new HexString("0x000000string").toShortString(); // result = "0xstring"
   * ```
   */
  toShortString(): string {
    const trimmed = this.hexString.replace(/^0x0*/, "");
    return `0x${trimmed}`;
  }

  /**
   * Converts hex string to a Uint8Array
   * @returns Uint8Array from inner hexString without prefix
   */
  toUint8Array(): Uint8Array {
    return Uint8Array.from(hexToBytes(this.noPrefix()));
  }
}
