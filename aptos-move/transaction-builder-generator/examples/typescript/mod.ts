// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import * as AptosStdlib from "./aptosStdlib/mod.ts";
import * as AptosTypes from "./aptosTypes/mod.ts";
import { BcsSerializer } from "./bcs/mod.ts";
import { ListTuple } from "./serde/mod.ts";
import { equal } from "https://deno.land/x/equal/mod.ts";

demoPeerToPeerScript();
demoPeerToPeerScriptFunction();

function demoPeerToPeerScript() {
  const address = hexToAccountAddress("0x00000000000000000000000000000001");
  const token = new AptosTypes.TypeTagVariantStruct(
    new AptosTypes.StructTag(
      address,
      new AptosTypes.Identifier("XDX"),
      new AptosTypes.Identifier("XDX"),
      [],
    ),
  );

  const payee = hexToAccountAddress("0x22222222222222222222222222222222");
  const amount = BigInt(1_234_567);
  const script = AptosStdlib.Stdlib.encodePeerToPeerWithMetadataScript(
    token,
    payee,
    amount,
    new Uint8Array(),
    new Uint8Array(),
  );

  const scriptCall = AptosStdlib.Stdlib.decodePeerToPeerWithMetadataScript(
    script,
  );
  if (scriptCall.amount != amount || scriptCall.payee != payee) {
    throw ("wrong script content");
  }

  const bcsSerializer = new BcsSerializer();
  script.serialize(bcsSerializer);

  // add a trailing space so output has parity w legacy implementations:
  // https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/transaction-builder-generator/examples/rust/tx_script_demo.rs#L45
  console.log(bcsSerializer.getBytes().join(" ") + " ");
}

function demoPeerToPeerScriptFunction() {
  const address = hexToAccountAddress("0x00000000000000000000000000000001");
  const token = new AptosTypes.TypeTagVariantStruct(
    new AptosTypes.StructTag(
      address,
      new AptosTypes.Identifier("XDX"),
      new AptosTypes.Identifier("XDX"),
      [],
    ),
  );

  const payee = hexToAccountAddress("0x22222222222222222222222222222222");
  const amount = BigInt(1_234_567);
  const payload = AptosStdlib.Stdlib.encodePeerToPeerWithMetadataScriptFunction(
    token,
    payee,
    amount,
    new Uint8Array(),
    new Uint8Array(),
  );

  const scriptFunCall = AptosStdlib.Stdlib.decodePeerToPeerWithMetadataScriptFunction(
    payload,
  );
  if (scriptFunCall.amount != amount || !equal(scriptFunCall.payee.value, payee.value)) {
    throw ("wrong script content");
  }

  const bcsSerializer = new BcsSerializer();
  payload.serialize(bcsSerializer);

  // add a trailing space so output has parity w legacy implementations:
  // https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/transaction-builder-generator/examples/rust/tx_script_demo.rs#L45
  console.log(bcsSerializer.getBytes().join(" ") + " ");
}

function hexToAccountAddress(hex: string): AptosTypes.AccountAddress {
  if (hex.startsWith("0x")) {
    hex = hex.slice(2);
  }
  const senderListTuple: ListTuple<[number]> = [];
  for (const entry of hexToBytes(hex)) { // encode as bytes
    senderListTuple.push([entry]);
  }
  return new AptosTypes.AccountAddress(senderListTuple);
}

function hexToBytes(hex: string) {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i !== bytes.length; i++) {
    bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
  }
  return bytes;
}
