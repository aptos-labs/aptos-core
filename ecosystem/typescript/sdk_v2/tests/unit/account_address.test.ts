// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { AccountAddress, AddressInvalidReason } from "../../src/core/account_address";

type Addresses = {
  shortWith0x: string;
  shortWithout0x: string;
  longWith0x: string;
  longWithout0x: string;
  bytes: Uint8Array;
};

// Special addresses.

const ADDRESS_ZERO: Addresses = {
  shortWith0x: "0x0",
  shortWithout0x: "0",
  longWith0x: "0x0000000000000000000000000000000000000000000000000000000000000000",
  longWithout0x: "0000000000000000000000000000000000000000000000000000000000000000",
  bytes: new Uint8Array([
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  ]),
};

const ADDRESS_ONE: Addresses = {
  shortWith0x: "0x1",
  shortWithout0x: "1",
  longWith0x: "0x0000000000000000000000000000000000000000000000000000000000000001",
  longWithout0x: "0000000000000000000000000000000000000000000000000000000000000001",
  bytes: new Uint8Array([
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
  ]),
};

const ADDRESS_F: Addresses = {
  shortWith0x: "0xf",
  shortWithout0x: "f",
  longWith0x: "0x000000000000000000000000000000000000000000000000000000000000000f",
  longWithout0x: "000000000000000000000000000000000000000000000000000000000000000f",
  bytes: new Uint8Array([
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 15,
  ]),
};

const ADDRESS_F_PADDED_SHORT_FORM: Addresses = {
  shortWith0x: "0x0f",
  shortWithout0x: "0f",
  longWith0x: "0x000000000000000000000000000000000000000000000000000000000000000f",
  longWithout0x: "000000000000000000000000000000000000000000000000000000000000000f",
  bytes: new Uint8Array([
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 15,
  ]),
};

// Non-special addresses.

const ADDRESS_TEN: Addresses = {
  shortWith0x: "0x10",
  shortWithout0x: "10",
  longWith0x: "0x0000000000000000000000000000000000000000000000000000000000000010",
  longWithout0x: "0000000000000000000000000000000000000000000000000000000000000010",
  bytes: new Uint8Array([
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16,
  ]),
};

const ADDRESS_OTHER: Addresses = {
  shortWith0x: "0xca843279e3427144cead5e4d5999a3d0ca843279e3427144cead5e4d5999a3d0",
  shortWithout0x: "ca843279e3427144cead5e4d5999a3d0ca843279e3427144cead5e4d5999a3d0",
  // These are the same as the short variants.
  longWith0x: "0xca843279e3427144cead5e4d5999a3d0ca843279e3427144cead5e4d5999a3d0",
  longWithout0x: "ca843279e3427144cead5e4d5999a3d0ca843279e3427144cead5e4d5999a3d0",
  bytes: new Uint8Array([
    202, 132, 50, 121, 227, 66, 113, 68, 206, 173, 94, 77, 89, 153, 163, 208, 202, 132, 50, 121, 227, 66, 113, 68, 206,
    173, 94, 77, 89, 153, 163, 208,
  ]),
};

