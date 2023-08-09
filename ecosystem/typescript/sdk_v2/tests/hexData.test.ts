import { HexData } from "../src/utils/hexData";

const mockHex = {
  withoutPrefix: "007711b4d0",
  withPrefix: "0x007711b4d0",
  bytes: new Uint8Array([0, 119, 17, 180, 208]),
};

test("creates a new HexData instance from bytes", () => {
  const hd = new HexData(mockHex.bytes);
  expect(hd.hex).toEqual(mockHex.bytes);
});

test("creates a new HexData instance from string", () => {
  const hd = new HexData(mockHex.withPrefix);
  expect(hd.hex).toEqual(mockHex.withPrefix);
});

test("converts hex bytes input into hex data", () => {
  const hd = HexData.fromBytes(mockHex.bytes);
  expect(hd instanceof HexData).toBeTruthy();
  expect(hd.hex).toEqual(mockHex.bytes);
});

test("converts hex string input into hex data", () => {
  const hd = HexData.fromString(mockHex.withPrefix);
  expect(hd instanceof HexData).toBeTruthy();
  expect(hd.hex).toEqual(mockHex.bytes);
});

test("accepts hex string input without prefix", () => {
  const hd = HexData.fromString(mockHex.withoutPrefix);
  expect(hd instanceof HexData).toBeTruthy();
  expect(hd.hex).toEqual(mockHex.bytes);
});

test("accepts hex string with prefix", () => {
  const hd = HexData.fromString(mockHex.withPrefix);
  expect(hd instanceof HexData).toBeTruthy();
  expect(hd.hex).toEqual(mockHex.bytes);
});

test("removes prefix from hex", () => {
  const hd = HexData.removePrefix(mockHex.withPrefix);
  expect(hd).toEqual(mockHex.withoutPrefix);
});

test("converts hex string to bytes", () => {
  const hd = HexData.toBytes(mockHex.withPrefix);
  expect(hd instanceof Uint8Array).toBeTruthy();
  expect(hd).toEqual(mockHex.bytes);
});

test("converts hex bytes to string", () => {
  const hd = HexData.toString(mockHex.bytes);
  expect(typeof hd).toEqual("string");
  expect(hd).toEqual(mockHex.withoutPrefix);
});
