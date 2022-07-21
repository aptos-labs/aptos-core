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
      `${ADDRESS_1}::aptos_coin`,
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
    "000000000000000000000000000000000000000000000000000000000a550c1800000000000000000300000000000000000000000000000000000000000000000000000000000012220a6170746f735f636f696e087472616e7366657200022000000000000000000000000000000000000000000000000000000000000000dd080100000000000000d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a492004061bb6440bfbdfac3fff8559704303bd72544794b432ab7f9d0f3f779b6cb01aad5c86b6574f04a00698d01f4102015de056a480addd57aab600c3d4d2cba580c",
  );
});

test("serialize script function payload with type args", () => {
  const token = new TypeTagStruct(StructTag.fromString(`${ADDRESS_4}::aptos_coin::AptosCoin`));

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
    "000000000000000000000000000000000000000000000000000000000a550c18000000000000000003000000000000000000000000000000000000000000000000000000000000122204636f696e087472616e73666572010700000000000000000000000000000000000000000000000000000000000000010a6170746f735f636f696e094170746f73436f696e00022000000000000000000000000000000000000000000000000000000000000000dd080100000000000000d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a4920040d7b32e9efbc640963782b11833159a3d62ba962c3f1e5580a9bab89ab012d99c38ed54ab8c0383a438a9a562b3b4b519bd31265130f2955f744125929ff23307",
  );
});

test("serialize script function payload with type args but no function args", () => {
  const token = new TypeTagStruct(StructTag.fromString(`${ADDRESS_4}::aptos_coin::AptosCoin`));

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
    "000000000000000000000000000000000000000000000000000000000a550c18000000000000000003000000000000000000000000000000000000000000000000000000000000122204636f696e0966616b655f66756e63010700000000000000000000000000000000000000000000000000000000000000010a6170746f735f636f696e094170746f73436f696e0000d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a49200401609f53eab9a0ba128ab203e7d2a635390b16c16cef55b6d704a7acdb3ecf0d50fe2285eb5c8ebfca0575f6da65b91efc0a44ebd385868f04a21b28028e12101",
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
  const token = new TypeTagStruct(StructTag.fromString(`${ADDRESS_4}::aptos_coin::AptosCoin`));

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
    "000000000000000000000000000000000000000000000000000000000a550c1800000000000000000126a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102010700000000000000000000000000000000000000000000000000000000000000010a6170746f735f636f696e094170746f73436f696e0000d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a4920040a8f35028bc99eff3896d076e603f67fb1caad39aaf0c44f535fd46f647980698c934885843ec52abea69d0159a9e8c9f7f379fd42d309a571c6184557238f10a",
  );
});

test("serialize script payload with type arg and function arg", () => {
  const token = new TypeTagStruct(StructTag.fromString(`${ADDRESS_4}::aptos_coin::AptosCoin`));

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
    "000000000000000000000000000000000000000000000000000000000a550c1800000000000000000126a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102010700000000000000000000000000000000000000000000000000000000000000010a6170746f735f636f696e094170746f73436f696e00010002d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a4920040662b626455b62ca41ef35b34c74ef0b848c5b3679ae3cf32af47d10ef3372ed4060cfaaeee6ab71ab0034951c21e589d70512c8c536625f532ebf9f127867209",
  );
});

test("serialize script payload with one type arg and two function args", () => {
  const token = new TypeTagStruct(StructTag.fromString(`${ADDRESS_4}::aptos_coin::AptosCoin`));

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
    "000000000000000000000000000000000000000000000000000000000a550c1800000000000000000126a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102010700000000000000000000000000000000000000000000000000000000000000010a6170746f735f636f696e094170746f73436f696e000204080100000000000000030000000000000000000000000000000000000000000000000000000000000001d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a4920040df17e1241a61001ea00141f96bdb0a8584fc68792db6297b1941b244d22accdeb344c2d575aea950046d18aacf056c234c054e32d096f22dd7151ba0e3fdc00e",
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