// These tests show that fromStringRelaxed works happily parses all formats.
describe("AccountAddress fromStringRelaxed", () => {
  it("parses special address: 0x0", () => {
    expect(AccountAddress.fromStringRelaxed({ input: ADDRESS_ZERO.longWith0x }).toString()).toBe(
      ADDRESS_ZERO.shortWith0x,
    );
    expect(AccountAddress.fromStringRelaxed({ input: ADDRESS_ZERO.longWithout0x }).toString()).toBe(
      ADDRESS_ZERO.shortWith0x,
    );
    expect(AccountAddress.fromStringRelaxed({ input: ADDRESS_ZERO.shortWith0x }).toString()).toBe(
      ADDRESS_ZERO.shortWith0x,
    );
    expect(AccountAddress.fromStringRelaxed({ input: ADDRESS_ZERO.shortWithout0x }).toString()).toBe(
      ADDRESS_ZERO.shortWith0x,
    );
  });

  it("parses special address: 0x1", () => {
    expect(AccountAddress.fromStringRelaxed({ input: ADDRESS_ONE.longWith0x }).toString()).toBe(
      ADDRESS_ONE.shortWith0x,
    );
    expect(AccountAddress.fromStringRelaxed({ input: ADDRESS_ONE.longWithout0x }).toString()).toBe(
      ADDRESS_ONE.shortWith0x,
    );
    expect(AccountAddress.fromStringRelaxed({ input: ADDRESS_ONE.shortWith0x }).toString()).toBe(
      ADDRESS_ONE.shortWith0x,
    );
    expect(AccountAddress.fromStringRelaxed({ input: ADDRESS_ONE.shortWithout0x }).toString()).toBe(
      ADDRESS_ONE.shortWith0x,
    );
  });

  it("parses special address: 0xf", () => {
    expect(AccountAddress.fromStringRelaxed({ input: ADDRESS_F.longWith0x }).toString()).toBe(ADDRESS_F.shortWith0x);
    expect(AccountAddress.fromStringRelaxed({ input: ADDRESS_F.longWithout0x }).toString()).toBe(ADDRESS_F.shortWith0x);
    expect(AccountAddress.fromStringRelaxed({ input: ADDRESS_F.shortWith0x }).toString()).toBe(ADDRESS_F.shortWith0x);
    expect(AccountAddress.fromStringRelaxed({ input: ADDRESS_F.shortWithout0x }).toString()).toBe(
      ADDRESS_F.shortWith0x,
    );
  });

  it("parses special address with padded short form: 0x0f", () => {
    expect(AccountAddress.fromStringRelaxed({ input: ADDRESS_F_PADDED_SHORT_FORM.shortWith0x }).toString()).toBe(
      ADDRESS_F.shortWith0x,
    );
    expect(AccountAddress.fromStringRelaxed({ input: ADDRESS_F_PADDED_SHORT_FORM.shortWithout0x }).toString()).toBe(
      ADDRESS_F.shortWith0x,
    );
  });

  it("parses non-special address: 0x10", () => {
    expect(AccountAddress.fromStringRelaxed({ input: ADDRESS_TEN.longWith0x }).toString()).toBe(ADDRESS_TEN.longWith0x);
    expect(AccountAddress.fromStringRelaxed({ input: ADDRESS_TEN.longWithout0x }).toString()).toBe(
      ADDRESS_TEN.longWith0x,
    );
    expect(AccountAddress.fromStringRelaxed({ input: ADDRESS_TEN.shortWith0x }).toString()).toBe(
      ADDRESS_TEN.longWith0x,
    );
    expect(AccountAddress.fromStringRelaxed({ input: ADDRESS_TEN.shortWithout0x }).toString()).toBe(
      ADDRESS_TEN.longWith0x,
    );
  });

  it("parses non-special address: 0xca843279e3427144cead5e4d5999a3d0ca843279e3427144cead5e4d5999a3d0", () => {
    expect(AccountAddress.fromStringRelaxed({ input: ADDRESS_OTHER.longWith0x }).toString()).toBe(
      ADDRESS_OTHER.longWith0x,
    );
    expect(AccountAddress.fromStringRelaxed({ input: ADDRESS_OTHER.longWithout0x }).toString()).toBe(
      ADDRESS_OTHER.longWith0x,
    );
  });
});

