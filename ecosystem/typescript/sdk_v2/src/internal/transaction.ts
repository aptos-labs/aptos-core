/**
 * This file contains the underlying implementations for exposed API surface in
 * the {@link api/transaction}. By moving the methods out into a separate file,
 * other namespaces and processes can access these methods without depending on the entire
 * transaction namespace and without having a dependency cycle error.
 */

import { AptosConfig } from "../api/aptos_config";
import { AptosApiError, getAptosFullNode, paginateWithCursor } from "../client";
import { AnyNumber, GasEstimation, PaginationArgs, TransactionResponse, isPendingTransaction } from "../types";
import { DEFAULT_TXN_TIMEOUT_SEC } from "../utils/const";
import { sleep } from "../utils/helpers";

export async function getTransactions(args: {
  aptosConfig: AptosConfig;
  options?: PaginationArgs;
}): Promise<TransactionResponse[]> {
  const { aptosConfig, options } = args;
  const data = await paginateWithCursor<{}, TransactionResponse[]>({
    aptosConfig,
    originMethod: "getTransactions",
    path: "transactions",
    params: { start: options?.start, limit: options?.limit },
  });
  return data;
}

export async function getGasPriceEstimation(args: { aptosConfig: AptosConfig }) {
  const { aptosConfig } = args;
  const { data } = await getAptosFullNode<{}, GasEstimation>({
    aptosConfig,
    originMethod: "getGasPriceEstimation",
    path: "estimate_gas_price",
  });
  return data;
}

export async function getTransactionByVersion(args: {
  aptosConfig: AptosConfig;
  txnVersion: AnyNumber;
}): Promise<TransactionResponse> {
  const { aptosConfig, txnVersion } = args;
  const { data } = await getAptosFullNode<{}, TransactionResponse>({
    aptosConfig,
    originMethod: "getTransactionByVersion",
    path: `transactions/by_version/${txnVersion}`,
  });
  return data;
}

export async function getTransactionByHash(args: {
  aptosConfig: AptosConfig;
  txnHash: string;
}): Promise<TransactionResponse> {
  const { aptosConfig, txnHash } = args;
  const { data } = await getAptosFullNode<{}, TransactionResponse>({
    aptosConfig,
    path: `transactions/by_hash/${txnHash}`,
    originMethod: "getTransactionByHash",
  });
  return data;
}

export async function isTransactionPending(args: { aptosConfig: AptosConfig; txnHash: string }): Promise<boolean> {
  const { aptosConfig, txnHash } = args;
  try {
    const response = await getTransactionByHash({ aptosConfig, txnHash });
    return isPendingTransaction(response);
  } catch (e: any) {
    if (e?.status === 404) {
      return true;
    }
    throw e;
  }
}

/**
 * Wait for a transaction to move past pending state.
 *
 * There are 4 possible outcomes:
 * 1. Transaction is processed and successfully committed to the blockchain.
 * 2. Transaction is rejected for some reason, and is therefore not committed
 *    to the blockchain.
 * 3. Transaction is committed but execution failed, meaning no changes were
 *    written to the blockchain state.
 * 4. Transaction is not processed within the specified timeout.
 *
 * In case 1, this function resolves with the transaction response returned
 * by the API.
 *
 * In case 2, the function will throw an ApiError, likely with an HTTP status
 * code indicating some problem with the request (e.g. 400).
 *
 * In case 3, if `checkSuccess` is false (the default), this function returns
 * the transaction response just like in case 1, in which the `success` field
 * will be false. If `checkSuccess` is true, it will instead throw a
 * FailedTransactionError.
 *
 * In case 4, this function throws a WaitForTransactionError.
 *
 * @param txnHash The hash of a transaction previously submitted to the blockchain.
 * @param extraArgs.timeoutSecs Timeout in seconds. Defaults to 20 seconds.
 * @param extraArgs.checkSuccess See above. Defaults to false.
 * @returns See above.
 *
 * @example
 * ```
 * const rawTransaction = await this.generateRawTransaction(sender.address(), payload, extraArgs);
 * const bcsTxn = AptosClient.generateBCSTransaction(sender, rawTransaction);
 * const pendingTransaction = await this.submitSignedBCSTransaction(bcsTxn);
 * const transasction = await this.aptosClient.waitForTransactionWithResult(pendingTransaction.hash);
 * ```
 */
export async function waitForTransaction(args: {
  aptosConfig: AptosConfig;
  txnHash: string;
  extraArgs?: { timeoutSecs?: number; checkSuccess?: boolean };
}): Promise<TransactionResponse> {
  const { aptosConfig, txnHash, extraArgs } = args;
  const timeoutSecs = extraArgs?.timeoutSecs ?? DEFAULT_TXN_TIMEOUT_SEC;
  const checkSuccess = extraArgs?.checkSuccess ?? false;

  let isPending = true;
  let count = 0;
  let lastTxn: TransactionResponse | undefined;

  while (isPending) {
    if (count >= timeoutSecs) {
      break;
    }
    try {
      // eslint-disable-next-line no-await-in-loop
      lastTxn = await getTransactionByHash({ aptosConfig, txnHash });

      isPending = isPendingTransaction(lastTxn);

      if (!isPending) {
        break;
      }
    } catch (e) {
      // In short, this means we will retry if it was an ApiError and the code was 404 or 5xx.
      const isAptosApiError = e instanceof AptosApiError;
      const isRequestError = isAptosApiError && e.status !== 404 && e.status >= 400 && e.status < 500;
      if (!isAptosApiError || isRequestError) {
        throw e;
      }
    }
    // eslint-disable-next-line no-await-in-loop
    await sleep(1000);
    count += 1;
  }

  // There is a chance that lastTxn is still undefined. Let's throw some error here
  if (lastTxn === undefined) {
    throw new Error(`Waiting for transaction ${txnHash} failed`);
  }

  if (isPendingTransaction(lastTxn)) {
    throw new WaitForTransactionError(
      `Waiting for transaction ${txnHash} timed out after ${timeoutSecs} seconds`,
      lastTxn,
    );
  }
  if (!checkSuccess) {
    return lastTxn;
  }
  if (!lastTxn.success) {
    throw new FailedTransactionError(
      `Transaction ${txnHash} failed with an error: ${(lastTxn as any).vm_status}`,
      lastTxn,
    );
  }
  return lastTxn;
}

/**
 * This error is used by `waitForTransaction` when waiting for a
 * transaction times out.
 */
export class WaitForTransactionError extends Error {
  public readonly lastSubmittedTransaction: TransactionResponse | undefined;

  constructor(message: string, lastSubmittedTransaction: TransactionResponse | undefined) {
    super(message);
    this.lastSubmittedTransaction = lastSubmittedTransaction;
  }
}

/**
 * This error is used by `waitForTransaction` if `checkSuccess` is true.
 * See that function for more information.
 */
export class FailedTransactionError extends Error {
  public readonly transaction: TransactionResponse;

  constructor(message: string, transaction: TransactionResponse) {
    super(message);
    this.transaction = transaction;
  }
}
