import { Buffer } from "buffer/"; // the trailing slash is important!
import { Types } from "./types";

// eslint-disable-next-line no-use-before-define
export type MaybeHexString = HexString | string | Types.HexEncodedBytes;

export class HexString {
  /// We want to make sure this hexString has the `0x` hex prefix
  private readonly hexString: string;

  static fromBuffer(buffer: Buffer): HexString {
    return new HexString(buffer.toString("hex"));
  }

  static fromUint8Array(arr: Uint8Array): HexString {
    return HexString.fromBuffer(Buffer.from(arr));
  }

  static ensure(hexString: MaybeHexString): HexString {
    if (typeof hexString === "string") {
      return new HexString(hexString);
    }
    return hexString;
  }

  constructor(hexString: string | Types.HexEncodedBytes) {
    if (hexString.startsWith("0x")) {
      this.hexString = hexString;
    } else {
      this.hexString = `0x${hexString}`;
    }
  }

  hex(): string {
    return this.hexString;
  }

  noPrefix(): string {
    return this.hexString.slice(2);
  }

  toString(): string {
    return this.hex();
  }

  toShortString(): string {
    const trimmed = this.hexString.replace(/^0x0*/, "");
    return `0x${trimmed}`;
  }

  toBuffer(): Buffer {
    return Buffer.from(this.noPrefix(), "hex");
  }

  toUint8Array(): Uint8Array {
    return Uint8Array.from(this.toBuffer());
  }
}
