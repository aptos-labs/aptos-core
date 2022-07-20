/* eslint-disable max-len */
import * as Nacl from "tweetnacl";
import { bcsSerializeUint64, bcsToBytes, Bytes } from "./bcs";
import { HexString } from "../hex_string";

import { TransactionBuilderEd25519, TransactionBuilder } from "./index";
import {
  AccountAddress,
  ChainId,
  Ed25519Signature,
  Module,
  ModuleBundle,
  RawTransaction,
  Script,
  ScriptFunction,
  StructTag,
  TransactionArgumentAddress,
  TransactionArgumentU8,
  TransactionArgumentU8Vector,
  TransactionPayloadModuleBundle,
  TransactionPayloadScript,
  TransactionPayloadScriptFunction,
  TypeTagStruct,
} from "./aptos_types";

const ADDRESS_1 = "0x1222";
const ADDRESS_2 = "0xdd";
const ADDRESS_3 = "0x0a550c18";
const ADDRESS_4 = "0x01";
const PRIVATE_KEY = "9bf49a6a0755f953811fce125f2683d50429c3bb49e074147e0089a52eae155f";
const TXN_EXPIRE = "18446744073709551615";

function hexToBytes(hex: string) {
  return new HexString(hex).toUint8Array();
}

function hexSignedTxn(signedTxn: Uint8Array): string {
  return Buffer.from(signedTxn).toString("hex");
}

function sign(rawTxn: RawTransaction): Bytes {
  const privateKeyBytes = new HexString(PRIVATE_KEY).toUint8Array();
  const signingKey = Nacl.sign.keyPair.fromSeed(privateKeyBytes.slice(0, 32));
  const { publicKey } = signingKey;

  const txnBuilder = new TransactionBuilderEd25519(
    (signingMessage) => new Ed25519Signature(Nacl.sign(signingMessage, signingKey.secretKey).slice(0, 64)),
    publicKey,
  );

  return txnBuilder.sign(rawTxn);
}

test("throws when preparing signing message with invalid payload", () => {
  expect(() => {
    // @ts-ignore
    TransactionBuilder.getSigningMessage("invalid");
  }).toThrow("Unknown transaction type.");
});

test("serialize script function payload with no type args", () => {
  const scriptFunctionPayload = new TransactionPayloadScriptFunction(
    ScriptFunction.natural(
      `${ADDRESS_1}::test_coin`,
      "transfer",
      [],
      [bcsToBytes(AccountAddress.fromHex(ADDRESS_2)), bcsSerializeUint64(1)],
    ),
  );

  const rawTxn = new RawTransaction(
    AccountAddress.fromHex(new HexString(ADDRESS_3)),
    0n,
    scriptFunctionPayload,
    2000n,
    0n,
    BigInt(TXN_EXPIRE),
    new ChainId(4),
  );

  const signedTxn = sign(rawTxn);

  expect(hexSignedTxn(signedTxn)).toBe(
    "000000000000000000000000000000000000000000000000000000000a550c18000000000000000003000000000000000000000000000000000000000000000000000000000000122209746573745f636f696e087472616e7366657200022000000000000000000000000000000000000000000000000000000000000000dd080100000000000000d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a4920040673d2343feed57e2409020d0433fa9ddbe9a790e34456f7f873533ef912b0b4a54f01c7d35f61980933e5c0127bbdf90d6266f86576ef715c9a434215960600d",
  );
});

