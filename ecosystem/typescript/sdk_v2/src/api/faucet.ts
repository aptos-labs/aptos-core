import { fundAccount } from "../internal/faucet";
import { HexInput } from "../types";
import { AptosConfig } from "./aptos_config";

/**
 * A class to query all `Faucet` related queries on Aptos.
 */
export class Faucet {
    readonly config: AptosConfig;
  
    constructor(config: AptosConfig) {
      this.config = config;
    }

    /**
     * This creates an account if it does not exist and mints the specified amount of
     * coins into that account
     * 
     * @param address Hex-encoded 16 bytes Aptos account address wich mints tokens
     * @param amount Amount of tokens to mint
     * @param timeoutSecs Timeout in seconds. Defaults to 20 seconds.
     * @returns Hashes of submitted transactions
     */
    async fundAccount(args: { accountAddress: HexInput, amount: number, timeoutSecs?: number }): Promise<Array<string>> {
        const txnStrings = await fundAccount({ aptosConfig: this.config, ...args });
        return txnStrings;
    }
}