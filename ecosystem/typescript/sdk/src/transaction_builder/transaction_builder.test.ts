import * as ed from "@noble/ed25519";
import { BcsSerializer } from "./bcs";
import { HexString } from "../hex_string";

import { TransactionBuilderEd25519 } from "./index";
import {
  AccountAddress,
  ChainId,
  Identifier,
  Module,
  ModuleBundle,
  ModuleId,
  RawTransaction,
  Script,
  ScriptFunction,
  StructTag,
  TransactionArgumentVariantAddress,
  TransactionArgumentVariantU8,
  TransactionArgumentVariantU8Vector,
  TransactionPayloadVariantModuleBundle,
  TransactionPayloadVariantScript,
  TransactionPayloadVariantScriptFunction,
  TypeTagVariantstruct,
} from "./aptos_types";

const ADDRESS1 = "0x1222";
const ADDRESS2 = "0xdd";
const ADDRESS3 = "0x0a550c18";
const ADDRESS4 = "0x01";
const PRIVATE_KEY = "9bf49a6a0755f953811fce125f2683d50429c3bb49e074147e0089a52eae155f";
const TXN_EXPIRE = "18446744073709551615";

function hexToBytes(hex: string) {
  return new HexString(hex).toUint8Array();
}

function bcsSerializeUint64(i: BigInt): Uint8Array {
  const bcsSerializer = new BcsSerializer();
  bcsSerializer.serializeU64(i);
  return bcsSerializer.getBytes();
}

function bcsSerializeAddress(addr: AccountAddress): Uint8Array {
  const bcsSerializer = new BcsSerializer();
  addr.serialize(bcsSerializer);
  return bcsSerializer.getBytes();
}

async function sign(rawTxn: RawTransaction): Promise<Uint8Array> {
  const privateKey = Uint8Array.from(Buffer.from(PRIVATE_KEY, "hex"));
  const publicKey = await ed.getPublicKey(privateKey);

  const txnBuilder = new TransactionBuilderEd25519(
    async (signingMessage) => ed.sign(signingMessage, privateKey),
    publicKey,
  );

  return txnBuilder.sign(rawTxn);
}

function hexSignedTxn(signedTxn: Uint8Array): string {
  return Buffer.from(signedTxn).toString("hex");
}

test("serialize script function payload with no type args", async () => {
  const bcsSerializer = new BcsSerializer();
  const accountAddress = AccountAddress.fromHex(ADDRESS2);
  accountAddress.serialize(bcsSerializer);

  const scriptFunctionPayload = new TransactionPayloadVariantScriptFunction(
    ScriptFunction.natual(
      `${ADDRESS1}::TestCoin`,
      "transfer",
      [],
      [bcsSerializer.getBytes(), bcsSerializeUint64(BigInt(1))],
    ),
  );

  const rawTxn = new RawTransaction(
    AccountAddress.fromHex(new HexString(ADDRESS3)),
    BigInt(0),
    scriptFunctionPayload,
    BigInt(2000),
    BigInt(BigInt(0)),
    BigInt(TXN_EXPIRE),
    new ChainId(4),
  );

  const signedTxn = await sign(rawTxn);

  expect(hexSignedTxn(signedTxn)).toBe(
    "000000000000000000000000000000000000000000000000000000000a550c1800000000000000000300000000000000000000000000000000000000000000000000000000000012220854657374436f696e087472616e7366657200022000000000000000000000000000000000000000000000000000000000000000dd080100000000000000d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a4920040812ffe56b64e6183f5202c05b9eb8fd2295a1a23ed16a934ef75c1b9e8ebdaafe3dc672e4f8130c192208170e3b891f1a58920f734e1ee9ec05ad86c1efef104",
  );
});

