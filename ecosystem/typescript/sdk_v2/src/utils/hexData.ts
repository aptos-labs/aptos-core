import { bytesToHex, hexToBytes } from "@noble/hashes/utils";
import { Hex } from "../types";

export class HexData {
  private hexData: Uint8Array;

  constructor(hex: Uint8Array) {
    this.hexData = hex;
  }

  public get hex(): Uint8Array {
    return this.hexData;
  }

  static validate(hex: Hex) {
    const hexString = hex.toString();
    if (hexString.startsWith("0x")) {
      return new HexData(HexData.fromString(hexString).toBytes());
    }
    return new HexData(HexData.fromString(`0x${hexString}`).toBytes());
  }

  static fromBytes(hex: Uint8Array): HexData {
    return new HexData(hex);
  }

  static fromString(hex: string): HexData {
    return new HexData(hexToBytes(hex));
  }

  toBytes(): Uint8Array {
    return Uint8Array.from(hexToBytes(HexData.noPrefix(this.hexData)));
  }

  toString(): string {
    return bytesToHex(this.toBytes());
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
    const trimmed = this.toString().replace(/^0x0*/, "");
    return `0x${trimmed}`;
  }

  static noPrefix(hex: Hex): string {
    return hex.toString().slice(2);
  }
}
