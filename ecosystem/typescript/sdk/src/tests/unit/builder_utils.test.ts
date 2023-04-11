// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { HexString } from "../../utils";
import {
  AccountAddress,
  Identifier,
  StructTag,
  TransactionArgumentAddress,
  TransactionArgumentBool,
  TransactionArgumentU128,
  TransactionArgumentU64,
  TransactionArgumentU8,
  TransactionArgumentU8Vector,
  TypeTagAddress,
  TypeTagBool,
  TypeTagParser,
  TypeTagStruct,
  TypeTagU128,
  TypeTagU16,
  TypeTagU256,
  TypeTagU32,
  TypeTagU64,
  TypeTagU8,
  TypeTagVector,
} from "../../aptos_types";
import { Serializer } from "../../bcs";
import {
  argToTransactionArgument,
  serializeArg,
  ensureBoolean,
  ensureNumber,
  ensureBigInt,
} from "../../transaction_builder/builder_utils";

describe("BuilderUtils", () => {
  it("parses a bool TypeTag", async () => {
    expect(new TypeTagParser("bool").parseTypeTag() instanceof TypeTagBool).toBeTruthy();
  });

  it("parses a u8 TypeTag", async () => {
    expect(new TypeTagParser("u8").parseTypeTag() instanceof TypeTagU8).toBeTruthy();
  });

  it("parses a u16 TypeTag", async () => {
    expect(new TypeTagParser("u16").parseTypeTag() instanceof TypeTagU16).toBeTruthy();
  });

  it("parses a u32 TypeTag", async () => {
    expect(new TypeTagParser("u32").parseTypeTag() instanceof TypeTagU32).toBeTruthy();
  });

  it("parses a u64 TypeTag", async () => {
    expect(new TypeTagParser("u64").parseTypeTag() instanceof TypeTagU64).toBeTruthy();
  });

  it("parses a u128 TypeTag", async () => {
    expect(new TypeTagParser("u128").parseTypeTag() instanceof TypeTagU128).toBeTruthy();
  });

  it("parses a u256 TypeTag", async () => {
    expect(new TypeTagParser("u256").parseTypeTag() instanceof TypeTagU256).toBeTruthy();
  });

  it("parses a address TypeTag", async () => {
    expect(new TypeTagParser("address").parseTypeTag() instanceof TypeTagAddress).toBeTruthy();
  });

  it("parses a vector TypeTag", async () => {
    const vectorAddress = new TypeTagParser("vector<address>").parseTypeTag();
    expect(vectorAddress instanceof TypeTagVector).toBeTruthy();
    expect((vectorAddress as TypeTagVector).value instanceof TypeTagAddress).toBeTruthy();

    const vectorU64 = new TypeTagParser(" vector < u64 > ").parseTypeTag();
    expect(vectorU64 instanceof TypeTagVector).toBeTruthy();
    expect((vectorU64 as TypeTagVector).value instanceof TypeTagU64).toBeTruthy();
  });

  it("parses a sturct TypeTag", async () => {
    const assertStruct = (struct: TypeTagStruct, accountAddress: string, moduleName: string, structName: string) => {
      expect(HexString.fromUint8Array(struct.value.address.address).toShortString()).toBe(accountAddress);
      expect(struct.value.module_name.value).toBe(moduleName);
      expect(struct.value.name.value).toBe(structName);
    };
    const coin = new TypeTagParser("0x1::test_coin::Coin").parseTypeTag();
    expect(coin instanceof TypeTagStruct).toBeTruthy();
    assertStruct(coin as TypeTagStruct, "0x1", "test_coin", "Coin");

    const aptosCoin = new TypeTagParser(
      "0x1::coin::CoinStore < 0x1::test_coin::AptosCoin1 ,  0x1::test_coin::AptosCoin2 > ",
    ).parseTypeTag();
    expect(aptosCoin instanceof TypeTagStruct).toBeTruthy();
    assertStruct(aptosCoin as TypeTagStruct, "0x1", "coin", "CoinStore");

    const aptosCoinTrailingComma = new TypeTagParser(
      "0x1::coin::CoinStore < 0x1::test_coin::AptosCoin1 ,  0x1::test_coin::AptosCoin2, > ",
    ).parseTypeTag();
    expect(aptosCoinTrailingComma instanceof TypeTagStruct).toBeTruthy();
    assertStruct(aptosCoinTrailingComma as TypeTagStruct, "0x1", "coin", "CoinStore");

    const structTypeTags = (aptosCoin as TypeTagStruct).value.type_args;
    expect(structTypeTags.length).toBe(2);

    const structTypeTag1 = structTypeTags[0];
    assertStruct(structTypeTag1 as TypeTagStruct, "0x1", "test_coin", "AptosCoin1");

    const structTypeTag2 = structTypeTags[1];
    assertStruct(structTypeTag2 as TypeTagStruct, "0x1", "test_coin", "AptosCoin2");

    const coinComplex = new TypeTagParser(
      // eslint-disable-next-line max-len
      "0x1::coin::CoinStore < 0x2::coin::LPCoin < 0x1::test_coin::AptosCoin1 <u8>, vector<0x1::test_coin::AptosCoin2 > > >",
    ).parseTypeTag();

    expect(coinComplex instanceof TypeTagStruct).toBeTruthy();
    assertStruct(coinComplex as TypeTagStruct, "0x1", "coin", "CoinStore");
    const coinComplexTypeTag = (coinComplex as TypeTagStruct).value.type_args[0];
    assertStruct(coinComplexTypeTag as TypeTagStruct, "0x2", "coin", "LPCoin");

    expect(() => {
      new TypeTagParser("0x1::test_coin").parseTypeTag();
    }).toThrow("Invalid type tag.");

    expect(() => {
      new TypeTagParser("0x1::test_coin::CoinStore<0x1::test_coin::AptosCoin").parseTypeTag();
    }).toThrow("Invalid type tag.");

    expect(() => {
      new TypeTagParser("0x1::test_coin::CoinStore<0x1::test_coin>").parseTypeTag();
    }).toThrow("Invalid type tag.");

    expect(() => {
      new TypeTagParser("0x1:test_coin::AptosCoin").parseTypeTag();
    }).toThrow("Unrecognized token.");

    expect(() => {
      new TypeTagParser("0x!::test_coin::AptosCoin").parseTypeTag();
    }).toThrow("Unrecognized token.");

    expect(() => {
      new TypeTagParser("0x1::test_coin::AptosCoin<").parseTypeTag();
    }).toThrow("Invalid type tag.");

    expect(() => {
      new TypeTagParser("0x1::test_coin::CoinStore<0x1::test_coin::AptosCoin,").parseTypeTag();
    }).toThrow("Invalid type tag.");

    expect(() => {
      new TypeTagParser("").parseTypeTag();
    }).toThrow("Invalid type tag.");

    expect(() => {
      new TypeTagParser("0x1::<::CoinStore<0x1::test_coin::AptosCoin,").parseTypeTag();
    }).toThrow("Invalid type tag.");

    expect(() => {
      new TypeTagParser("0x1::test_coin::><0x1::test_coin::AptosCoin,").parseTypeTag();
    }).toThrow("Invalid type tag.");

    expect(() => {
      new TypeTagParser("u3").parseTypeTag();
    }).toThrow("Invalid type tag.");
  });

  it("serializes a boolean arg", async () => {
    let serializer = new Serializer();
    serializeArg(true, new TypeTagBool(), serializer);
    expect(serializer.getBytes()).toEqual(new Uint8Array([0x01]));
    serializer = new Serializer();
    expect(() => {
      serializeArg(123, new TypeTagBool(), serializer);
    }).toThrow(/Invalid arg/);
  });

  it("serializes a u8 arg", async () => {
    let serializer = new Serializer();
    serializeArg(255, new TypeTagU8(), serializer);
    expect(serializer.getBytes()).toEqual(new Uint8Array([0xff]));

    serializer = new Serializer();
    expect(() => {
      serializeArg("u8", new TypeTagU8(), serializer);
    }).toThrow(/Invalid number string/);
  });

  it("serializes a u16 arg", async () => {
    let serializer = new Serializer();
    serializeArg(0x7fff, new TypeTagU16(), serializer);
    expect(serializer.getBytes()).toEqual(new Uint8Array([0xff, 0x7f]));

    serializer = new Serializer();
    expect(() => {
      serializeArg("u16", new TypeTagU16(), serializer);
    }).toThrow(/Invalid number string/);
  });

  it("serializes a u32 arg", async () => {
    let serializer = new Serializer();
    serializeArg(0x01020304, new TypeTagU32(), serializer);
    expect(serializer.getBytes()).toEqual(new Uint8Array([0x04, 0x03, 0x02, 0x01]));

    serializer = new Serializer();
    expect(() => {
      serializeArg("u32", new TypeTagU32(), serializer);
    }).toThrow(/Invalid number string/);
  });

  it("serializes a u64 arg", async () => {
    let serializer = new Serializer();
    serializeArg(BigInt("18446744073709551615"), new TypeTagU64(), serializer);
    expect(serializer.getBytes()).toEqual(new Uint8Array([0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]));

    serializer = new Serializer();
    expect(() => {
      serializeArg("u64", new TypeTagU64(), serializer);
    }).toThrow(/^Cannot convert/);
  });

  it("serializes a u128 arg", async () => {
    let serializer = new Serializer();
    serializeArg(BigInt("340282366920938463463374607431768211455"), new TypeTagU128(), serializer);
    expect(serializer.getBytes()).toEqual(
      new Uint8Array([0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
    );

    serializer = new Serializer();
    expect(() => {
      serializeArg("u128", new TypeTagU128(), serializer);
    }).toThrow(/^Cannot convert/);
  });

  it("serializes a u256 arg", async () => {
    let serializer = new Serializer();
    serializeArg(
      BigInt("0x0001020304050607080910111213141516171819202122232425262728293031"),
      new TypeTagU256(),
      serializer,
    );
    expect(serializer.getBytes()).toEqual(
      new Uint8Array([
        0x31, 0x30, 0x29, 0x28, 0x27, 0x26, 0x25, 0x24, 0x23, 0x22, 0x21, 0x20, 0x19, 0x18, 0x17, 0x16, 0x15, 0x14,
        0x13, 0x12, 0x11, 0x10, 0x09, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 0x00,
      ]),
    );

    serializer = new Serializer();
    expect(() => {
      serializeArg("u256", new TypeTagU256(), serializer);
    }).toThrow(/^Cannot convert/);
  });

  it("serializes an AccountAddress arg", async () => {
    let serializer = new Serializer();
    serializeArg("0x1", new TypeTagAddress(), serializer);
    expect(HexString.fromUint8Array(serializer.getBytes()).toShortString()).toEqual("0x1");

    serializer = new Serializer();
    serializeArg(AccountAddress.fromHex("0x1"), new TypeTagAddress(), serializer);
    expect(HexString.fromUint8Array(serializer.getBytes()).toShortString()).toEqual("0x1");

    serializer = new Serializer();
    expect(() => {
      serializeArg(123456, new TypeTagAddress(), serializer);
    }).toThrow("Invalid account address.");
  });

  it("serializes a vector arg", async () => {
    let serializer = new Serializer();
    serializeArg([255], new TypeTagVector(new TypeTagU8()), serializer);
    expect(serializer.getBytes()).toEqual(new Uint8Array([0x1, 0xff]));

    serializer = new Serializer();
    serializeArg("abc", new TypeTagVector(new TypeTagU8()), serializer);
    expect(serializer.getBytes()).toEqual(new Uint8Array([0x3, 0x61, 0x62, 0x63]));

    serializer = new Serializer();
    serializeArg(new Uint8Array([0x61, 0x62, 0x63]), new TypeTagVector(new TypeTagU8()), serializer);
    expect(serializer.getBytes()).toEqual(new Uint8Array([0x3, 0x61, 0x62, 0x63]));

    serializer = new Serializer();
    expect(() => {
      serializeArg(123456, new TypeTagVector(new TypeTagU8()), serializer);
    }).toThrow("Invalid vector args.");
  });

  it("serializes a struct arg", async () => {
    let serializer = new Serializer();
    serializeArg(
      "abc",
      new TypeTagStruct(
        new StructTag(AccountAddress.fromHex("0x1"), new Identifier("string"), new Identifier("String"), []),
      ),
      serializer,
    );
    expect(serializer.getBytes()).toEqual(new Uint8Array([0x3, 0x61, 0x62, 0x63]));

    serializer = new Serializer();
    expect(() => {
      serializeArg(
        "abc",
        new TypeTagStruct(
          new StructTag(AccountAddress.fromHex("0x3"), new Identifier("token"), new Identifier("Token"), []),
        ),
        serializer,
      );
    }).toThrow("The only supported struct arg is of type 0x1::string::String");
  });

  it("throws at unrecognized arg types", async () => {
    const serializer = new Serializer();
    expect(() => {
      // @ts-ignore
      serializeArg(123456, "unknown_type", serializer);
    }).toThrow("Unsupported arg type.");
  });

  it("converts a boolean TransactionArgument", async () => {
    const res = argToTransactionArgument(true, new TypeTagBool());
    expect((res as TransactionArgumentBool).value).toEqual(true);
    expect(() => {
      argToTransactionArgument(123, new TypeTagBool());
    }).toThrow(/Invalid arg/);
  });

  it("converts a u8 TransactionArgument", async () => {
    const res = argToTransactionArgument(123, new TypeTagU8());
    expect((res as TransactionArgumentU8).value).toEqual(123);
    expect(() => {
      argToTransactionArgument("u8", new TypeTagBool());
    }).toThrow(/Invalid boolean string/);
  });

  it("converts a u64 TransactionArgument", async () => {
    const res = argToTransactionArgument(123, new TypeTagU64());
    expect((res as TransactionArgumentU64).value).toEqual(BigInt(123));
    expect(() => {
      argToTransactionArgument("u64", new TypeTagU64());
    }).toThrow(/Cannot convert/);
  });

  it("converts a u128 TransactionArgument", async () => {
    const res = argToTransactionArgument(123, new TypeTagU128());
    expect((res as TransactionArgumentU128).value).toEqual(BigInt(123));
    expect(() => {
      argToTransactionArgument("u128", new TypeTagU128());
    }).toThrow(/Cannot convert/);
  });

  it("converts an AccountAddress TransactionArgument", async () => {
    let res = argToTransactionArgument("0x1", new TypeTagAddress()) as TransactionArgumentAddress;
    expect(HexString.fromUint8Array(res.value.address).toShortString()).toEqual("0x1");

    res = argToTransactionArgument(AccountAddress.fromHex("0x2"), new TypeTagAddress()) as TransactionArgumentAddress;
    expect(HexString.fromUint8Array(res.value.address).toShortString()).toEqual("0x2");

    expect(() => {
      argToTransactionArgument(123456, new TypeTagAddress());
    }).toThrow("Invalid account address.");
  });

  it("converts a vector TransactionArgument", async () => {
    const res = argToTransactionArgument(
      new Uint8Array([0x1]),
      new TypeTagVector(new TypeTagU8()),
    ) as TransactionArgumentU8Vector;
    expect(res.value).toEqual(new Uint8Array([0x1]));

    expect(() => {
      argToTransactionArgument(123456, new TypeTagVector(new TypeTagU8()));
    }).toThrow(/.*should be an instance of Uint8Array$/);
  });

  it("throws at unrecognized TransactionArgument types", async () => {
    expect(() => {
      // @ts-ignore
      argToTransactionArgument(123456, "unknown_type");
    }).toThrow("Unknown type for TransactionArgument.");
  });

  it("ensures a boolean", async () => {
    expect(ensureBoolean(false)).toBe(false);
    expect(ensureBoolean(true)).toBe(true);
    expect(ensureBoolean("true")).toBe(true);
    expect(ensureBoolean("false")).toBe(false);
    expect(() => ensureBoolean("True")).toThrow("Invalid boolean string.");
  });

  it("ensures a number", async () => {
    expect(ensureNumber(10)).toBe(10);
    expect(ensureNumber("123")).toBe(123);
    expect(() => ensureNumber("True")).toThrow("Invalid number string.");
  });

  it("ensures a bigint", async () => {
    expect(ensureBigInt(10)).toBe(BigInt(10));
    expect(ensureBigInt("123")).toBe(BigInt(123));
    expect(() => ensureBigInt("True")).toThrow(/^Cannot convert/);
  });
});
