// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AccountAddress } from "./account_address";

const ADDRESS_LONG = "000000000000000000000000000000000000000000000000000000000a550c18";
const ADDRESS_SHORT = "a550c18";

describe("AccountAddress", () => {
  it("gets created from full hex string", async () => {
    const addr = AccountAddress.fromHex(ADDRESS_LONG);
    expect(Buffer.from(addr.address).toString("hex")).toBe(ADDRESS_LONG);
  });

  it("gets created from short hex string", async () => {
    const addr = AccountAddress.fromHex(ADDRESS_SHORT);
    expect(Buffer.from(addr.address).toString("hex")).toBe(ADDRESS_LONG);
  });

  it("gets created from prefixed full hex string", async () => {
    const addr = AccountAddress.fromHex(`0x${ADDRESS_LONG}`);
    expect(Buffer.from(addr.address).toString("hex")).toBe(ADDRESS_LONG);
  });

  it("gets created from prefixed short hex string", async () => {
    const addr = AccountAddress.fromHex(`0x${ADDRESS_SHORT}`);
    expect(Buffer.from(addr.address).toString("hex")).toBe(ADDRESS_LONG);
  });

  it("throws exception when initiating from a long hex string", async () => {
    expect(() => {
      AccountAddress.fromHex(`1${ADDRESS_LONG}`);
      // eslint-disable-next-line quotes
    }).toThrow("Hex string is too long. Address's length is 32 bytes.");
  });
});
