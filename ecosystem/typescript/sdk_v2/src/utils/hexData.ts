import { bytesToHex, hexToBytes } from "@noble/hashes/utils";
import { Hex } from "../types";

export class HexData {
  private hexData: Hex;

  /**
   * Creates new HexData instance from Hex type.
   * @param hex Uint8Array
   */
  constructor(hex: Hex) {
    this.hexData = hex;
  }

  /**
   * Getter method to get the inner hexData
   * @returns inner hexData
   */
  public get hex(): Hex {
    return this.hexData;
  }

  /**
   * Static method to convert given hex string to HexData
   * @returns HexData
   */
  static fromString(hex: string): HexData {
    if (hex.startsWith("0x")) {
      return new HexData(HexData.toBytes(hex));
    }
    return new HexData(HexData.toBytes(`0x${hex}`));
  }

  /**
   * Static method to convert given hex bytes to HexData
   * @returns HexData
   */
  static fromBytes(hex: Uint8Array): HexData {
    return new HexData(hex);
  }

  /**
   * Static method to convert given hex to without prefix
   * @returns hex without prefix
   */
  static removePrefix(hex: Hex): string {
    return hex.toString().slice(2);
  }

  /**
   * Static method to convert hex to bytes
   * @returns hex as bytes
   */
  static toBytes(hex: Hex): Uint8Array {
    if (hex instanceof Uint8Array) return hex;
    return Uint8Array.from(hexToBytes(HexData.removePrefix(hex)));
  }

  /**
   * Static method to convert hex to string
   * @returns hex as string
   */
  static toString(hex: Hex): string {
    if (hex instanceof Uint8Array) return bytesToHex(hex);
    return hex;
  }
}