// These tests show that fromString only parses addresses with a leading 0x and only
// SHORT if it is a special address.
describe("AccountAddress fromString", () => {
  it("parses special address: 0x0", () => {
    expect(AccountAddress.fromString({ input: ADDRESS_ZERO.longWith0x }).toString()).toBe(ADDRESS_ZERO.shortWith0x);
    expect(() => AccountAddress.fromString({ input: ADDRESS_ZERO.longWithout0x })).toThrow();
    expect(AccountAddress.fromString({ input: ADDRESS_ZERO.shortWith0x }).toString()).toBe(ADDRESS_ZERO.shortWith0x);
    expect(() => AccountAddress.fromString({ input: ADDRESS_ZERO.shortWithout0x })).toThrow();
  });

  it("parses special address: 0x1", () => {
    expect(AccountAddress.fromString({ input: ADDRESS_ONE.longWith0x }).toString()).toBe(ADDRESS_ONE.shortWith0x);
    expect(() => AccountAddress.fromString({ input: ADDRESS_ONE.longWithout0x })).toThrow();
    expect(AccountAddress.fromString({ input: ADDRESS_ONE.shortWith0x }).toString()).toBe(ADDRESS_ONE.shortWith0x);
    expect(() => AccountAddress.fromString({ input: ADDRESS_ONE.shortWithout0x })).toThrow();
  });

  it("parses special address: 0xf", () => {
    expect(AccountAddress.fromString({ input: ADDRESS_F.longWith0x }).toString()).toBe(ADDRESS_F.shortWith0x);
    expect(() => AccountAddress.fromString({ input: ADDRESS_F.longWithout0x })).toThrow();
    expect(AccountAddress.fromString({ input: ADDRESS_F.shortWith0x }).toString()).toBe(ADDRESS_F.shortWith0x);
    expect(() => AccountAddress.fromString({ input: ADDRESS_F.shortWithout0x })).toThrow();
  });

  it("throws when parsing special address with padded short form: 0x0f", () => {
    expect(() => AccountAddress.fromString({ input: ADDRESS_F_PADDED_SHORT_FORM.shortWith0x })).toThrow();
    expect(() => AccountAddress.fromString({ input: ADDRESS_F_PADDED_SHORT_FORM.shortWithout0x })).toThrow();
  });

  it("parses non-special address: 0x10", () => {
    expect(AccountAddress.fromString({ input: ADDRESS_TEN.longWith0x }).toString()).toBe(ADDRESS_TEN.longWith0x);
    expect(() => AccountAddress.fromString({ input: ADDRESS_TEN.longWithout0x })).toThrow();
    expect(() => AccountAddress.fromString({ input: ADDRESS_TEN.shortWith0x })).toThrow();
    expect(() => AccountAddress.fromString({ input: ADDRESS_TEN.shortWithout0x })).toThrow();
  });

  it("parses non-special address: 0xca843279e3427144cead5e4d5999a3d0ca843279e3427144cead5e4d5999a3d0", () => {
    expect(AccountAddress.fromString({ input: ADDRESS_OTHER.longWith0x }).toString()).toBe(ADDRESS_OTHER.longWith0x);
    expect(() => AccountAddress.fromString({ input: ADDRESS_OTHER.longWithout0x })).toThrow();
  });
});

describe("AccountAddress fromHexInput", () => {
  it("parses special address: 0x1", () => {
    expect(AccountAddress.fromHexInput({ input: ADDRESS_ONE.longWith0x }).toString()).toBe(ADDRESS_ONE.shortWith0x);
    expect(() => AccountAddress.fromHexInput({ input: ADDRESS_ONE.longWithout0x })).toThrow();
    expect(AccountAddress.fromHexInput({ input: ADDRESS_ONE.shortWith0x }).toString()).toBe(ADDRESS_ONE.shortWith0x);
    expect(() => AccountAddress.fromHexInput({ input: ADDRESS_ONE.shortWithout0x })).toThrow();
    expect(AccountAddress.fromHexInput({ input: ADDRESS_ONE.bytes }).toString()).toBe(ADDRESS_ONE.shortWith0x);
  });

  it("parses non-special address: 0x10", () => {
    expect(AccountAddress.fromHexInput({ input: ADDRESS_TEN.longWith0x }).toString()).toBe(ADDRESS_TEN.longWith0x);
    expect(() => AccountAddress.fromHexInput({ input: ADDRESS_TEN.longWithout0x })).toThrow();
    expect(() => AccountAddress.fromHexInput({ input: ADDRESS_TEN.shortWith0x })).toThrow();
    expect(() => AccountAddress.fromHexInput({ input: ADDRESS_TEN.shortWithout0x })).toThrow();
    expect(AccountAddress.fromHexInput({ input: ADDRESS_TEN.bytes }).toString()).toBe(ADDRESS_TEN.longWith0x);
  });

  it("parses non-special address: 0xca843279e3427144cead5e4d5999a3d0ca843279e3427144cead5e4d5999a3d0", () => {
    expect(AccountAddress.fromHexInput({ input: ADDRESS_OTHER.longWith0x }).toString()).toBe(ADDRESS_OTHER.longWith0x);
    expect(() => AccountAddress.fromHexInput({ input: ADDRESS_OTHER.longWithout0x })).toThrow();
    expect(AccountAddress.fromHexInput({ input: ADDRESS_OTHER.bytes }).toString()).toBe(ADDRESS_OTHER.shortWith0x);
  });
});

