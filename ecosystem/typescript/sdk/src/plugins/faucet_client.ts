// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/** Faucet creates and funds accounts. This is a thin wrapper around that. */
import { AptosClient } from "../providers/aptos_client";
import { HexString, MaybeHexString, DEFAULT_TXN_TIMEOUT_SEC } from "../utils";
import { post, ClientConfig } from "../client";

/**
 * Class for requsting tokens from faucet
 */
export class FaucetClient extends AptosClient {
  readonly faucetUrl: string;

  readonly config: ClientConfig | undefined;

  /**
   * Establishes a connection to Aptos node
   * @param nodeUrl A url of the Aptos Node API endpoint
   * @param faucetUrl A faucet url
   * @param config An optional config for inner axios instance
   * Detailed config description: {@link https://github.com/axios/axios#request-config}
   */
  constructor(nodeUrl: string, faucetUrl: string, config?: ClientConfig) {
    super(nodeUrl, config);

    if (!faucetUrl) {
      throw new Error("Faucet URL cannot be empty.");
    }
    this.faucetUrl = faucetUrl;
    this.config = config;
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
    const { data } = await post<any, Array<string>>({
      url: this.faucetUrl,
      endpoint: "mint",
      body: null,
      params: {
        address: HexString.ensure(address).noPrefix(),
        amount,
      },
      overrides: { ...this.config },
      originMethod: "fundAccount",
    });

    const promises: Promise<void>[] = [];
    for (let i = 0; i < data.length; i += 1) {
      const tnxHash = data[i];
      promises.push(this.waitForTransaction(tnxHash, { timeoutSecs }));
    }
    await Promise.all(promises);
    return data;
  }
}
