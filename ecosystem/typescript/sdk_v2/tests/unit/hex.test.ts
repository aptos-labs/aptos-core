import { ParsingError } from "../../src/core";
import { Hex, HexInvalidReason } from "../../src/core/hex";

const mockHex = {
  withoutPrefix: "007711b4d0",
  withPrefix: "0x007711b4d0",
  bytes: new Uint8Array([0, 119, 17, 180, 208]),
};

test("creates a new Hex instance from bytes", () => {
  const hex = new Hex({ data: mockHex.bytes });
  expect(hex.toUint8Array()).toEqual(mockHex.bytes);
});

test("creates a new Hex instance from string", () => {
  const hex = new Hex({ data: mockHex.bytes });
  expect(hex.toString()).toEqual(mockHex.withPrefix);
});

test("converts hex bytes input into hex data", () => {
  const hex = new Hex({ data: mockHex.bytes });
  expect(hex instanceof Hex).toBeTruthy();
  expect(hex.toUint8Array()).toEqual(mockHex.bytes);
});

test("converts hex string input into hex data", () => {
  const hex = Hex.fromString({ str: mockHex.withPrefix });
  expect(hex instanceof Hex).toBeTruthy();
  expect(hex.toUint8Array()).toEqual(mockHex.bytes);
});

test("accepts hex string input without prefix", () => {
  const hex = Hex.fromString({ str: mockHex.withoutPrefix });
  expect(hex instanceof Hex).toBeTruthy();
  expect(hex.toUint8Array()).toEqual(mockHex.bytes);
});

test("accepts hex string with prefix", () => {
  const hex = Hex.fromString({ str: mockHex.withPrefix });
  expect(hex instanceof Hex).toBeTruthy();
  expect(hex.toUint8Array()).toEqual(mockHex.bytes);
});

test("converts hex string to bytes", () => {
  const hex = Hex.fromHexInput({ hexInput: mockHex.withPrefix }).toUint8Array();
  expect(hex instanceof Uint8Array).toBeTruthy();
  expect(hex).toEqual(mockHex.bytes);
});

test("converts hex bytes to string", () => {
  const hex = Hex.fromHexInput({ hexInput: mockHex.bytes }).toString();
  expect(typeof hex).toEqual("string");
  expect(hex).toEqual(mockHex.withPrefix);
});

test("converts hex bytes to string without 0x prefix", () => {
  const hex = Hex.fromHexInput({ hexInput: mockHex.withPrefix }).toStringWithoutPrefix();
  expect(hex).toEqual(mockHex.withoutPrefix);
});

test("throws when parsing invalid hex char", () => {
  expect(() => Hex.fromString({ str: "0xzyzz" })).toThrow(
    "Hex string contains invalid hex characters: Invalid byte sequence",
  );
});

test("throws when parsing hex of length zero", () => {
  expect(() => Hex.fromString({ str: "0x" })).toThrow(
    "Hex string is too short, must be at least 1 char long, excluding the optional leading 0x.",
  );
  expect(() => Hex.fromString({ str: "" })).toThrow(
    "Hex string is too short, must be at least 1 char long, excluding the optional leading 0x.",
  );
});

test("throws when parsing hex of invalid length", () => {
  expect(() => Hex.fromString({ str: "0x1" })).toThrow("Hex string must be an even number of hex characters.");
});

test("isValid returns true when parsing valid string", () => {
  const result = Hex.isValid({ str: "0x11aabb" });
  expect(result.valid).toBe(true);
  expect(result.invalidReason).toBeUndefined();
  expect(result.invalidReasonMessage).toBeUndefined();
});

test("isValid returns false when parsing hex of invalid length", () => {
  const result = Hex.isValid({ str: "0xa" });
  expect(result.valid).toBe(false);
  expect(result.invalidReason).toBe(HexInvalidReason.INVALID_LENGTH);
  expect(result.invalidReasonMessage).toBe("Hex string must be an even number of hex characters.");
});

test("compares equality with equals as expected", () => {
  const hexOne = Hex.fromString({ str: "0x11" });
  const hexTwo = Hex.fromString({ str: "0x11" });
  expect(hexOne.equals(hexTwo)).toBeTruthy();
});
