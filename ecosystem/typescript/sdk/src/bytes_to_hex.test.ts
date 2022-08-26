import { bytesToHex, hexToBytes } from "./bytes_to_hex.js";

describe("bytesToHex", () => {
  describe("bytesToHex", () => {
    it("should pad", () => {
      expect(bytesToHex(new Uint8Array([0x1, 0x2, 0x3]))).toBe("010203");
    });

    it("should support zero bytes", () => {
      expect(bytesToHex(new Uint8Array([0, 0x2, 0x3]))).toBe("000203");
    });

    it("should support ff", () => {
      expect(bytesToHex(new Uint8Array([0xff, 0x2, 0x3]))).toBe("ff0203");
    });
  });

  describe("hexToBytes", () => {
    it("works with non-zero", () => {
      expect(hexToBytes("abcdef")).toEqual(new Uint8Array([0xab, 0xcd, 0xef]));
    });

    it("zero pads", () => {
      expect(hexToBytes("00102")).toEqual(new Uint8Array([0x0, 0x1, 0x2]));
    });

    it("works with empty", () => {
      expect(hexToBytes("")).toEqual(new Uint8Array([]));
    });

    it("works with null bytes", () => {
      expect(hexToBytes("00")).toEqual(new Uint8Array([0x0]));
    });
  });
});
