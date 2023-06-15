/* eslint-disable @typescript-eslint/return-await */
import { AptosConfig } from "./aptos_config";
import { get, post } from "../client";
import { Gen } from "../types";
import { DEFAULT_TXN_TIMEOUT_SEC, HexString, MaybeHexString, Memoize, parseApiError, sleep } from "../utils";
import { GasEstimation } from "../types/generated";
import {
  AptosEntryFunctionTransactionPayload,
  AptosFeePayerTransactionPayload,
  AptosMultiAgentTransactionPayload,
  AptosMultiSigTransactionPayload,
  AptosScriptTransactionPayload,
  AptosTransactionPayload,
  RawTransaction,
  RawTransactionWithData,
  TransactionOptions,
} from "../transactions/types";
import { AptosApiError } from "../client/types";
import {
  generateEntryFunctionRawTransaction,
  generateMultiAgentRawTransaction,
  generateFeePayerRawTransaction,
  generateScriptRawTransaction,
  generateMultiSigRawTransaction,
} from "../transactions/generate_raw_transaction";

export class Transaction {
  readonly config: AptosConfig;

  constructor(config: AptosConfig) {
    this.config = config;
  }

  // SUBMISSION

  async generate(
    sender: MaybeHexString,
    payload: AptosTransactionPayload,
    options?: TransactionOptions,
  ): Promise<RawTransaction | RawTransactionWithData> {
    switch (payload.type) {
      case "entry_function":
        return await generateEntryFunctionRawTransaction(
          sender,
          payload as AptosEntryFunctionTransactionPayload,
          this.config,
          options,
        );
      case "script":
        return await generateScriptRawTransaction(
          sender,
          payload as AptosScriptTransactionPayload,
          this.config,
          options,
        );
      case "multi_agent":
        return await generateMultiAgentRawTransaction(
          sender,
          payload as AptosMultiAgentTransactionPayload,
          this.config,
          options,
        );
      case "fee_payer":
        return await generateFeePayerRawTransaction(
          sender,
          payload as AptosFeePayerTransactionPayload,
          this.config,
          options,
        );
      case "multi_sig":
        return await generateMultiSigRawTransaction(
          sender,
          payload as AptosMultiSigTransactionPayload,
          this.config,
          options,
        );
      default:
        throw new Error("transaction type is not supported");
    }
  }

  /**
   * This function works the same as `waitForTransactionWithResult` except it
   * doesn't return the transaction in those cases, it returns nothing. For
   * more information, see the documentation for `waitForTransactionWithResult`.
   */
  async waitForTransaction(
    txnHash: string,
    extraArgs?: { timeoutSecs?: number; checkSuccess?: boolean },
  ): Promise<void> {
    await this.waitForTransactionWithResult(txnHash, extraArgs);
  }

  async waitForTransactionWithResult(
    txnHash: string,
    extraArgs?: { timeoutSecs?: number; checkSuccess?: boolean },
  ): Promise<Gen.Transaction> {
    const timeoutSecs = extraArgs?.timeoutSecs ?? DEFAULT_TXN_TIMEOUT_SEC;
    const checkSuccess = extraArgs?.checkSuccess ?? false;

    let isPending = true;
    let count = 0;
    let lastTxn: Gen.Transaction | undefined;

    while (isPending) {
      if (count >= timeoutSecs) {
        break;
      }
      try {
        // eslint-disable-next-line no-await-in-loop
        lastTxn = await this.getTransactionByHash(txnHash);
        isPending = lastTxn.type === "pending_transaction";
        if (!isPending) {
          break;
        }
      } catch (e) {
        // In short, this means we will retry if it was an ApiError and the code was 404 or 5xx.
        const isApiError = e instanceof AptosApiError;
        const isRequestError = isApiError && e.status !== 404 && e.status >= 400 && e.status < 500;
        if (!isApiError || isRequestError) {
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

    if (isPending) {
      throw new WaitForTransactionError(
        `Waiting for transaction ${txnHash} timed out after ${timeoutSecs} seconds`,
        lastTxn,
      );
    }
    if (!checkSuccess) {
      return lastTxn;
    }
    if (!(lastTxn as any)?.success) {
      throw new FailedTransactionError(
        `Transaction ${txnHash} committed to the blockchain but execution failed`,
        lastTxn,
      );
    }

    return lastTxn;
  }

  // QUERIES

  /**
   * This creates an account if it does not exist and mints the specified amount of
   * coins into that account
   * @param address Hex-encoded 16 bytes Aptos account address wich mints tokens
   * @param amount Amount of tokens to mint
   * @param timeoutSecs
   * @returns Hashes of submitted transactions
   */
  @parseApiError
  async fundAccount(address: MaybeHexString, amount: number, timeoutSecs = DEFAULT_TXN_TIMEOUT_SEC): Promise<string[]> {
    const { data } = await post<any, Array<string>>({
      url: this.config.faucet,
      endpoint: "mint",
      body: null,
      params: {
        address: HexString.ensure(address).noPrefix(),
        amount,
      },
      overrides: { ...this.config.clientConfig },
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

  @parseApiError
  @Memoize({
    ttlMs: 5 * 60 * 1000, // cache result for 5min
    tags: ["gas_estimates"],
  })
  async estimateGasPrice(): Promise<GasEstimation> {
    const { data } = await get<{}, GasEstimation>({
      url: this.config.network,
      endpoint: "estimate_gas_price",
      originMethod: "estimateGasPrice",
      overrides: { ...this.config.clientConfig },
    });
    return data;
  }

  /**
   * @param txnHash - Transaction hash should be hex-encoded bytes string with 0x prefix.
   * @returns Transaction from mempool (pending) or on-chain (committed) transaction
   */
  @parseApiError
  async getTransactionByHash(txnHash: string): Promise<Gen.Transaction> {
    const { data } = await get<{}, Gen.Transaction>({
      url: this.config.network,
      endpoint: `transactions/by_hash/${txnHash}`,
      originMethod: "getTransactionByHash",
      overrides: { ...this.config.clientConfig },
    });

    return data;
  }
}

/**
 * This error is used by `waitForTransactionWithResult` when waiting for a
 * transaction times out.
 */
export class WaitForTransactionError extends Error {
  public readonly lastSubmittedTransaction: Gen.Transaction | undefined;

  constructor(message: string, lastSubmittedTransaction: Gen.Transaction | undefined) {
    super(message);
    this.lastSubmittedTransaction = lastSubmittedTransaction;
  }
}

/**
 * This error is used by `waitForTransactionWithResult` if `checkSuccess` is true.
 * See that function for more information.
 */
export class FailedTransactionError extends Error {
  public readonly transaction: Gen.Transaction;

  constructor(message: string, transaction: Gen.Transaction) {
    super(message);
    this.transaction = transaction;
  }
}
