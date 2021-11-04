// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

import * as path from "https://deno.land/std@0.110.0/path/mod.ts";
import urlcat from "https://deno.land/x/urlcat@v2.0.4/src/index.ts";
import { green } from "https://deno.land/x/nanocolors@0.1.12/mod.ts";
import { isURL } from "https://deno.land/x/is_url@v1.0.1/mod.ts";

export const shuffleDir = String(Deno.env.get("SHUFFLE_HOME"));
export const projectPath = String(Deno.env.get("PROJECT_PATH"));
export const nodeUrl = getNetworkEndpoint(
  String(Deno.env.get("SHUFFLE_NETWORK")),
);
export const senderAddress = String(Deno.env.get("SENDER_ADDRESS"));
export const privateKeyPath = String(Deno.env.get("PRIVATE_KEY_PATH"));
const privateKeyBytes = normalizePrivateKey(
  await Deno.readFile(privateKeyPath),
);

export const receiverPrivateKeyPath = path.join(
  shuffleDir,
  "accounts/test/dev.key",
);
export const receiverAddressPath = path.join(
  shuffleDir,
  "accounts/test/address",
);
export const receiverAddress = await Deno.readTextFile(receiverAddressPath);

function highlight(content: string) {
  return green(content);
}

console.log(`Loading Project ${highlight(projectPath)}`);
console.log(`Sender Account Address ${highlight(senderAddress)}`);
console.log(
  `"helpers", "devapi", "context", "main", "codegen", "help" top level objects available`,
);
console.log(`Run "help" for more information on top level objects`);
console.log(`Connecting to Node ${highlight(nodeUrl)}`);
console.log();

export function privateKey(): Uint8Array {
  return privateKeyBytes.slice(0);
}

export function addressOrDefault(addr: string | undefined): string {
  if (addr) {
    return addr;
  }
  return senderAddress;
}

function getNetworkEndpoint(inputNetwork: string) {
  if (inputNetwork == "unknown") {
    throw new Error("Invalid network.");
  }
  let network = "";
  if (isURL(inputNetwork)) {
    network = inputNetwork;
  } else {
    network = urlcat("http://", inputNetwork);
  }
  return network;
}

function normalizePrivateKey(privateKeyBytes: Uint8Array): Uint8Array {
  if (privateKeyBytes.length == 33) {
    // slice off first BIP type byte, rest of 32 bytes is private key
    privateKeyBytes = privateKeyBytes.slice(1);
  }
  return privateKeyBytes;
}

export function relativeUrl(tail: string) {
  return new URL(tail, nodeUrl).href;
}
