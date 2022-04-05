import { HexString } from "./hex_string";

const withoutPrefix = "007711b4d0";
const withPrefix = `0x${withoutPrefix}`;

function validate(hexString: HexString) {
  expect(hexString.hex()).toBe(withPrefix);
  expect(hexString.toString()).toBe(withPrefix);
  expect(`${hexString}`).toBe(withPrefix);
  expect(hexString.noPrefix()).toBe(withoutPrefix);
}

test("from/to buffer", () => {
  const hs = new HexString(withPrefix);
  expect(hs.toBuffer().toString("hex")).toBe(withoutPrefix);
  expect(HexString.fromBuffer(hs.toBuffer()).hex()).toBe(withPrefix);
});

test("from/to Uint8Array", () => {
  const hs = new HexString(withoutPrefix);
  expect(HexString.fromUint8Array(hs.toUint8Array()).hex()).toBe(withPrefix);
});

test("accepts input without prefix", () => {
  const hs = new HexString(withoutPrefix);
  validate(hs);
});

test("accepts input with prefix", () => {
  const hs = new HexString(withPrefix);
  validate(hs);
});

test("ensures input when string", () => {
  const hs = HexString.ensure(withoutPrefix);
  validate(hs);
});

test("ensures input when HexString", () => {
  const hs1 = new HexString(withPrefix);
  const hs = HexString.ensure(hs1);
  validate(hs);
});
