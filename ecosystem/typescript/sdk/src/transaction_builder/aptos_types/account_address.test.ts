import { HexString } from "../../hex_string";
import { AccountAddress } from "./account_address";

const ADDRESS_LONG = "000000000000000000000000000000000000000000000000000000000a550c18";
const ADDRESS_SHORT = "a550c18";

describe("AccountAddress", () => {
  test("gets created from full hex string", async () => {
    const addr = AccountAddress.fromHex(new HexString(ADDRESS_LONG));
    expect(Buffer.from(addr.address).toString("hex")).toBe(ADDRESS_LONG);
  });

  test("gets created from short hex string", async () => {
    const addr = AccountAddress.fromHex(new HexString(ADDRESS_SHORT));
    expect(Buffer.from(addr.address).toString("hex")).toBe(ADDRESS_LONG);
  });

  test("gets created from prefixed full hex string", async () => {
    const addr = AccountAddress.fromHex(new HexString(`0x${ADDRESS_LONG}`));
    expect(Buffer.from(addr.address).toString("hex")).toBe(ADDRESS_LONG);
  });

  test("gets created from prefixed short hex string", async () => {
    const addr = AccountAddress.fromHex(new HexString(`0x${ADDRESS_SHORT}`));
    expect(Buffer.from(addr.address).toString("hex")).toBe(ADDRESS_LONG);
  });

  test("throws exception when initiating from a long hex string", async () => {
    expect(() => {
      AccountAddress.fromHex(new HexString(`1${ADDRESS_LONG}`));
    }).toThrow("Hex string is too long. Address's length is 32 bytes.");
  });
});
