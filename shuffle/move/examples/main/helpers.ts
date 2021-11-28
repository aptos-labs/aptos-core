// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

// deno-lint-ignore-file no-explicit-any
import * as DiemTypes from "./generated/diemTypes/mod.ts";
import { defaultUserContext, UserContext } from "./context.ts";
import * as devapi from "./devapi.ts";
import * as ed from "https://deno.land/x/ed25519@1.0.1/mod.ts";
import * as util from "https://deno.land/std@0.85.0/node/util.ts";
import { BcsSerializer } from "./generated/bcs/mod.ts";
import { bytes, ListTuple, uint8 } from "./generated/serde/types.ts";
import { createHash } from "https://deno.land/std@0.77.0/hash/mod.ts";

const textEncoder = new util.TextEncoder();
const textDecoder = new util.TextDecoder();

export async function buildAndSubmitTransaction(
  addressStr: string,
  sequenceNumber: number,
  privateKeyBytes: Uint8Array,
  payload: DiemTypes.TransactionPayload,
) {
  if (sequenceNumber == undefined) {
    throw "Must pass in parameter sequenceNumber. Try devapi.sequenceNumber()";
  }

  const rawTxn = newRawTransaction(
    addressStr,
    payload,
    sequenceNumber,
  );
  const signingMsg = generateSigningMessage(rawTxn);
  const signedTxnBytes = await newSignedTransaction(
    privateKeyBytes,
    rawTxn,
    signingMsg,
  );

  return await devapi.postTransactionBcs(signedTxnBytes);
}

export function buildScriptFunctionTransaction(
  moduleAddress: string,
  moduleName: string,
  functionName: string,
  tyArgs: DiemTypes.TypeTag[],
  args: bytes[],
): DiemTypes.TransactionPayload {
  const moduleId: DiemTypes.ModuleId = new DiemTypes.ModuleId(
    hexToAccountAddress(moduleAddress),
    new DiemTypes.Identifier(moduleName),
  );
  return new DiemTypes.ScriptFunction(
    moduleId,
    new DiemTypes.Identifier(functionName),
    tyArgs,
    args,
  );
}

// Invokes a script function using the Dev API's signing_message/ JSON endpoint.
export async function invokeScriptFunction(
  scriptFunction: string,
  typeArguments: string[],
  args: any[],
): Promise<any> {
  return await invokeScriptFunctionForContext(
    defaultUserContext,
    scriptFunction,
    typeArguments,
    args,
  );
}

export async function invokeScriptFunctionForContext(
  userContext: UserContext,
  scriptFunction: string,
  typeArguments: string[],
  args: any[],
): Promise<any> {
  return await invokeScriptFunctionForAddress(
    userContext.address,
    await devapi.sequenceNumber(userContext.address),
    await userContext.readPrivateKey(),
    scriptFunction,
    typeArguments,
    args,
  );
}
// Invokes a script function using the Dev API's signing_message/ JSON endpoint.
export async function invokeScriptFunctionForAddress(
  senderAddressStr: string,
  sequenceNumber: number,
  privateKeyBytes: Uint8Array,
  scriptFunction: string,
  typeArguments: string[],
  args: any[],
): Promise<any> {
  const request: any = {
    "sender": senderAddressStr,
    "sequence_number": `${sequenceNumber}`,
    "max_gas_amount": "1000000",
    "gas_unit_price": "0",
    "gas_currency_code": "XUS",
    "expiration_timestamp_secs": "99999999999",
    "payload": {
      "type": "script_function_payload",
      "function": scriptFunction,
      "type_arguments": typeArguments,
      "arguments": normalizeScriptFunctionArgs(args),
    },
  };

  const signingMsgPayload = await devapi.postTransactionSigningMessage(
    JSON.stringify(request),
  );
  const signingMsg = signingMsgPayload.message.slice(2); // remove 0x prefix

  const publicKey = bufferToHex(await ed.getPublicKey(privateKeyBytes));
  const signature = await ed.sign(signingMsg, privateKeyBytes);
  request.signature = {
    "type": "ed25519_signature",
    "public_key": publicKey,
    "signature": signature,
  };

  return await devapi.postTransactionJson(JSON.stringify(request));
}

export function newRawTransaction(
  addressStr: string,
  payload: DiemTypes.TransactionPayload,
  sequenceNumber: number,
): DiemTypes.RawTransaction {
  return new DiemTypes.RawTransaction(
    hexToAccountAddress(addressStr),
    BigInt(sequenceNumber),
    payload, // txn payload
    BigInt(1000000), // max gas amount
    BigInt(0), // gas_unit_price
    "XUS", // currency
    BigInt(99999999999), // expiration_timestamp_secs
    new DiemTypes.ChainId(4), // chain id, hardcoded to test
  );
}

export function hashPrefix(name: string): Uint8Array {
  const hash = createHash("sha3-256");
  hash.update("DIEM::");
  hash.update(name);
  return new Uint8Array(hash.digest());
}

export function generateSigningMessage(
  rawTxn: DiemTypes.RawTransaction,
): Uint8Array {
  const bcsSerializer = new BcsSerializer();
  rawTxn.serialize(bcsSerializer);
  const rawTxnBytes = bcsSerializer.getBytes();

  const signingMsg = appendBuffer(hashPrefix("RawTransaction"), rawTxnBytes);
  return signingMsg;
}

export async function newSignedTransaction(
  privateKeyBytes: Uint8Array,
  rawTxn: DiemTypes.RawTransaction,
  signingMsg: Uint8Array,
): Promise<Uint8Array> {
  const publicKey = await ed.getPublicKey(privateKeyBytes);

  const signatureTmp = await ed.sign(signingMsg, privateKeyBytes);
  const signature = new DiemTypes.Ed25519Signature(signatureTmp);

  const txnAuthenticator = new DiemTypes.TransactionAuthenticatorVariantEd25519(
    new DiemTypes.Ed25519PublicKey(publicKey),
    signature,
  );

  const signedTxn = new DiemTypes.SignedTransaction(rawTxn, txnAuthenticator);
  const signedTxnSerializer = new BcsSerializer();
  signedTxn.serialize(signedTxnSerializer);
  return signedTxnSerializer.getBytes();
}

export function hexToAccountAddress(hex: string): DiemTypes.AccountAddress {
  const senderListTuple: ListTuple<[uint8]> = [];
  for (const entry of hexToBytes(hex)) { // encode as bytes
    senderListTuple.push([entry]);
  }
  return new DiemTypes.AccountAddress(senderListTuple);
}

function normalizeScriptFunctionArgs(args: any[]) {
  return args.map((a) => {
    if (isString(a) && !a.startsWith("0x")) {
      return bufferToHex(textEncoder.encode(a));
    }
    return a;
  });
}

function isString(value: any) {
  return typeof value === "string" || value instanceof String;
}

export function bufferToHex(buffer: any) {
  return [...new Uint8Array(buffer)]
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

function appendBuffer(buffer1: Uint8Array, buffer2: Uint8Array): Uint8Array {
  const tmp = new Uint8Array(buffer1.byteLength + buffer2.byteLength);
  tmp.set(new Uint8Array(buffer1));
  tmp.set(new Uint8Array(buffer2), buffer1.byteLength);
  return tmp;
}

function hexToBytes(hex: string): Uint8Array {
  if (hex.startsWith("0x")) {
    hex = hex.slice(2);
  }
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i !== bytes.length; i++) {
    bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
  }
  return bytes;
}

export function hexToAscii(hex: string) {
  const bytes = hexToBytes(hex);
  return textDecoder.decode(bytes);
}
