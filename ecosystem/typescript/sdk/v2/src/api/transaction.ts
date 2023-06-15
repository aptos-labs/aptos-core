import { AptosAccount } from "../account";
import { Aptos, AptosConfig } from "../api";
import { AptosApiError, get, post } from "../client";
import { Ed25519Signature } from "../crypto";
import { TransactionBuilderEd25519, TransactionBuilderRemoteABI } from "../transactions";
import { RawTransaction, Gen, FailedTransactionError, WaitForTransactionError } from "../types";
import { DEFAULT_TXN_TIMEOUT_SEC, sleep } from "../utils";

export class Transaction {
  readonly config: AptosConfig;

  constructor(config: AptosConfig) {
    this.config = config;
  }

  async generate(payload: TransactionPayload): Promise<RawTransaction> {
    const { account, func, typeArgs, args, extraArgs } = payload;
    const aptos = new Aptos(this.config);
    const builder = new TransactionBuilderRemoteABI(aptos, {
      sender: account.address(),
      ...extraArgs,
    });
    const rawTxn = await builder.build(func, typeArgs, args);
    return rawTxn;
  }

  /**
   * Generates a signed transaction that can be submitted to the chain for execution.
   *
   * @param account
   * @param rawTxn
   * @returns
   */
  async sign(account: AptosAccount, rawTxn: RawTransaction) {
    const txnBuilder = new TransactionBuilderEd25519((signingMessage: Uint8Array) => {
      // @ts-ignore
      const sigHexStr = account.signBuffer(signingMessage);
      return new Ed25519Signature(sigHexStr.toUint8Array());
    }, account.pubKey().toUint8Array());

    return txnBuilder.sign(rawTxn);
  }

  async submit(signedTxn: Uint8Array): Promise<string> {
    return await post(this.config, `/transactions`, signedTxn, "submitTransaction", {
      headers: { "Content-Type": "application/x.aptos.signed_transaction+bcs" },
    });
  }

  async generateAndSubmit(payload: TransactionPayload): Promise<string> {
    const { account } = payload;
    const rawTxn = await this.generate(payload);
    const signedTxn = await this.sign(account, rawTxn);
    const hash = await this.submit(signedTxn);
    return hash;
  }

  async estimateGasPrice(): Promise<any> {
    return await get(this.config, "/estimate_gas_price", null, "estimateGasPrice");
  }

  async getTransactionByHash(txnHash: string): Promise<Gen.Transaction> {
    return await get(this.config, `/transactions/by_hash/${txnHash}`, null, "estimateGasPrice");
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
}
