import { AptosConfig } from "./aptos_config";
import { aptosRequest } from "../client";
import { HexString, MaybeHexString } from "../types";
import { DEFAULT_TXN_TIMEOUT_SEC } from "../utils";
import { Aptos } from "./aptos";

export class Faucet {
  readonly config: AptosConfig;

  constructor(config: AptosConfig) {
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
    const tnxHashes = await aptosRequest<any, Array<string>>(
      this.config,
      "/mint",
      "POST",
      null,
      { address: HexString.ensure(address).noPrefix(), amount },
      "fundAccount",
    );

    const aptos = new Aptos(this.config);
    const promises: Promise<void>[] = [];
    for (let i = 0; i < tnxHashes.length; i += 1) {
      const tnxHash = tnxHashes[i];
      promises.push(aptos.transaction.waitForTransaction(tnxHash, { timeoutSecs }));
    }
    await Promise.all(promises);
    return tnxHashes;
  }
}
