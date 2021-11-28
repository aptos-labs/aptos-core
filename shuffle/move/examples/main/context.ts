// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

import * as path from "https://deno.land/std@0.116.0/path/mod.ts";
import urlcat from "https://deno.land/x/urlcat@v2.0.4/src/index.ts";
import { BcsDeserializer } from "./generated/bcs/mod.ts";
import { isURL } from "https://deno.land/x/is_url@v1.0.1/mod.ts";

class ConsoleContext {
  constructor(
    readonly projectPath: string,
    readonly networkName: string,
    readonly networksPath: string,
    readonly nodeUrl: string,
  ) {}

  static fromEnv(): ConsoleContext {
    return new ConsoleContext(
      String(Deno.env.get("PROJECT_PATH")),
      String(Deno.env.get("SHUFFLE_NETWORK_NAME")),
      String(Deno.env.get("SHUFFLE_BASE_NETWORKS_PATH")),
      getNetworkEndpoint(
        String(Deno.env.get("SHUFFLE_NETWORK_DEV_API_URL")),
      ),
    );
  }

  // Returns the address file path for the passed username based on
  // conventions from shuffle account creation
  // ie: ~/.shuffle/networks/localhost/accounts/test/address
  accountAddressPath(username: string): string {
    return path.join(
      this.networksPath,
      this.networkName,
      "accounts",
      username,
      "address",
    );
  }

  // Returns the private key file path for the passed username based on
  // conventions from shuffle account creation
  // ie: ~/.shuffle/networks/localhost/accounts/test/dev.key
  accountKeyPath(username: string): string {
    return path.join(
      this.networksPath,
      this.networkName,
      "accounts",
      username,
      "dev.key",
    );
  }
}

export const consoleContext = ConsoleContext.fromEnv();

export class UserContext {
  constructor(
    readonly username: string,
    readonly address: string,
    readonly privateKeyPath: string,
  ) {}

  // Creates a UserContext based on parameters set in ENV vars, usually via
  // shuffle CLI commands `console` and `test`.
  static fromEnv(username: string): UserContext {
    return new UserContext(
      username,
      String(Deno.env.get("SENDER_ADDRESS")),
      String(Deno.env.get("PRIVATE_KEY_PATH")),
    );
  }

  // Creates a UserContext based on an account name saved to disk based on
  // conventions used on the shuffle CLI account command.
  // ie: ~/.shuffle/networks/localhost/accounts/test
  static async fromDisk(username: string): Promise<UserContext> {
    const addressPath = consoleContext.accountAddressPath(username);
    const privateKeyPath = consoleContext.accountKeyPath(username);
    return new UserContext(
      username,
      await Deno.readTextFile(addressPath),
      privateKeyPath,
    );
  }

  async readPrivateKey(): Promise<Uint8Array> {
    return bcsToBytes(
      await Deno.readFile(this.privateKeyPath),
    );
  }
}

export const defaultUserContext = UserContext.fromEnv("default");

export function addressOrDefault(addr: string | undefined): string {
  if (addr) {
    return addr;
  }
  return defaultUserContext.address;
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
  return bcsDeserializer.deserializeBytes();
}

export function relativeUrl(tail: string) {
  return new URL(tail, consoleContext.nodeUrl).href;
}
