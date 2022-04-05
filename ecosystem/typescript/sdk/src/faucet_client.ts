/** Faucet creates and funds accounts. This is a thin wrapper around that. */
import axios from "axios";
import { AptosClient, AptosClientConfig, raiseForStatus } from "./aptos_client";
import { Types } from "./types";
import { HexString, MaybeHexString } from "./hex_string";

export class FaucetClient extends AptosClient {
  faucetUrl: string;

  constructor(nodeUrl: string, faucetUrl: string, config?: AptosClientConfig) {
    super(nodeUrl, config);
    this.faucetUrl = faucetUrl;
  }

  /** This creates an account if it does not exist and mints the specified amount of
   coins into that account */
  async fundAccount(address: MaybeHexString, amount: number): Promise<Types.HexEncodedBytes[]> {
    const url = `${this.faucetUrl}/mint?amount=${amount}&address=${HexString.ensure(address).noPrefix()}`;
    const response = await axios.post<Array<string>>(url, {}, { validateStatus: () => true });
    raiseForStatus(200, response);

    const tnxHashes = response.data;
    const promises = [];
    for (let i = 0; i < tnxHashes.length; i += 1) {
      const tnxHash = tnxHashes[i];
      promises.push(this.waitForTransaction(tnxHash));
    }
    await Promise.all(promises);
    return tnxHashes;
  }
}