test("serialize script function payload with type args", async () => {
  const moduleName = ModuleId.fromStr(`${ADDRESS1}::Coin`);

  const token = new TypeTagVariantstruct(
    new StructTag(
      AccountAddress.fromHex(new HexString(ADDRESS4)),
      new Identifier("TestCoin"),
      new Identifier("TestCoin"),
      [],
    ),
  );

  const scriptFunctionPayload = new TransactionPayloadVariantScriptFunction(
    new ScriptFunction(
      moduleName,
      new Identifier("transfer"),
      [token],
      [bcsSerializeAddress(AccountAddress.fromHex(ADDRESS2)), bcsSerializeUint64(BigInt(1))],
    ),
  );

  const rawTxn = new RawTransaction(
    AccountAddress.fromHex(new HexString(ADDRESS3)),
    BigInt(0),
    scriptFunctionPayload,
    BigInt(2000),
    BigInt(0),
    BigInt(TXN_EXPIRE),
    new ChainId(4),
  );

  const signedTxn = await sign(rawTxn);

  expect(hexSignedTxn(signedTxn)).toBe(
    "000000000000000000000000000000000000000000000000000000000a550c18000000000000000003000000000000000000000000000000000000000000000000000000000000122204436f696e087472616e73666572010700000000000000000000000000000000000000000000000000000000000000010854657374436f696e0854657374436f696e00022000000000000000000000000000000000000000000000000000000000000000dd080100000000000000d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a49200402d3f812049069c5b7f8e7a77b5281982e788cb8330a4505788a5cb09d070bffd90c4005b81afc78e785d466b3fcb6c2e8fa972440580f3f84e6e75cd6cb0810e",
  );
});

test("serialize script function payload with type args but no function args", async () => {
  const moduleName = ModuleId.fromStr(`${ADDRESS1}::Coin`);

  const token = new TypeTagVariantstruct(
    new StructTag(
      AccountAddress.fromHex(new HexString(ADDRESS4)),
      new Identifier("TestCoin"),
      new Identifier("TestCoin"),
      [],
    ),
  );

  const scriptFunctionPayload = new TransactionPayloadVariantScriptFunction(
    new ScriptFunction(moduleName, new Identifier("fake_func"), [token], []),
  );

  const rawTxn = new RawTransaction(
    AccountAddress.fromHex(ADDRESS3),
    BigInt(0),
    scriptFunctionPayload,
    BigInt(2000),
    BigInt(0),
    BigInt(TXN_EXPIRE),
    new ChainId(4),
  );

  const signedTxn = await sign(rawTxn);

  expect(hexSignedTxn(signedTxn)).toBe(
    "000000000000000000000000000000000000000000000000000000000a550c18000000000000000003000000000000000000000000000000000000000000000000000000000000122204436f696e0966616b655f66756e63010700000000000000000000000000000000000000000000000000000000000000010854657374436f696e0854657374436f696e0000d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a492004031c5c351ff23b0af2b19d1bafc5ab79794a8ef3fc32ec9cdfc48d3d33252e6c629ff1aabadcf5542bd9832695766a02296588fd118484b6b30f786b986e6c602",
  );
});

test("serialize script payload with no type args and no function args", async () => {
  const script = hexToBytes("a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102");

  const scriptPayload = new TransactionPayloadVariantScript(new Script(script, [], []));

  const rawTxn = new RawTransaction(
    AccountAddress.fromHex(ADDRESS3),
    BigInt(0),
    scriptPayload,
    BigInt(2000),
    BigInt(0),
    BigInt(TXN_EXPIRE),
    new ChainId(4),
  );

  const signedTxn = await sign(rawTxn);

  expect(hexSignedTxn(signedTxn)).toBe(
    "000000000000000000000000000000000000000000000000000000000a550c1800000000000000000126a11ceb0b030000000105000100000000050601000000000000000600000000000000001a01020000d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a4920040d698cec385e8696ca2333c75a40c7fb7be053e022d160dd224d36035001d0ea168db48ab7a88900fdbb94fc12ba3ec16d9c16cf85ea97f5370b55a22025e3b0c",
  );
});

test("serialize script payload with type args but no function args", async () => {
  const token = new TypeTagVariantstruct(
    new StructTag(AccountAddress.fromHex(ADDRESS4), new Identifier("TestCoin"), new Identifier("TestCoin"), []),
  );

  const script = hexToBytes("a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102");

  const scriptPayload = new TransactionPayloadVariantScript(new Script(script, [token], []));

  const rawTxn = new RawTransaction(
    AccountAddress.fromHex(ADDRESS3),
    BigInt(0),
    scriptPayload,
    BigInt(2000),
    BigInt(0),
    BigInt(TXN_EXPIRE),
    new ChainId(4),
  );

  const signedTxn = await sign(rawTxn);

  expect(hexSignedTxn(signedTxn)).toBe(
    "000000000000000000000000000000000000000000000000000000000a550c1800000000000000000126a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102010700000000000000000000000000000000000000000000000000000000000000010854657374436f696e0854657374436f696e0000d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a4920040ba5e41864ab77190c078c25050aa43c1b5c36ed6b29b0c2c97370e5104a1a91664e673f4c0834cd3f4a6db81f17f58e6acbc8a26cce35b94d3c6354073d2f10a",
  );
});

