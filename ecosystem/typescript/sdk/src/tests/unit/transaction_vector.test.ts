// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/**
 * Do fuzzing tests with test vectors. The test vectors are produced by the same code
 * used by the Aptos Blockchain. The test vectors are arrays of JSON objects.
 * Each JSON object contains randomized inputs to Transaction Builder and BCS and
 * the expected outputs.
 */

import path from "path";
import nacl from "tweetnacl";
import fs from "fs";
import { bytesToHex } from "@noble/hashes/utils";
import {
  AccountAddress,
  ChainId,
  RawTransaction,
  EntryFunction,
  StructTag,
  TypeTag,
  TypeTagVector,
  TransactionPayloadEntryFunction,
  Identifier,
  TypeTagStruct,
  TypeTagAddress,
  TypeTagBool,
  TypeTagU8,
  TypeTagU64,
  TypeTagU128,
  TypeTagSigner,
  Ed25519Signature,
  TransactionPayloadScript,
  Script,
  TransactionArgument,
  TransactionArgumentBool,
  TransactionArgumentU8,
  TransactionArgumentU64,
  TransactionArgumentAddress,
  TransactionArgumentU8Vector,
  TransactionArgumentU128,
} from "../../aptos_types";
import { HexString } from "../../utils";
import { TransactionBuilderEd25519 } from "../../transaction_builder/builder";

// eslint-disable-next-line operator-linebreak
const VECTOR_FILES_ROOT_DIR =
  process.env.VECTOR_FILES_ROOT_DIR || path.resolve(__dirname, "..", "..", "..", "..", "..", "..", "api", "goldens");

const ENTRY_FUNCTION_VECTOR = path.join(
  VECTOR_FILES_ROOT_DIR,
  "aptos_api__tests__transaction_vector_test__test_entry_function_payload.json",
);

const SCRIPT_VECTOR = path.join(
  VECTOR_FILES_ROOT_DIR,
  "aptos_api__tests__transaction_vector_test__test_script_payload.json",
);

function parseTypeTag(typeTag: any): TypeTag {
  if (typeTag.vector) {
    return new TypeTagVector(parseTypeTag(typeTag.vector));
  }

  if (typeTag.struct) {
    const {
      address,
      module,
      name,
      // eslint-disable-next-line @typescript-eslint/naming-convention
      type_args,
    }: {
      address: string;
      module: string;
      name: string;
      type_args: any[];
    } = typeTag.struct;

    const typeArgs = type_args.map((arg) => parseTypeTag(arg));
    const structTag = new StructTag(
      AccountAddress.fromHex(address),
      new Identifier(module),
      new Identifier(name),
      typeArgs,
    );

    return new TypeTagStruct(structTag);
  }

  switch (typeTag) {
    case "bool":
      return new TypeTagBool();
    case "u8":
      return new TypeTagU8();
    case "u64":
      return new TypeTagU64();
    case "u128":
      return new TypeTagU128();
    case "address":
      return new TypeTagAddress();
    case "signer":
      return new TypeTagSigner();
    default:
      throw new Error("Unknown type tag");
  }
}

function parseTransactionArgument(arg: any): TransactionArgument {
  const argHasOwnProperty = (propertyName: string) => Object.prototype.hasOwnProperty.call(arg, propertyName);
  if (argHasOwnProperty("U8")) {
    // arg.U8 is a number
    return new TransactionArgumentU8(arg.U8);
  }

  if (argHasOwnProperty("U64")) {
    // arg.U64 is a string literal
    return new TransactionArgumentU64(BigInt(arg.U64));
  }

  if (argHasOwnProperty("U128")) {
    // arg.U128 is a string literal
    return new TransactionArgumentU128(BigInt(arg.U128));
  }

  if (argHasOwnProperty("Address")) {
    // arg.Address is a hex string
    return new TransactionArgumentAddress(AccountAddress.fromHex(arg.Address));
  }

  if (argHasOwnProperty("U8Vector")) {
    // arg.U8Vector is a hex string
    return new TransactionArgumentU8Vector(new HexString(arg.U8Vector).toUint8Array());
  }

  if (argHasOwnProperty("Bool")) {
    return new TransactionArgumentBool(arg.Bool);
  }

  throw new Error("Invalid Transaction Argument");
}

function sign(rawTxn: RawTransaction, privateKey: string): string {
  const privateKeyBytes = new HexString(privateKey).toUint8Array();
  const signingKey = nacl.sign.keyPair.fromSeed(privateKeyBytes.slice(0, 32));
  const { publicKey } = signingKey;

  const txnBuilder = new TransactionBuilderEd25519(
    (signingMessage) => new Ed25519Signature(nacl.sign(signingMessage, signingKey.secretKey).slice(0, 64)),
    publicKey,
  );

  return bytesToHex(txnBuilder.sign(rawTxn));
}

type IRawTxn = {
  // hex string for an AccountAddress
  sender: string;
  // u64 string literal
  sequence_number: string;
  // u64 string literal
  max_gas_amount: string;
  // u64 string literal
  gas_unit_price: string;
  // u64 string literal
  expiration_timestamp_secs: string;

  chain_id: number;
};

function verify(
  raw_txn: IRawTxn,
  payload: TransactionPayloadEntryFunction | TransactionPayloadScript,
  private_key: string,
  expected_output: string,
) {
  const rawTxn = new RawTransaction(
    AccountAddress.fromHex(raw_txn.sender),
    BigInt(raw_txn.sequence_number),
    payload,
    BigInt(raw_txn.max_gas_amount),
    BigInt(raw_txn.gas_unit_price),
    BigInt(raw_txn.expiration_timestamp_secs),
    new ChainId(raw_txn.chain_id),
  );

  const signedTxn = sign(rawTxn, private_key);

  expect(signedTxn).toBe(expected_output);
}

describe("Transaction builder vector test", () => {
  it("should pass on entry function payload", () => {
    const vector: any[] = JSON.parse(fs.readFileSync(ENTRY_FUNCTION_VECTOR, "utf8"));
    vector.forEach(({ raw_txn, signed_txn_bcs, private_key }) => {
      const payload = raw_txn.payload.EntryFunction;
      const entryFunctionPayload = new TransactionPayloadEntryFunction(
        EntryFunction.natural(
          `${payload.module.address}::${payload.module.name}`,
          payload.function,
          payload.ty_args.map((tag: any) => parseTypeTag(tag)),
          payload.args.map((arg: any) => new HexString(arg).toUint8Array()),
        ),
      );

      verify(raw_txn, entryFunctionPayload, private_key, signed_txn_bcs);
    });
  });

  it("should pass on script payload", () => {
    const vector: any[] = JSON.parse(fs.readFileSync(SCRIPT_VECTOR, "utf8"));
    vector.forEach(({ raw_txn, signed_txn_bcs, private_key }) => {
      const payload = raw_txn.payload.Script;
      // payload.code is hex string
      const code = new HexString(payload.code).toUint8Array();
      const scriptPayload = new TransactionPayloadScript(
        new Script(
          code,
          payload.ty_args.map((tag: any) => parseTypeTag(tag)),
          payload.args.map((arg: any) => parseTransactionArgument(arg)),
        ),
      );

      verify(raw_txn, scriptPayload, private_key, signed_txn_bcs);
    });
  });
});