test("serialize script function payload with type args", () => {
  const token = new TypeTagStruct(StructTag.fromString(`${ADDRESS_4}::test_coin::TestCoin`));

  const scriptFunctionPayload = new TransactionPayloadScriptFunction(
    ScriptFunction.natural(
      `${ADDRESS_1}::coin`,
      "transfer",
      [token],
      [bcsToBytes(AccountAddress.fromHex(ADDRESS_2)), bcsSerializeUint64(1)],
    ),
  );

  const rawTxn = new RawTransaction(
    AccountAddress.fromHex(ADDRESS_3),
    0n,
    scriptFunctionPayload,
    2000n,
    0n,
    BigInt(TXN_EXPIRE),
    new ChainId(4),
  );

  const signedTxn = sign(rawTxn);

  expect(hexSignedTxn(signedTxn)).toBe(
    "000000000000000000000000000000000000000000000000000000000a550c18000000000000000003000000000000000000000000000000000000000000000000000000000000122204636f696e087472616e736665720107000000000000000000000000000000000000000000000000000000000000000109746573745f636f696e0854657374436f696e00022000000000000000000000000000000000000000000000000000000000000000dd080100000000000000d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a49200405df30fc905d88ba1af3e35ddd063a06ef629b582aa321b9dc080f1c511f3a0336a4b416e3d2a86445281e8bb1af38b8b7f2904da9a32da2c2f170fcd90f90b0d",
  );
});

test("serialize script function payload with type args but no function args", () => {
  const token = new TypeTagStruct(StructTag.fromString(`${ADDRESS_4}::test_coin::TestCoin`));

  const scriptFunctionPayload = new TransactionPayloadScriptFunction(
    ScriptFunction.natural(`${ADDRESS_1}::coin`, "fake_func", [token], []),
  );

  const rawTxn = new RawTransaction(
    AccountAddress.fromHex(ADDRESS_3),
    0n,
    scriptFunctionPayload,
    2000n,
    0n,
    BigInt(TXN_EXPIRE),
    new ChainId(4),
  );

  const signedTxn = sign(rawTxn);

  expect(hexSignedTxn(signedTxn)).toBe(
    "000000000000000000000000000000000000000000000000000000000a550c18000000000000000003000000000000000000000000000000000000000000000000000000000000122204636f696e0966616b655f66756e630107000000000000000000000000000000000000000000000000000000000000000109746573745f636f696e0854657374436f696e0000d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a492004005d5111028f24dcdca30ff5bd61fc2c39765a9c7464098016f6e1e789d0e834e5ba8276e3cf3ec1934d96e10e31abcd439bfd72d14807e7e9af0c8ba2b1bf20d",
  );
});

test("serialize script payload with no type args and no function args", () => {
  const script = hexToBytes("a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102");

  const scriptPayload = new TransactionPayloadScript(new Script(script, [], []));

  const rawTxn = new RawTransaction(
    AccountAddress.fromHex(ADDRESS_3),
    0n,
    scriptPayload,
    2000n,
    0n,
    BigInt(TXN_EXPIRE),
    new ChainId(4),
  );

  const signedTxn = sign(rawTxn);

  expect(hexSignedTxn(signedTxn)).toBe(
    "000000000000000000000000000000000000000000000000000000000a550c1800000000000000000126a11ceb0b030000000105000100000000050601000000000000000600000000000000001a01020000d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a4920040d698cec385e8696ca2333c75a40c7fb7be053e022d160dd224d36035001d0ea168db48ab7a88900fdbb94fc12ba3ec16d9c16cf85ea97f5370b55a22025e3b0c",
  );
});

test("serialize script payload with type args but no function args", () => {
  const token = new TypeTagStruct(StructTag.fromString(`${ADDRESS_4}::test_coin::TestCoin`));

  const script = hexToBytes("a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102");

  const scriptPayload = new TransactionPayloadScript(new Script(script, [token], []));

  const rawTxn = new RawTransaction(
    AccountAddress.fromHex(ADDRESS_3),
    0n,
    scriptPayload,
    2000n,
    0n,
    BigInt(TXN_EXPIRE),
    new ChainId(4),
  );

  const signedTxn = sign(rawTxn);

  expect(hexSignedTxn(signedTxn)).toBe(
    "000000000000000000000000000000000000000000000000000000000a550c1800000000000000000126a11ceb0b030000000105000100000000050601000000000000000600000000000000001a01020107000000000000000000000000000000000000000000000000000000000000000109746573745f636f696e0854657374436f696e0000d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a49200404f9c0b74e5d36be002645c03e8758611992435734fe8f6a29009f59073ca7300c260e9611fcd06a47f304e2557317ffda3094d6576918a342d3e78d96027580d",
  );
});