test("serialize script payload with one type arg and one function arg", async () => {
  const token = new TypeTagVariantstruct(
    new StructTag(AccountAddress.fromHex(ADDRESS4), new Identifier("TestCoin"), new Identifier("TestCoin"), []),
  );

  const argU8 = new TransactionArgumentVariantU8(2);

  const script = hexToBytes("a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102");

  const scriptPayload = new TransactionPayloadVariantScript(new Script(script, [token], [argU8]));
  const rawTxn = new RawTransaction(
    AccountAddress.fromHex(ADDRESS3),
    BigInt(0),
    scriptPayload,
    BigInt(2000),
    BigInt(0),
    BigInt(TXN_EXPIRE),
    new ChainId(4),
  );

  const signedTxn = await sign(rawTxn);

  expect(hexSignedTxn(signedTxn)).toBe(
    "000000000000000000000000000000000000000000000000000000000a550c1800000000000000000126a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102010700000000000000000000000000000000000000000000000000000000000000010854657374436f696e0854657374436f696e00010002d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a4920040e4c420cd6c6beab55824adbad2ceb7f3e01ba1d20f9952ceda1535df54e66ce4b191b8f85eaf43d6ae41f315ba279004dde3dc519c21395eccdac98af3cb4c04",
  );
});

test("serialize script payload with one type arg and two function args", async () => {
  const token = new TypeTagVariantstruct(
    new StructTag(AccountAddress.fromHex(ADDRESS4), new Identifier("TestCoin"), new Identifier("TestCoin"), []),
  );

  const argU8Vec = new TransactionArgumentVariantU8Vector(bcsSerializeUint64(BigInt(1)));
  const argAddress = new TransactionArgumentVariantAddress(
    AccountAddress.fromHex(new HexString("0000000000000000000000000000000000000000000000000000000000000001")),
  );

  const script = hexToBytes("a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102");

  const scriptPayload = new TransactionPayloadVariantScript(new Script(script, [token], [argU8Vec, argAddress]));

  const rawTxn = new RawTransaction(
    AccountAddress.fromHex(ADDRESS3),
    BigInt(0),
    scriptPayload,
    BigInt(2000),
    BigInt(0),
    BigInt(TXN_EXPIRE),
    new ChainId(4),
  );

  const signedTxn = await sign(rawTxn);

  expect(hexSignedTxn(signedTxn)).toBe(
    "000000000000000000000000000000000000000000000000000000000a550c1800000000000000000126a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102010700000000000000000000000000000000000000000000000000000000000000010854657374436f696e0854657374436f696e000204080100000000000000030000000000000000000000000000000000000000000000000000000000000001d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a4920040665bf547d6d0c3e0a1a12fd685de2fc803fdee21f3413f829dcf0178e56e9e6bb34399fba79915b8374c77f481e4d56c2b173c462545754b57c81477c9a9f70a",
  );
});

test("serialize module payload", async () => {
  const module = hexToBytes(
    "a11ceb0b0300000006010002030205050703070a0c0816100c260900000001000100000102084d794d6f64756c650269640000000000000000000000000b1e55ed00010000000231010200",
  );

  const modulePayload = new TransactionPayloadVariantModuleBundle(new ModuleBundle([new Module(module)]));

  const rawTxn = new RawTransaction(
    AccountAddress.fromHex(ADDRESS3),
    BigInt(0),
    modulePayload,
    BigInt(2000),
    BigInt(0),
    BigInt(TXN_EXPIRE),
    new ChainId(4),
  );

  const signedTxn = await sign(rawTxn);

  expect(hexSignedTxn(signedTxn)).toBe(
    "000000000000000000000000000000000000000000000000000000000a550c18000000000000000002014ba11ceb0b0300000006010002030205050703070a0c0816100c260900000001000100000102084d794d6f64756c650269640000000000000000000000000b1e55ed00010000000231010200d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a4920040bc95e84dd102f6512729646436ebbc37cb74e4796ad0ac8578572029e4798303226a65ca767c0623627e9b57f6c6d526d556f1ca695fde0353709ac38944d20c",
  );
});
