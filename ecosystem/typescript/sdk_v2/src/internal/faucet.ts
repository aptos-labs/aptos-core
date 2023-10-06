/**
 * This file contains the underlying implementations for exposed API surface in
 * the {@link api/faucet}. By moving the methods out into a separate file,
 * other namespaces and processes can access these methods without depending on the entire
 * faucet namespace and without having a dependency cycle error.
 */

import { AptosConfig } from "../api/aptos_config";
import { postAptosFaucet } from "../client";
import { AccountAddress } from "../core";
import { HexInput, TransactionResponse } from "../types";
import { DEFAULT_TXN_TIMEOUT_SEC } from "../utils/const";
import { waitForTransaction } from "./transaction";

export async function fundAccount(args: {
  aptosConfig: AptosConfig;
  accountAddress: HexInput;
  amount: number;
  timeoutSecs?: number;
}): Promise<string[]> {
  const { aptosConfig, accountAddress, amount } = args;
  const timeoutSecs = args.timeoutSecs ?? DEFAULT_TXN_TIMEOUT_SEC;

  // address: HexInput, amount: number, timeoutSecs = DEFAULT_TXN_TIMEOUT_SEC)
  const address = AccountAddress.fromHexInput({ input: accountAddress }).toString();
  const { data } = await postAptosFaucet<any, Array<string>>({
    aptosConfig,
    path: "mint",
    params: {
      accountAddress: address,
      amount,
    },
    originMethod: "fundAccount",
  });

  const promises: Promise<TransactionResponse>[] = [];
  for (let i = 0; i < data.length; i += 1) {
    const txnHash = data[i];
    promises.push(waitForTransaction({ aptosConfig, txnHash, extraArgs: { timeoutSecs } }));
  }
  await Promise.all(promises);
  return data;
}
