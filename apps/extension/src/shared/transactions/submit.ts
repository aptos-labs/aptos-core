// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { RawTransaction } from 'aptos/dist/transaction_builder/aptos_types';
import { AptosAccount, AptosClient, WaitForTransactionError } from 'aptos';
import { ApiError, Transaction, UserTransaction } from 'aptos/dist/generated';
import { sleep } from 'aptos/dist/util';

import { handleApiError, throwForVmError } from './utils';

/**
 * Copy of AptosClient.waitForTransactionWithResult with local fix, will remove once
 * the fix is applied to the typescript SDK
 */
async function waitForTransactionWithResult(client: AptosClient, txnHash: string) {
  const timeoutSecs = 10;

  let isPending = true;
  let count = 0;
  let lastTxn: Transaction | undefined;
  while (isPending) {
    if (count >= timeoutSecs) {
      break;
    }
    try {
      // eslint-disable-next-line no-await-in-loop
      lastTxn = await client.getTransactionByHash(txnHash);
      isPending = lastTxn.type === 'pending_transaction';
      if (!isPending) {
        break;
      }
    } catch (e) {
      const isApiError = e instanceof ApiError;
      const isRequestError = isApiError
        && e.status !== 404
        && e.status >= 400
        && e.status < 500;
      if (!isApiError || isRequestError) {
        throw e;
      }
    }
    // eslint-disable-next-line no-await-in-loop
    await sleep(1000);
    count += 1;
  }

  if (isPending) {
    throw new WaitForTransactionError(
      `Waiting for transaction ${txnHash} timed out after ${timeoutSecs} seconds`,
      lastTxn,
    );
  }

  return lastTxn;
}

export async function simulateTransaction(
  aptosAccount: AptosAccount,
  aptosClient: AptosClient,
  rawTxn: RawTransaction,
) {
  const simulatedTxn = AptosClient.generateBCSSimulation(aptosAccount, rawTxn);
  try {
    const userTxn = (await aptosClient.submitBCSSimulation(simulatedTxn))[0];
    throwForVmError(userTxn);
    return userTxn;
  } catch (err) {
    handleApiError(err);
    throw err;
  }
}

export async function submitTransaction(
  aptosAccount: AptosAccount,
  aptosClient: AptosClient,
  rawTxn: RawTransaction,
) {
  const signedTxn = AptosClient.generateBCSTransaction(aptosAccount, rawTxn);
  try {
    const { hash } = await aptosClient.submitSignedBCSTransaction(signedTxn);
    return await waitForTransactionWithResult(aptosClient, hash) as UserTransaction;
  } catch (err) {
    handleApiError(err);
    throw err;
  }
}