describe("AccountAddress fromHexInputRelaxed", () => {
  it("parses special address: 0x1", () => {
    expect(AccountAddress.fromHexInputRelaxed({ input: ADDRESS_ONE.longWith0x }).toString()).toBe(
      ADDRESS_ONE.shortWith0x,
    );
    expect(AccountAddress.fromHexInputRelaxed({ input: ADDRESS_ONE.longWithout0x }).toString()).toBe(
      ADDRESS_ONE.shortWith0x,
    );
    expect(AccountAddress.fromHexInputRelaxed({ input: ADDRESS_ONE.shortWith0x }).toString()).toBe(
      ADDRESS_ONE.shortWith0x,
    );
    expect(AccountAddress.fromHexInputRelaxed({ input: ADDRESS_ONE.shortWithout0x }).toString()).toBe(
      ADDRESS_ONE.shortWith0x,
    );
    expect(AccountAddress.fromHexInputRelaxed({ input: ADDRESS_ONE.bytes }).toString()).toBe(ADDRESS_ONE.shortWith0x);
  });

  it("parses non-special address: 0x10", () => {
    expect(AccountAddress.fromHexInputRelaxed({ input: ADDRESS_TEN.longWith0x }).toString()).toBe(
      ADDRESS_TEN.longWith0x,
    );
    expect(AccountAddress.fromHexInputRelaxed({ input: ADDRESS_TEN.longWithout0x }).toString()).toBe(
      ADDRESS_TEN.longWith0x,
    );
    expect(AccountAddress.fromHexInputRelaxed({ input: ADDRESS_TEN.shortWith0x }).toString()).toBe(
      ADDRESS_TEN.longWith0x,
    );
    expect(AccountAddress.fromHexInputRelaxed({ input: ADDRESS_TEN.shortWithout0x }).toString()).toBe(
      ADDRESS_TEN.longWith0x,
    );
    expect(AccountAddress.fromHexInputRelaxed({ input: ADDRESS_TEN.bytes }).toString()).toBe(ADDRESS_TEN.longWith0x);
  });

  it("parses non-special address: 0xca843279e3427144cead5e4d5999a3d0ca843279e3427144cead5e4d5999a3d0", () => {
    expect(AccountAddress.fromHexInputRelaxed({ input: ADDRESS_OTHER.longWith0x }).toString()).toBe(
      ADDRESS_OTHER.longWith0x,
    );
    expect(AccountAddress.fromHexInputRelaxed({ input: ADDRESS_OTHER.longWithout0x }).toString()).toBe(
      ADDRESS_OTHER.longWith0x,
    );
    expect(AccountAddress.fromHexInputRelaxed({ input: ADDRESS_OTHER.bytes }).toString()).toBe(
      ADDRESS_OTHER.longWith0x,
    );
  });
});

describe("AccountAddress toUint8Array", () => {
  it("correctly returns bytes for special address: 0x1", () => {
    expect(AccountAddress.fromHexInput({ input: ADDRESS_ONE.longWith0x }).toUint8Array()).toEqual(ADDRESS_ONE.bytes);
  });

  it("correctly returns bytes for  non-special address: 0x10", () => {
    expect(AccountAddress.fromHexInput({ input: ADDRESS_TEN.longWith0x }).toUint8Array()).toEqual(ADDRESS_TEN.bytes);
  });

  it("correctly returns bytes for  non-special address: 0xca843279e3427144cead5e4d5999a3d0ca843279e3427144cead5e4d5999a3d0", () => {
    expect(AccountAddress.fromHexInput({ input: ADDRESS_OTHER.longWith0x }).toUint8Array()).toEqual(
      ADDRESS_OTHER.bytes,
    );
  });
});