test("serialize script payload with type arg and function arg", () => {
  const token = new TypeTagStruct(StructTag.fromString(`${ADDRESS_4}::test_coin::TestCoin`));

  const argU8 = new TransactionArgumentU8(2);

  const script = hexToBytes("a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102");

  const scriptPayload = new TransactionPayloadScript(new Script(script, [token], [argU8]));
  const rawTxn = new RawTransaction(
    AccountAddress.fromHex(ADDRESS_3),
    0n,
    scriptPayload,
    2000n,
    0n,
    BigInt(TXN_EXPIRE),
    new ChainId(4),
  );

  const signedTxn = sign(rawTxn);

  expect(hexSignedTxn(signedTxn)).toBe(
    "000000000000000000000000000000000000000000000000000000000a550c1800000000000000000126a11ceb0b030000000105000100000000050601000000000000000600000000000000001a01020107000000000000000000000000000000000000000000000000000000000000000109746573745f636f696e0854657374436f696e00010002d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a4920040d5fe884b74b4d0d6272d6a499af4e7cf8a2932c49e69320cc1f94cf83b1eeea8df6ff63f95e0ec3b8185ce552ddb498d61b7a42d14b4fc69aa52bead92b21c0c",
  );
});

test("serialize script payload with one type arg and two function args", () => {
  const token = new TypeTagStruct(StructTag.fromString(`${ADDRESS_4}::test_coin::TestCoin`));

  const argU8Vec = new TransactionArgumentU8Vector(bcsSerializeUint64(1));
  const argAddress = new TransactionArgumentAddress(AccountAddress.fromHex("0x01"));

  const script = hexToBytes("a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102");

  const scriptPayload = new TransactionPayloadScript(new Script(script, [token], [argU8Vec, argAddress]));

  const rawTxn = new RawTransaction(
    AccountAddress.fromHex(ADDRESS_3),
    0n,
    scriptPayload,
    2000n,
    0n,
    BigInt(TXN_EXPIRE),
    new ChainId(4),
  );

  const signedTxn = sign(rawTxn);

  expect(hexSignedTxn(signedTxn)).toBe(
    "000000000000000000000000000000000000000000000000000000000a550c1800000000000000000126a11ceb0b030000000105000100000000050601000000000000000600000000000000001a01020107000000000000000000000000000000000000000000000000000000000000000109746573745f636f696e0854657374436f696e000204080100000000000000030000000000000000000000000000000000000000000000000000000000000001d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a4920040480f9557d54671a25ed25a67ffe39de64af0c27286d96d3119609ac62ec67f5fc8316a6126e16b33ce16646865377c0941b80c23c530f8894bb5a551c6477107",
  );
});

test("serialize module payload", () => {
  const module = hexToBytes(
    "a11ceb0b0300000006010002030205050703070a0c0816100c260900000001000100000102084d794d6f64756c650269640000000000000000000000000b1e55ed00010000000231010200",
  );

  const modulePayload = new TransactionPayloadModuleBundle(new ModuleBundle([new Module(module)]));

  const rawTxn = new RawTransaction(
    AccountAddress.fromHex(ADDRESS_3),
    0n,
    modulePayload,
    2000n,
    0n,
    BigInt(TXN_EXPIRE),
    new ChainId(4),
  );

  const signedTxn = sign(rawTxn);

  expect(hexSignedTxn(signedTxn)).toBe(
    "000000000000000000000000000000000000000000000000000000000a550c18000000000000000002014ba11ceb0b0300000006010002030205050703070a0c0816100c260900000001000100000102084d794d6f64756c650269640000000000000000000000000b1e55ed00010000000231010200d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a4920040bc95e84dd102f6512729646436ebbc37cb74e4796ad0ac8578572029e4798303226a65ca767c0623627e9b57f6c6d526d556f1ca695fde0353709ac38944d20c",
  );
});
