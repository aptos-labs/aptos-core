// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/** Faucet creates and funds accounts. This is a thin wrapper around that. */
import { AptosClient } from "./aptos_client";
import { OpenAPIConfig } from "./generated";
import { AxiosHttpRequest } from "./generated/core/AxiosHttpRequest";
import { HexString, MaybeHexString } from "./hex_string";
import { DEFAULT_TXN_TIMEOUT_SEC } from "./utils";

/**
 * Class for requsting tokens from faucet
 */
export class FaucetClient extends AptosClient {
  faucetRequester: AxiosHttpRequest;

  /**
   * Establishes a connection to Aptos node
   * @param nodeUrl A url of the Aptos Node API endpoint
   * @param faucetUrl A faucet url
   * @param config An optional config for inner axios instance
   * Detailed config description: {@link https://github.com/axios/axios#request-config}
   */
  constructor(nodeUrl: string, faucetUrl: string, config?: Partial<OpenAPIConfig>) {
    super(nodeUrl, config);

    if (!faucetUrl) {
      throw new Error("Faucet URL cannot be empty.");
    }
    // Build a requester configured to talk to the faucet.
    this.faucetRequester = new AxiosHttpRequest({
      BASE: faucetUrl,
      VERSION: config?.VERSION ?? "0.1.0",
      WITH_CREDENTIALS: config?.WITH_CREDENTIALS ?? false,
      CREDENTIALS: config?.CREDENTIALS ?? "include",
      TOKEN: config?.TOKEN,
      USERNAME: config?.USERNAME,
      PASSWORD: config?.PASSWORD,
      HEADERS: config?.HEADERS,
      ENCODE_PATH: config?.ENCODE_PATH,
    });
  }

  /**
   * This creates an account if it does not exist and mints the specified amount of
   * coins into that account
   * @param address Hex-encoded 16 bytes Aptos account address wich mints tokens
   * @param amount Amount of tokens to mint
   * @param timeoutSecs
   * @returns Hashes of submitted transactions
   */
  async fundAccount(address: MaybeHexString, amount: number, timeoutSecs = DEFAULT_TXN_TIMEOUT_SEC): Promise<string[]> {
    const tnxHashes = await this.faucetRequester.request<Array<string>>({
      method: "POST",
      url: "/mint",
      query: {
        address: HexString.ensure(address).noPrefix(),
        amount,
      },
    });

    const promises: Promise<void>[] = [];
    for (let i = 0; i < tnxHashes.length; i += 1) {
      const tnxHash = tnxHashes[i];
      promises.push(this.waitForTransaction(tnxHash, { timeoutSecs }));
    }
    await Promise.all(promises);
    return tnxHashes;
  }
}
