// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

import * as DiemStdlib from "./diemStdlib/mod.ts";
import * as DiemTypes from "./diemTypes/mod.ts";
import { BcsSerializer } from "./bcs/mod.ts";
import { ListTuple } from "./serde/mod.ts";
import { equal } from "https://deno.land/x/equal/mod.ts";

demoPeerToPeerScript();
demoPeerToPeerScriptFunction();

function demoPeerToPeerScript() {
  const address = hexToAccountAddress("0x00000000000000000000000000000001");
  const token = new DiemTypes.TypeTagVariantStruct(
    new DiemTypes.StructTag(
      address,
      new DiemTypes.Identifier("XDX"),
      new DiemTypes.Identifier("XDX"),
      [],
    ),
  );

  const payee = hexToAccountAddress("0x22222222222222222222222222222222");
  const amount = BigInt(1_234_567);
  const script = DiemStdlib.Stdlib.encodePeerToPeerWithMetadataScript(
    token,
    payee,
    amount,
    new Uint8Array(),
    new Uint8Array(),
  );

  const scriptCall = DiemStdlib.Stdlib.decodePeerToPeerWithMetadataScript(
    script,
  );
  if (scriptCall.amount != amount || scriptCall.payee != payee) {
    throw ("wrong script content");
  }

  const bcsSerializer = new BcsSerializer();
  script.serialize(bcsSerializer);

  // add a trailing space so output has parity w legacy implementations:
  // https://github.com/diem/diem/blob/main/diem-move/transaction-builder-generator/examples/rust/tx_script_demo.rs#L45
  console.log(bcsSerializer.getBytes().join(" ") + " ");
}

function demoPeerToPeerScriptFunction() {
  const address = hexToAccountAddress("0x00000000000000000000000000000001");
  const token = new DiemTypes.TypeTagVariantStruct(
    new DiemTypes.StructTag(
      address,
      new DiemTypes.Identifier("XDX"),
      new DiemTypes.Identifier("XDX"),
      [],
    ),
  );

  const payee = hexToAccountAddress("0x22222222222222222222222222222222");
  const amount = BigInt(1_234_567);
  const payload = DiemStdlib.Stdlib.encodePeerToPeerWithMetadataScriptFunction(
    token,
    payee,
    amount,
    new Uint8Array(),
    new Uint8Array(),
  );

  const scriptFunCall = DiemStdlib.Stdlib.decodePeerToPeerWithMetadataScriptFunction(
    payload,
  );
  if (scriptFunCall.amount != amount || !equal(scriptFunCall.payee.value, payee.value)) {
    throw ("wrong script content");
  }

  const bcsSerializer = new BcsSerializer();
  payload.serialize(bcsSerializer);

  // add a trailing space so output has parity w legacy implementations:
  // https://github.com/diem/diem/blob/main/diem-move/transaction-builder-generator/examples/rust/tx_script_demo.rs#L45
  console.log(bcsSerializer.getBytes().join(" ") + " ");
}

function hexToAccountAddress(hex: string): DiemTypes.AccountAddress {
  if (hex.startsWith("0x")) {
    hex = hex.slice(2);
  }
  const senderListTuple: ListTuple<[number]> = [];
  for (const entry of hexToBytes(hex)) { // encode as bytes
    senderListTuple.push([entry]);
  }
  return new DiemTypes.AccountAddress(senderListTuple);
}

function hexToBytes(hex: string) {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i !== bytes.length; i++) {
    bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
  }
  return bytes;
}
