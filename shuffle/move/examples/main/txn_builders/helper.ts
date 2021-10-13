// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

// deno-lint-ignore-file no-explicit-any
import * as TxnBuilder from "./mod.ts";
import * as ed from 'https://deno.land/x/ed25519@1.0.1/mod.ts';
import * as util from "https://deno.land/std@0.85.0/node/util.ts";
import * as path from "https://deno.land/std@0.110.0/path/mod.ts";
import { ListTuple, uint8 } from './serde/types.ts';
import { createRemote } from "https://deno.land/x/gentle_rpc@v3.0/mod.ts";
import { hashPrefix } from "./signer.ts";

const shuffleDir = Deno.env.get("SHUFFLE_HOME") || "unknown";
const senderAddressPath = path.join(shuffleDir,"accounts/latest/address");
const senderAddress = await Deno.readTextFile(senderAddressPath);
const textEncoder = new util.TextEncoder();
const privateKeyPath = path.join(shuffleDir,"accounts/latest/dev.key");


// Client side creation and signing of transactions.
// https://github.com/diem/diem/blob/main/json-rpc/docs/method_submit.md#method-submit
export async function setMessage(message: string, sequenceNumber: number) {
  if(sequenceNumber == undefined) {
    console.log("Must pass in parameters: message, sequenceNumber. Try Shuffle.sequenceNumber()");
    return;
  }

  const [rawTxn, signingMsg] = newRawTransactionAndSigningMsg(message, sequenceNumber);
  const signedTxnHex = await newSignedTransaction(rawTxn, signingMsg);

  const remote = createRemote("http://127.0.0.1:8080/v1");
  return await remote.call("submit", [signedTxnHex]);
}

function newRawTransactionAndSigningMsg(
  message: string,
  sequenceNumber: number
):[TxnBuilder.RawTransaction, Uint8Array] {
  // TODO: Remove hardcoded address. Rather than passing in sequenceNumber, and
  // hardcoding address, pass in a Shuffle object that this helper can then use
  // to retrieve what it needs. Or better yet, at initialize, construct a
  // new Helper(Shuffle) that is set at top level that does this for you, so
  // you don't need to pass it in per call.
  const rawTxn = setMessageRawTransaction(
    senderAddress,
    message,
    sequenceNumber
  );

  const bcsSerializer = new TxnBuilder.BcsSerializer();
  rawTxn.serialize(bcsSerializer);
  const rawTxnBytes = bcsSerializer.getBytes();

  const signingMsg = appendBuffer(hashPrefix("RawTransaction"), rawTxnBytes);
  return [rawTxn, signingMsg];
}

async function newSignedTransaction(
  rawTxn: TxnBuilder.RawTransaction,
  signingMsg: Uint8Array
): Promise<string> {
  let privateKeyBytes = await Deno.readFile(privateKeyPath);

  // slice off first BIP type byte, rest of 32 bytes is private key
  privateKeyBytes = privateKeyBytes.slice(1);
  const publicKey = await ed.getPublicKey(privateKeyBytes);

  const signatureTmp = await ed.sign(signingMsg, privateKeyBytes);
  const signature = new TxnBuilder.Ed25519Signature(signatureTmp);

  const txnAuthenticator = new TxnBuilder.TransactionAuthenticatorVariantEd25519(
    new TxnBuilder.Ed25519PublicKey(publicKey),
    signature
  );

  const signedTxn = new TxnBuilder.SignedTransaction(rawTxn, txnAuthenticator);
  const signedTxnSerializer = new TxnBuilder.BcsSerializer();
  signedTxn.serialize(signedTxnSerializer);
  return bufferToHex(signedTxnSerializer.getBytes());
}

export function setMessageTransactionPayload(message: string): TxnBuilder.TransactionPayloadVariantScript {
  const script = TxnBuilder.Stdlib.encodeSetMessageScript(textEncoder.encode(message));
  return new TxnBuilder.TransactionPayloadVariantScript(script);
}

// senderStr example 0x24163afcc6e33b0a9473852e18327fa9
export function setMessageRawTransaction(senderStr: string, message: string, sequenceNumber: number): TxnBuilder.RawTransaction {
  if(senderStr.startsWith("0x")) {
    senderStr = senderStr.slice(2);
  }
  const payload = setMessageTransactionPayload(message);
  const senderListTuple: ListTuple<[uint8]> = [];
  for(const entry of hexToBytes(senderStr)) { // encode as bytes
    senderListTuple.push([entry]);
  }
  return new TxnBuilder.RawTransaction(
    new TxnBuilder.AccountAddress(senderListTuple), // sender
    BigInt(sequenceNumber),
    payload, // txn payload
    BigInt(1000000),  // max gas amount
    BigInt(0), // gas_unit_price
    "XUS", // currency
    BigInt(99999999999),  // expiration_timestamp_secs
    new TxnBuilder.ChainId(4), // chain id, hardcoded to test
  );
}


export function bufferToHex (buffer: any) {
  return [...new Uint8Array (buffer)]
    .map (b => b.toString (16).padStart (2, "0"))
    .join ("");
}

export function hex2a(hexx: string) {
  const hex = hexx.toString(); // normalize
  let str = '';
  for (let i = 0; i < hex.length; i += 2)
    str += String.fromCharCode(parseInt(hex.substr(i, 2), 16));
  return str;
}

export function messagesFrom(resources: any[]) {
  return resources.
    filter(
      entry => entry["type"]["name"] == "MessageHolder"
    );
}

export function decodedMessages(resources: any[]) {
  return messagesFrom(resources).
    map(entry => hex2a(entry.value.message));
}

function appendBuffer (buffer1: Uint8Array, buffer2: Uint8Array): Uint8Array {
  const tmp = new Uint8Array(buffer1.byteLength + buffer2.byteLength);
  tmp.set(new Uint8Array(buffer1));
  tmp.set(new Uint8Array(buffer2), buffer1.byteLength);
  return tmp;
}

function hexToBytes(hex: string) {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i !== bytes.length; i++) {
    bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
  }
  return bytes;
}
