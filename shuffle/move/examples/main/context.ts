// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

import * as _path from "https://deno.land/std@0.110.0/path/mod.ts";
import urlcat from "https://deno.land/x/urlcat@v2.0.4/src/index.ts";
import { BcsDeserializer } from "./generated/bcs/mod.ts";
import { isURL } from "https://deno.land/x/is_url@v1.0.1/mod.ts";

export const shuffleBaseNetworksPath = String(Deno.env.get("SHUFFLE_BASE_NETWORKS_PATH"));
export const projectPath = String(Deno.env.get("PROJECT_PATH"));
export const networkName = String(Deno.env.get("SHUFFLE_NETWORK_NAME"))
export const nodeUrl = getNetworkEndpoint(
  String(Deno.env.get("SHUFFLE_NETWORK_DEV_API_URL")),
);
export const senderAddress = String(Deno.env.get("SENDER_ADDRESS"));
export const privateKeyPath = String(Deno.env.get("PRIVATE_KEY_PATH"));
const privateKeyBytes = bcsToBytes(
  await Deno.readFile(privateKeyPath)
);

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

function bcsToBytes(bcsBytes: Uint8Array): Uint8Array {
  const bcsDeserializer = new BcsDeserializer(bcsBytes);
  return bcsDeserializer.deserializeBytes()
}

export function relativeUrl(tail: string) {
  return new URL(tail, nodeUrl).href;
}