describe("AccountAddress toStringWithoutPrefix", () => {
  it("formats special address correctly: 0x0", () => {
    const addr = AccountAddress.fromString({ input: ADDRESS_ZERO.shortWith0x });
    expect(addr.toStringWithoutPrefix()).toBe(ADDRESS_ZERO.shortWithout0x);
  });

  it("formats non-special address correctly: 0x10", () => {
    const addr = AccountAddress.fromString({ input: ADDRESS_TEN.longWith0x });
    expect(addr.toStringWithoutPrefix()).toBe(ADDRESS_TEN.longWithout0x);
  });
});

describe("AccountAddress toStringLong", () => {
  it("formats special address correctly: 0x0", () => {
    const addr = AccountAddress.fromString({ input: ADDRESS_ZERO.shortWith0x });
    expect(addr.toStringLong()).toBe(ADDRESS_ZERO.longWith0x);
  });

  it("formats non-special address correctly: 0x10", () => {
    const addr = AccountAddress.fromString({ input: ADDRESS_TEN.longWith0x });
    expect(addr.toStringLong()).toBe(ADDRESS_TEN.longWith0x);
  });
});

describe("AccountAddress toStringLongWithoutPrefix", () => {
  it("formats special address correctly: 0x0", () => {
    const addr = AccountAddress.fromString({ input: ADDRESS_ZERO.shortWith0x });
    expect(addr.toStringLongWithoutPrefix()).toBe(ADDRESS_ZERO.longWithout0x);
  });

  it("formats non-special address correctly: 0x10", () => {
    const addr = AccountAddress.fromString({ input: ADDRESS_TEN.longWith0x });
    expect(addr.toStringLongWithoutPrefix()).toBe(ADDRESS_TEN.longWithout0x);
  });
});

describe("AccountAddress other parsing", () => {
  it("throws exception when initiating from too long hex string", () => {
    expect(() => {
      AccountAddress.fromString({ input: `${ADDRESS_ONE.longWith0x}1` });
    }).toThrow("Hex string is too long, must be 1 to 64 chars long, excluding the leading 0x.");
  });

  test("throws when parsing invalid hex char", () => {
    expect(() => AccountAddress.fromString({ input: "0xxyz" })).toThrow();
  });

  test("throws when parsing account address of length zero", () => {
    expect(() => AccountAddress.fromString({ input: "0x" })).toThrow();
    expect(() => AccountAddress.fromString({ input: "" })).toThrow();
  });

  test("throws when parsing invalid prefix", () => {
    expect(() => AccountAddress.fromString({ input: "0za" })).toThrow();
  });

  it("isValid is false if too long with 0xf", () => {
    const { valid, invalidReason, invalidReasonMessage } = AccountAddress.isValid({
      input: `0x00${ADDRESS_F.longWithout0x}`,
    });
    expect(valid).toBe(false);
    expect(invalidReason).toBe(AddressInvalidReason.TOO_LONG);
    expect(invalidReasonMessage).toBe("Hex string is too long, must be 1 to 64 chars long, excluding the leading 0x.");
  });

  it("isValid is true if account address string is valid", () => {
    const { valid, invalidReason, invalidReasonMessage } = AccountAddress.isValid({ input: ADDRESS_F.longWith0x });
    expect(valid).toBe(true);
    expect(invalidReason).toBeUndefined();
    expect(invalidReasonMessage).toBeUndefined();
  });

  it("compares equality with equals as expected", () => {
    const addressOne = AccountAddress.fromStringRelaxed({ input: "0x123" });
    const addressTwo = AccountAddress.fromStringRelaxed({ input: "0x123" });
    expect(addressOne.equals(addressTwo)).toBeTruthy();
  });
});
