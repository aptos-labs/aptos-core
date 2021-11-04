// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

import * as DiemTypes from "./generated/diemTypes/mod.ts";
import * as ed from "https://deno.land/x/ed25519@1.0.1/mod.ts";
import * as util from "https://deno.land/std@0.85.0/node/util.ts";
import { BcsSerializer } from "./generated/bcs/mod.ts";
import { bytes, ListTuple, uint8 } from "./generated/serde/types.ts";
import { createHash } from "https://deno.land/std@0.77.0/hash/mod.ts";
import { nodeUrl } from "../repl.ts";

const textEncoder = new util.TextEncoder();

export async function buildAndSubmitTransaction(
  addressStr: string,
  sequenceNumber: number,
  privateKeyBytes: Uint8Array,
  payload: DiemTypes.TransactionPayload,
) {
  if (sequenceNumber == undefined) {
    throw "Must pass in parameter sequenceNumber. Try Shuffle.sequenceNumber()";
  }

  const rawTxn = newRawTransaction(
    addressStr,
    payload,
    sequenceNumber,
  );
  const signingMsg = generateSigningMessage(rawTxn);
  const signedTxnBytes = await newSignedTransaction(
    normalizePrivateKey(privateKeyBytes),
    rawTxn,
    signingMsg,
  );

  const settings = {
    method: "POST",
    body: signedTxnBytes,
    headers: {
      "Content-Type": "application/vnd.bcs+signed_transaction",
    },
  };
  const res = await fetch(relativeUrl("/transactions"), settings);
  return await res.json();
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
  addressStr: string,
  sequenceNumber: number,
  privateKeyBytes: Uint8Array,
  scriptFunction: string,
  typeArguments: string[],
  // deno-lint-ignore no-explicit-any
  args: any[],
) {
  privateKeyBytes = normalizePrivateKey(privateKeyBytes);
  const [sfAddress, sfModule, sfFunction] = splitFullyQualifiedName(
    scriptFunction,
  );

  // deno-lint-ignore no-explicit-any
  const request: any = {
    "sender": addressStr,
    "sequence_number": `${sequenceNumber}`,
    "max_gas_amount": "1000000",
    "gas_unit_price": "0",
    "gas_currency_code": "XUS",
    "expiration_timestamp_secs": "99999999999",
    "payload": {
      "type": "script_function_payload",
      "module": {
        "address": sfAddress,
        "name": sfModule,
      },
      "function": sfFunction,
      "type_arguments": typeArguments,
      "arguments": normalizeScriptFunctionArgs(args),
    },
  };

  const resp = await fetch(relativeUrl("/transactions/signing_message"), {
    method: "POST",
    body: JSON.stringify(request),
    headers: {
      "Content-Type": "application/json",
    },
  });

  if (!isSuccess(resp.status)) {
    const errorMsg = (await resp.json()).message;
    throw `unable to create signing message: ${resp.statusText}. ${errorMsg}`;
  }

  const signingMsgPayload = await resp.json();
  const signingMsg = signingMsgPayload.message.slice(2); // remove 0x prefix

  const publicKey = bufferToHex(await ed.getPublicKey(privateKeyBytes));
  const signature = await ed.sign(signingMsg, privateKeyBytes);
  request.signature = {
    "type": "ed25519_signature",
    "public_key": publicKey,
    "signature": signature,
  };

  // deno-lint-ignore no-explicit-any
  const txnResponse: any = await fetch(relativeUrl("/transactions"), {
    method: "POST",
    body: JSON.stringify(request),
    headers: {
      "Content-Type": "application/json",
    },
  });
  if (!isSuccess(txnResponse.status)) {
    const errorMsg = (await txnResponse.json()).message;
    throw `unable to create signing message: ${txnResponse.statusText}. ${errorMsg}`;
  }
  return await txnResponse.json();
}

function isSuccess(status: number) {
  return status >= 200 && status < 300;
}

function splitFullyQualifiedName(
  scriptFunction: string,
): [string, string, string] {
  const [addr, moduleName, functionName]: string[] = scriptFunction.split("::");
  return [addr, moduleName, functionName];
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

// deno-lint-ignore no-explicit-any
function normalizeScriptFunctionArgs(args: any[]) {
  return args.map((a) => {
    if (isString(a) && !a.startsWith("0x")) {
      return bufferToHex(textEncoder.encode(a));
    }
    return a;
  });
}

// deno-lint-ignore no-explicit-any
function isString(value: any) {
  return typeof value === "string" || value instanceof String;
}

// deno-lint-ignore no-explicit-any
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

export function hexToAscii(hexx: string) {
  const hex = hexx.toString(); // normalize
  let str = "";
  for (let i = 0; i < hex.length; i += 2) {
    str += String.fromCharCode(parseInt(hex.substr(i, 2), 16));
  }
  return str;
}

function normalizePrivateKey(privateKeyBytes: Uint8Array): Uint8Array {
  if (privateKeyBytes.length == 33) {
    // slice off first BIP type byte, rest of 32 bytes is private key
    privateKeyBytes = privateKeyBytes.slice(1);
  }
  return privateKeyBytes;
}

function relativeUrl(tail: string) {
  return new URL(tail, nodeUrl).href;
}
